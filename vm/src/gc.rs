use std::cell::*;

use std::marker::Unsize;
use std::ops::CoerceUnsized;

pub trait Trace {
    fn trace(&self) {}
    fn finalize(&mut self) {}
    //fn finalizer(&mut self) {}
}

impl<T: Trace + ?Sized + Unsize<U>, U: Trace + ?Sized> CoerceUnsized<GC<U>> for GC<T> {}
struct InGC<T: Trace + ?Sized> {
    mark: bool,
    ptr: RefCell<T>,
}

pub struct GC<T: Trace + ?Sized> {
    ptr: *mut InGC<T>,
}

impl<T: Trace + ?Sized> Copy for GC<T> {}
impl<T: Trace + ?Sized> Clone for GC<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Trace + ?Sized> GC<T> {
    /// Get shared reference to object
    ///
    /// Function will panic if object already mutable borrowed
    pub fn borrow(&self) -> Ref<'_, T> {
        unsafe { (*self.ptr).ptr.borrow() }
    }

    /// Get mutable reference to object
    ///
    /// Function will panic if object already mutable borrowed
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        unsafe { (*self.ptr).ptr.borrow_mut() }
    }

    pub fn marked(&self) -> bool {
        unsafe { (*self.ptr).mark }
    }

    pub fn mark(&self) {
        unsafe {
            let ptr = &mut *self.ptr;
            ptr.mark = true;
            ptr.ptr.borrow().trace();
        }
    }

    pub fn ref_eq(&self, other: &GC<T>) -> bool {
        self.ptr as *const u8 == other.ptr as *const u8
    }
}

pub struct GarbageCollector {
    allocated: Vec<GC<dyn Trace>>,
    roots: Vec<GC<dyn Trace>>,
    should_collect: bool,
    collecting: bool,
    ratio: usize,
}

pub(crate) struct FormattedSize {
    size: usize,
}

impl fmt::Display for FormattedSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ksize = (self.size as f64) / 1024f64;

        if ksize < 1f64 {
            return write!(f, "{}B", self.size);
        }

        let msize = ksize / 1024f64;

        if msize < 1f64 {
            return write!(f, "{:.1}K", ksize);
        }

        let gsize = msize / 1024f64;

        if gsize < 1f64 {
            write!(f, "{:.1}M", msize)
        } else {
            write!(f, "{:.1}G", gsize)
        }
    }
}

pub(crate) fn formatted_size(size: usize) -> FormattedSize {
    FormattedSize { size }
}

impl GarbageCollector {
    pub fn new() -> GarbageCollector {
        GarbageCollector {
            allocated: vec![],
            roots: vec![],
            should_collect: false,
            collecting: false,
            ratio: 0,
        }
    }
    pub fn allocated(&self) -> usize {
        self.allocated.len()
    }

    pub fn alloc<T: Trace + Sized + 'static>(&mut self, val: T) -> GC<T> {
        let layout = std::alloc::Layout::new::<InGC<T>>();
        let mem = unsafe { std::alloc::alloc(layout) } as *mut InGC<T>;
        self.ratio += layout.size();
        unsafe {
            mem.write(InGC {
                ptr: RefCell::new(val),
                mark: false,
            });
        }

        let gc = GC { ptr: mem };
        self.allocated.push(gc);
        gc
    }

    pub fn add_root(&mut self, object: GC<dyn Trace>) {
        if self.roots.iter().find(|x| x.ref_eq(&object)).is_some() {
            return;
        }
        self.roots.push(object);
    }

    pub fn remove_root(&mut self, object: GC<dyn Trace>) {
        for i in 0..self.roots.len() {
            if self.roots[i].ref_eq(&object) {
                self.roots.remove(i);
                return;
            }
        }
    }

    pub fn collect(&mut self, verbose: bool) {
        let mut size_before = None;
        if verbose
            || std::env::var("GC_PRINT_STATS")
                .map(|x| x == "1" || x == "true")
                .unwrap_or(false)
        {
            let mut sum = 0;
            for alloc in self.allocated.iter() {
                sum += unsafe { std::alloc::Layout::for_value(&*alloc.ptr).size() };
            }
            size_before = Some(sum);
        }
        let start = time::PreciseTime::now();
        self.mark();
        self.sweep();
        let end = time::PreciseTime::now();
        if verbose
            || std::env::var("GC_PRINT_STATS")
                .map(|x| x == "1" || x == "true")
                .unwrap_or(false)
        {
            let finish = start.to(end);
            let mut sum = 0;
            for alloc in self.allocated.iter() {
                sum += unsafe { std::alloc::Layout::for_value(&*alloc.ptr).size() };
            }
            let garbage = size_before.unwrap().wrapping_sub(sum);
            let ratio = if size_before.unwrap() == 0 {
                0f64
            } else {
                (garbage as f64 / size_before.unwrap() as f64) * 100f64
            };
            println!(
                "GC: Collection finished in {} ms({}ns).",
                finish.num_milliseconds(),
                finish.num_nanoseconds().unwrap(),
            );
            println!(
                "GC: {}->{} size, {}/{:.0}% garbage",
                formatted_size(size_before.unwrap()),
                formatted_size(sum),
                formatted_size(garbage),
                ratio,
            );
        }
    }

    fn mark(&mut self) {
        for i in 0..self.roots.len() {
            let root = self.roots[i];
            root.mark();
        }
    }

    fn sweep(&mut self) {
        let mut new_heap = vec![];
        for object in self.allocated.iter() {
            unsafe {
                if (*object.ptr).mark {
                    (*object.ptr).mark = false;
                    new_heap.push(object.clone());
                } else {
                    std::alloc::dealloc(
                        object.ptr as *mut u8,
                        std::alloc::Layout::for_value(&*object.ptr),
                    )
                }
            }
        }
        self.allocated = new_heap;
    }
}

impl Drop for GarbageCollector {
    fn drop(&mut self) {
        self.allocated.retain(|x| {
            unsafe {
                std::alloc::dealloc(x.ptr as *mut _, std::alloc::Layout::for_value(&*x.ptr));
            }
            false
        });
    }
}

macro_rules! collectable_for_simple_types {
    ($($t: tt),*) => {
      $(  impl Trace for $t {
            fn trace(&self) {}
        }
      )*
    };
}

collectable_for_simple_types! {
    u8,u16,u32,u64,u128,
    i8,i16,i32,i128,i64,
    bool,String
}

use std::collections::HashMap;
impl<K: Trace, V: Trace> Trace for HashMap<K, V> {
    fn trace(&self) {
        for (x, y) in self.iter() {
            x.trace();
            y.trace();
        }
    }
}

impl<T: Trace> Trace for Vec<T> {
    fn trace(&self) {
        self.iter().for_each(|x| x.trace());
    }
}

impl<T: Trace> Trace for GC<T> {
    fn trace(&self) {
        self.mark();
    }
}

impl Trace for std::fs::File {
    fn finalize(&mut self) {}
}

use std::fmt;

impl<T: fmt::Debug + Trace> fmt::Debug for GC<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.borrow())
    }
}

impl<T: Trace + Eq> Eq for GC<T> {}

impl<T: Trace + PartialEq> PartialEq for GC<T> {
    fn eq(&self, other: &Self) -> bool {
        *self.borrow() == *other.borrow()
    }
}

use std::cmp::{Ord, Ordering, PartialOrd};

impl<T: Trace + PartialOrd> PartialOrd for GC<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.borrow().partial_cmp(&other.borrow())
    }
}

impl<T: Trace + Ord + Eq> Ord for GC<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.borrow().cmp(&other.borrow())
    }
}

use std::hash::{Hash, Hasher};
impl<T: Hash + Trace> Hash for GC<T> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.borrow().hash(h);
    }
}

unsafe impl Send for GarbageCollector {}
unsafe impl Sync for GarbageCollector {}

use parking_lot::RwLock;

/*lazy_static::lazy_static! {
    pub static ref COLLECTOR: RwLock<GarbageCollector> = RwLock::new(GarbageCollector::new());
}*/

thread_local! {
    static COLLECTOR: RefCell<GarbageCollector> = RefCell::new(GarbageCollector::new());
}

/// Clear roots,should be invoked at end of program when you need cleanup memory.
pub fn gc_clear_roots() {
    COLLECTOR.with(|gc| gc.borrow_mut().roots.clear());
}
/// Force collection.
pub fn gc_force_collect(verbose: bool) {
    unsafe {
        COLLECTOR.with(|gc| {
            gc.borrow_mut().collect(verbose);
        })
    }
}

pub fn gc_alloc<T: Trace + 'static>(x: T) -> GC<T> {
    COLLECTOR.with(|gc| gc.borrow_mut().alloc(x))
}
pub fn gc_collect() {
    gc_force_collect(false);
}

pub fn gc_add_root(x: GC<dyn Trace>) {
    COLLECTOR.with(|gc| gc.borrow_mut().add_root(x));
}

pub fn gc_remove_root(x: GC<dyn Trace>) {
    COLLECTOR.with(|gc| gc.borrow_mut().remove_root(x));
}

pub fn gc_total_allocated() -> usize {
    0
}
pub fn gc_allocated_count() -> usize {
    COLLECTOR.with(|gc| gc.borrow().allocated.len())
}
