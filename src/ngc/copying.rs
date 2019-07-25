pub trait Collectable {
    fn child(&self) -> Vec<GCValue<dyn Collectable>> {
        unimplemented!()
    }

    fn size(&self) -> usize;
}

use std::cell::{RefCell,Ref,RefMut};

use super::*;
use bump::*;

pub fn align_usize(value: usize, align: usize) -> usize {
    if align == 0 {
        return value;
    }

    ((value + align - 1) / align) * align
}

use std::marker::Unsize;
use std::ops::CoerceUnsized;

impl<T: Collectable + ?Sized + Unsize<U>, U: Collectable + ?Sized> CoerceUnsized<GCValue<U>> for GCValue<T> {}
struct InGC<T: Collectable + ?Sized> {
    
    fwd: Address,
    ptr: RefCell<T>
    //fwd: Address,
}

unsafe impl<T: Collectable + ?Sized + Send> Send for InGC<T> {}
unsafe impl<T: Collectable + ?Sized + Sync> Sync for InGC<T> {}
unsafe impl<T: Collectable + ?Sized + Send> Send for GCValue<T> {}
unsafe impl<T: Collectable + ?Sized + Sync> Sync for GCValue<T> {}

impl<T: Collectable + ?Sized> InGC<T> {
    fn size(&self) -> usize {
        self.ptr.borrow().size()
    }

    fn shild(&self) -> Vec<GCValue<dyn Collectable>> {
        self.ptr.borrow().child()
    }

    fn copy_to(&self, dest: Address, size: usize) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                self as *const Self as *const u8,
                dest.to_mut_ptr::<u8>(),
                size,
            )
        }
    }
}

pub struct GCValue<T: Collectable + ?Sized> {
    ptr: *mut InGC<T>,
}

impl<T: Collectable + ?Sized> GCValue<T> {
    fn size(&self) -> usize {
        unsafe { ((*self.ptr).ptr).borrow().size() }
    }

    fn fwd(&self) -> Address {
        unsafe { (*self.ptr).fwd }
    }

    fn get_ptr(&self) -> *mut InGC<T> {
        self.ptr
    }

    pub fn borrow(&self) -> Ref<'_,T> {
        unsafe {
            (*self.ptr).ptr.borrow()
        }
    }

    pub fn borrow_mut(&self) -> RefMut<'_,T> {
        unsafe {
            (*self.ptr).ptr.borrow_mut()
        }
    }
}

impl<T: Collectable + ?Sized> Clone for GCValue<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Collectable + ?Sized> Copy for GCValue<T> {}

pub struct CopyGC {
    total: Region,
    separator: Address,

    alloc: BumpAllocator,
    roots: Vec<GCValue<dyn Collectable>>,
    allocated: Vec<GCValue<dyn Collectable>>,
    pub stats: bool
}
extern "C" {
    fn malloc(_: usize) -> *mut u8;
    fn free(_: *mut u8);
    fn memcpy(_: *mut u8,_: *const u8,_: usize);
}

impl CopyGC {
    pub fn new() -> CopyGC {
        let alignment = 2 * 4096;
        let heap_size = align_usize(M * 128, alignment);
        let ptr = super::mmap(heap_size,ProtType::Writable);
        let heap_start = Address::from_ptr(ptr);
        let heap = heap_start.region_start(heap_size);

        let semi_size = heap_size / 2;
        let separator = heap_start.offset(semi_size);

        CopyGC {
            total: heap,
            separator,
            roots: vec![],
            stats: false,
            allocated: vec![],
            alloc: BumpAllocator::new(heap_start, separator),

        }
    }
    
    pub fn total_allocated(&self) -> usize {
        let mut s = 0;
        for allocated in self.allocated.iter() {
            s += allocated.size();
        }
        s
    }

    pub fn from_space(&self) -> Region {
        if self.alloc.limit() == self.separator {
            Region::new(self.total.start, self.separator)
        } else {
            Region::new(self.separator, self.total.end)
        }
    }

    pub fn to_space(&self) -> Region {
        if self.alloc.limit() == self.separator {
            Region::new(self.separator, self.total.end)
        } else {
            Region::new(self.total.start, self.separator)
        }
    }

    pub fn remove_root(&mut self,val: GCValue<dyn Collectable>) {
        for i in 0..self.roots.len() {
            if self.roots[i].ptr == val.ptr {
                self.roots.remove(i);
                break;
            }
        }
    }

    pub fn add_root(&mut self,val: GCValue<dyn Collectable>) 
    {
        let mut contains = false;
        for root in self.roots.iter() {
            contains = root.ptr == val.ptr;
            if contains {
                break;
            }
        }

        if !contains {
            self.roots.push(val);
        }
        
    }

    pub fn collect(&mut self) {
        let start_time = time::PreciseTime::now();
        let to_space = self.to_space();
        let from_space = self.from_space();
        let old_size = self.alloc.top().offset_from(from_space.start);
        let mut top = to_space.start;
        let mut scan = top;
        for i in 0..self.roots.len() {
            let mut root = self.roots[i];
            let root_ptr = root.ptr;
            let ptr = unsafe {
                std::mem::transmute_copy(&root_ptr)
            };
            if from_space.contains(ptr) {
                let ptr2 = unsafe {
                    std::mem::transmute_copy(&root_ptr)
                };
                unsafe {
                    root.ptr = std::mem::transmute_copy(&self
                        .copy(ptr2, &mut top));
                }
                    
            }
        }
        let mut i = 0;
        while scan < top {
            
            unsafe {
                let object: *mut InGC<dyn Collectable> = self.allocated[i].ptr;
                assert!(!object.is_null());
                for child in (*object).ptr.borrow().child().iter() {
                    let child_ptr: *mut InGC<dyn Collectable> = child.get_ptr();
                    if child_ptr.is_null() {
                        panic!();
                    }
                    if from_space.contains(std::mem::transmute_copy(&child_ptr)) {
                        *(child_ptr as *mut *mut InGC<dyn Collectable>) = std::mem::transmute_copy(&self
                            .copy(std::mem::transmute_copy(&child_ptr), &mut top));
                            
                    }
                }
                i = i + 1;
                scan = scan.offset((*object).size());
            }
        }
        self.alloc.reset(top, to_space.end);

        if self.stats {
            let end = time::PreciseTime::now();
            let new_size = top.offset_from(to_space.start);
            let garbage = old_size.wrapping_sub(new_size);
            let garbage_ratio = if old_size == 0 {
                0f64
            } else {
                (garbage as f64 / old_size as f64) * 100f64
            };
            println!(
                "GC: {:.1} ms, {}->{} size, {}/{:.0}% garbage",
                start_time.to(end).num_milliseconds(),
                formatted_size(old_size),
                formatted_size(new_size),
                formatted_size(garbage),
                garbage_ratio,
                
            );

        }
    }

    fn copy(&self, obj: *mut InGC<dyn Collectable>, top: &mut Address) -> Address {
        let obj: *mut InGC<dyn Collectable> = obj;
        assert!(!obj.is_null());
        unsafe {
            if (*obj).fwd.is_non_null() {
                assert!((*obj).fwd.is_non_null());
                return (*obj).fwd;
            }
            
            let addr = *top;
            let size = (*obj).size();
            
            (*obj).copy_to(addr, size);
            *top = top.offset(size);
            assert!(top.is_non_null());

            (*obj).fwd = addr;
            assert!(addr.is_non_null());
            
            addr
        }
    }

    pub fn allocate<T: Collectable + Sized + 'static>(&mut self, val: T) -> GCValue<T> {
        let real_layout = std::alloc::Layout::new::<InGC<T>>();
        let ptr = self.alloc.bump_alloc(real_layout.size());
        
        if ptr.is_non_null() {
            let val_ = GCValue {
                ptr: ptr.to_mut_ptr(),
            };
            
            unsafe {
                ((*val_.ptr).fwd) = Address::null();
                ((*val_.ptr).ptr) = RefCell::new(val);
            }
            /*unsafe {
                std::ptr::copy_nonoverlapping(
                    &val as *const T as *const u8,
                    (*val_.ptr).ptr as *mut u8,
                    layout.size(),
                );
            }*/
            self.allocated.push(val_);
            return val_;
        }
        
        println!("allocation failed");
        self.collect();
        let ptr = self.alloc.bump_alloc(real_layout.size());
        let val_ = GCValue {
            ptr: ptr.to_mut_ptr(),
        };
        unsafe {
             ((*val_.ptr).ptr) = RefCell::new(val);
        }
        self.allocated.push(val_);
        return val_;
    }
}

impl Drop for CopyGC {
    fn drop(&mut self) {
        munmap(self.total.start.to_ptr(), self.total.size());
    }
}

impl Collectable for i64 {
    fn child(&self) -> Vec<GCValue<dyn Collectable>> {
        vec![]
    }

    fn size(&self) -> usize {
        std::mem::size_of::<i64>()
    }
}

macro_rules! collectable_for_simple_types {
    ($($t: tt),*) => {
      $(  impl Collectable for $t {
            fn child(&self) -> Vec<GCValue<dyn Collectable>> {
                vec![]
            }

            fn size(&self) -> usize {
                std::mem::size_of::<$t>()
            }
        }
      )*
    };
}

collectable_for_simple_types! {
    u8,u16,u32,u64,u128,
    i8,i16,i32,i128,
    bool,String
}

impl<T: Collectable> Collectable for Vec<T> {
    fn child(&self) -> Vec<GCValue<dyn Collectable>> {
        let mut child = vec![];
        for x in self.iter() {
            child.extend(x.child().iter().cloned());
        }
        child
    }

    fn size(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}


impl<T: Collectable> Collectable for GCValue<T> {
    fn child(&self) -> Vec<GCValue<dyn Collectable>> {
        self.borrow().child()
    }

    fn size(&self) -> usize 
    {
        self.borrow().size()
    }
}

use std::fmt;

impl<T: fmt::Debug + Collectable> fmt::Debug for GCValue<T> {
    fn fmt(&self,f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{:?}",self.borrow())
    }
}


impl<T: Collectable + Eq> Eq for GCValue<T> {}

impl<T: Collectable + PartialEq> PartialEq for GCValue<T> {
    fn eq(&self,other: &Self) -> bool {
        *self.borrow() == *other.borrow()
    }
}

use std::cmp::{Ordering,PartialOrd,Ord};

impl<T: Collectable + PartialOrd> PartialOrd for GCValue<T> {
    fn partial_cmp(&self,other: &Self) -> Option<Ordering> {
        self.borrow().partial_cmp(&other.borrow())
    }
}

impl<T: Collectable + Ord + Eq> Ord for GCValue<T> {
    fn cmp(&self,other: &Self) -> Ordering {
        self.borrow().cmp(&other.borrow())
    }
}

