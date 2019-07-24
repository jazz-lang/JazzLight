use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::cmp::PartialEq;
use std::marker::Sized;
use std::marker::Unsize;
use std::mem;
use std::mem::{align_of_val, size_of_val};
use std::ops::CoerceUnsized;
use std::ptr;

extern crate alloc;
extern crate core;

use core::alloc::Alloc;
use core::alloc::Layout;

use std::fmt;

const GCVALID: u16 = 0xF123;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
enum GcColor {
    Unbound,
    Grey,
    White,
    Black,
}

#[cfg(feature = "gc_debug")]
pub trait Mark: fmt::Debug {
    fn mark(&self, _: &mut InGcEnv) {}
}

#[cfg(not(feature = "gc_debug"))]
pub trait Mark {
    fn mark(&self, _: &mut InGcEnv) {}
}

impl<T: Mark + ?Sized + Unsize<U>, U: Mark + ?Sized> CoerceUnsized<Gc<U>> for Gc<T> {}

struct InGc<T: Mark + ?Sized> {
    valid: u16,
    color: GcColor,
    content: RefCell<T>,
}

pub struct Gc<T: Mark + ?Sized> {
    ptr: *mut InGc<T>,
}

impl<T: Mark + ?Sized> Copy for Gc<T> {}

impl<T: Mark + ?Sized> Gc<T> {
    fn mark(&self, gc: &mut InGcEnv) {
        // where Mark: Sized {
        unsafe { (*self.ptr).content.borrow().mark(gc) }
    }
    fn color(&self) -> GcColor {
        unsafe { (*self.ptr).color }
    }

    fn set_color(&self, color: GcColor) {
        unsafe {
            (*self.ptr).color = color;
        }
    }
    fn forget(&self) {
        #[cfg(feature = "gc_debug")]
        println!("forgetting {:?}", self);
        unsafe {
            (*self.ptr).valid = 0;
            ptr::drop_in_place(&mut (*self.ptr).content);
            alloc::alloc::Global.dealloc(
                std::ptr::NonNull::new_unchecked(self.ptr as *mut u8),
                Layout::from_size_align_unchecked(
                    size_of_val(&*self.ptr),
                    align_of_val(&*self.ptr),
                ),
            );
        }
    }
}

impl<T: 'static + Mark> Gc<T> {
    fn new(o: T, gc: &GcEnv) -> Gc<T> {
        let white = if gc.inner.borrow().white_is_black {
            GcColor::Black
        } else {
            GcColor::White
        };
        Gc {
            ptr: Box::into_raw(Box::new(InGc {
                valid: GCVALID,
                color: white,
                content: RefCell::new(o),
            })),
        }
    }
    pub fn mark_grey(&self, gc: &mut InGcEnv) {
        gc.mark_grey(*self);
    }
}

impl<T: Mark + ?Sized> Gc<T> {
    pub fn borrow(&self) -> Ref<T> {
        unsafe {
            assert!((*self.ptr).valid == GCVALID);
            (*self.ptr).content.borrow()
        }
    }
    pub fn borrow_mut(&self) -> RefMut<T> {
        unsafe {
            assert!((*self.ptr).valid == GCVALID);
            (*self.ptr).content.borrow_mut()
        }
    }

    pub fn try_borrow(&self) -> Option<Ref<T>> {
        unsafe {
            if (*self.ptr).valid == GCVALID {
                Some((*self.ptr).content.borrow())
            } else {
                None
            }
        }
    }
}

impl<T: Mark + ?Sized> Clone for Gc<T> {
    fn clone(&self) -> Self {
        Gc { ptr: self.ptr }
    }
}

impl<T: Mark + ?Sized + PartialEq> PartialEq for Gc<T> {
    fn eq(&self, obj2: &Gc<T>) -> bool {
        self.borrow().eq(&obj2.borrow())
    }
}
impl<T: Mark + ?Sized + Eq> Eq for Gc<T> {}

use std::cmp::{Ord, Ordering, PartialOrd};

impl<T: Mark + ?Sized + PartialOrd> PartialOrd for Gc<T> {
    fn partial_cmp(&self,other: &Self) -> Option<Ordering> {
        self.borrow().partial_cmp(&other.borrow())
    }
}

impl<T: Mark + ?Sized + Ord> Ord for Gc<T> {
    fn cmp(&self,other: &Self) -> Ordering {
        self.borrow().cmp(&other.borrow())
    }
}

#[cfg(feature = "gc_debug")]
impl<T: Mark + ?Sized> fmt::Debug for Gc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            if (*self.ptr).valid == GCVALID {
                write!(f, "{:?} {:?}", self.color(), self.borrow())
            } else {
                write!(f, "<deallocated object>")
            }
        }
    }
}

#[cfg(not(feature = "gc_debug"))]
impl<T: fmt::Debug + Mark + ?Sized> fmt::Debug for Gc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.borrow())
    }
}

const MAX_WHITES: usize = 100;

pub struct InGcEnv {
    whites: Vec<Gc<dyn Mark>>,
    greys: Vec<Gc<dyn Mark>>,
    blacks: Vec<Gc<dyn Mark>>,
    roots: Vec<Gc<dyn Mark>>,
    white_is_black: bool,
    auto: bool,
}

impl InGcEnv {
    #[cfg(feature = "gc_debug")]
    fn col(&self, c: GcColor) -> &'static str {
        match c {
            GcColor::Unbound => &"unbound",
            GcColor::Grey => &"grey",
            GcColor::White => {
                if self.white_is_black {
                    &"black"
                } else {
                    &"white"
                }
            }
            GcColor::Black => {
                if self.white_is_black {
                    &"white"
                } else {
                    &"black"
                }
            }
        }
    }

    fn movec(&mut self, obj: Gc<dyn Mark>, color: GcColor) {
        if obj.color() != color {
            if obj.color() != GcColor::Unbound {
                self.remove(&obj);
            }
            self.add(obj, color);
        }
    }
    fn remove(&mut self, obj: &Gc<dyn Mark>) {
        let color = obj.color();
        obj.set_color(GcColor::Unbound);
        match color {
            GcColor::Unbound => {
                unreachable!();
            }
            GcColor::Grey => {
                //self.greys.remove(obj.position());
                let i = self.greys.iter().position(|e| e.ptr == obj.ptr).unwrap();
                self.greys.remove(i);
            }
            GcColor::White => {
                if self.white_is_black {
                    let i = self.blacks.iter().position(|e| e.ptr == obj.ptr).unwrap();
                    self.blacks.remove(i);
                } else {
                    let i = self.whites.iter().position(|e| e.ptr == obj.ptr).unwrap();
                    self.whites.remove(i);
                }
            }
            GcColor::Black => {
                if self.white_is_black {
                    let i = self.whites.iter().position(|e| e.ptr == obj.ptr).unwrap();
                    self.whites.remove(i);
                } else {
                    let i = self.blacks.iter().position(|e| e.ptr == obj.ptr).unwrap();
                    self.blacks.remove(i);
                }
            }
        }
    }

    fn add(&mut self, obj: Gc<dyn Mark>, color: GcColor) {
        obj.set_color(color);
        match color {
            GcColor::Unbound => {
                unreachable!();
            }
            GcColor::Grey => {
                self.greys.push(obj);
            }
            GcColor::White => {
                if self.white_is_black {
                    self.blacks.push(obj);
                } else {
                    self.whites.push(obj);
                }
            }
            GcColor::Black => {
                if self.white_is_black {
                    self.whites.push(obj);
                } else {
                    self.blacks.push(obj);
                }
            }
        }
    }

    fn swap_white_and_black(&mut self) {
        if cfg!(feature = "gc_debug") {
            println!("swap white and black");
        }
        let oldblacks = mem::replace(&mut (self.blacks), vec![]);
        let oldwhites = mem::replace(&mut (self.whites), oldblacks);
        mem::replace(&mut (self.blacks), oldwhites);
        self.white_is_black = !self.white_is_black;
    }

    fn mark_1(&mut self, obj: Gc<dyn Mark>) {
        let black = if self.white_is_black {
            GcColor::White
        } else {
            GcColor::Black
        };
        self.movec(obj, black);
        obj.mark(self);
    }

    fn mark_grey(&mut self, obj: Gc<dyn Mark>) {
        match obj.color() {
            GcColor::Unbound => {
                unreachable!();
            }
            GcColor::Grey => {}
            GcColor::White => {
                if self.white_is_black {
                } else {
                    self.movec(obj, GcColor::Grey);
                }
            }
            GcColor::Black => {
                if self.white_is_black {
                    self.movec(obj, GcColor::Grey);
                } else {
                }
            }
        }
    }

    fn auto_mark(&self) -> bool {
        self.auto && self.whites.len() >= MAX_WHITES
    }

    fn auto_sweep(&self) -> bool {
        self.auto && self.whites.len() >= MAX_WHITES
    }
}

pub struct GcEnv {
    inner: RefCell<InGcEnv>,
}

impl GcEnv {
    pub fn new(auto: bool) -> GcEnv {
        GcEnv {
            inner: RefCell::new(InGcEnv {
                whites: vec![],
                greys: vec![],
                blacks: vec![],
                roots: vec![],
                white_is_black: false,
                auto: auto,
            }),
        }
    }

    pub fn add_root(&self, obj: Gc<dyn Mark>) {
        let mut gc = self.inner.borrow_mut();
        let mut contains = false;
        for root in gc.roots.iter() {
            contains = root.ptr == obj.ptr;
        }
        if !contains {
            gc.roots.push(obj);
            gc.movec(obj, GcColor::Grey);
        }
    }

    pub fn rm_root(&self, obj: Gc<dyn Mark>) {
        let mut gc = self.inner.borrow_mut();
        let mut contains = false;
        for root in gc.roots.iter() {
            contains = root.ptr == obj.ptr;
        }
        if contains {
            let i = gc.roots.iter().position(|e| e.ptr == obj.ptr).unwrap();
            gc.blacks.remove(i);
        }
    }

    fn auto_mark_sweep(&self) {
        if self.inner.borrow().auto_mark() {
            self.mark(MAX_WHITES);
        }
        if self.inner.borrow().auto_sweep() {
            self.sweep();
        }
    }

    pub fn new_gc<T: 'static + Mark>(&self, obj: T) -> Gc<T> {
        self.auto_mark_sweep();
        let gobj = Gc::<T>::new(obj, &self);
        let mut gc = self.inner.borrow_mut();
        gc.whites.push(gobj);
        return gobj;
    }

    /// write barrier
    /// o: referring object
    /// robj: referred object
    /// if o is marked and it adds a new reference it is remarked as grey
    pub fn new_ref(&self, o: Gc<dyn Mark>, robj: Gc<dyn Mark>) {
        let mut gc = self.inner.borrow_mut();
        if gc.white_is_black {
            if o.color() == GcColor::White && robj.color() == GcColor::Black {
                gc.movec(o, GcColor::Grey);
            }
        } else {
            if o.color() == GcColor::Black && robj.color() == GcColor::White {
                gc.movec(o, GcColor::Grey);
            }
        }
    }

    pub fn pause(&self, b: bool) {
        self.inner.borrow_mut().auto = !b;
        self.auto_mark_sweep();
    }

    pub fn mark(&self, mut steps: usize) {
        #[cfg(feature = "gc_debug")]
        println!("mark");

        let mut gc = self.inner.borrow_mut();
        while !gc.greys.is_empty() && steps > 0 {
            if let Some(obj) = gc.greys.pop() {
                obj.set_color(GcColor::Unbound);
                gc.mark_1(obj);
                steps = steps - 1;
            }
        }
        #[cfg(feature = "gc_debug")]
        println!("end mark");
    }

    pub fn sweep(&self) {
        let mut gc = self.inner.borrow_mut();
        #[cfg(feature = "gc_debug")]
        {
            println!("sweep");
            println!("{:?}", gc.roots);
            println!("w {:?}", gc.whites);
            println!("b {:?}", gc.blacks);
            println!("g {:?}", gc.greys);
        }
        while let Some(obj) = gc.greys.pop() {
            obj.set_color(GcColor::Unbound);
            gc.mark_1(obj);
        }

        while let Some(ref obj) = gc.whites.pop() {
            obj.set_color(GcColor::Unbound);
            obj.forget();
        }
        gc.swap_white_and_black();

        let mut it = Vec::<Gc<dyn Mark>>::new();
        for obj in gc.roots.iter() {
            #[cfg(feature = "gc_debug")]
            println!("marking root {:?}", obj);

            it.push(*obj);
        }
        for obj in it {
            gc.movec(obj, GcColor::Grey);
        }
    }

    fn finalize(&self) {
        #[cfg(feature = "gc_debug")]
        println!("dropping GcEnv");
        self.sweep();

        let mut gc = self.inner.borrow_mut();

        gc.roots.clear();

        while let Some(ref obj) = gc.whites.pop() {
            obj.forget();
        }
        while let Some(ref obj) = gc.greys.pop() {
            obj.forget();
        }
        while let Some(ref obj) = gc.blacks.pop() {
            obj.forget();
        }
    }
}

impl Drop for GcEnv {
    fn drop(&mut self) {}
}

thread_local!(static _GC : GcEnv = GcEnv::new(true));

pub mod gc {
    use super::{Gc, Mark, _GC};

    pub fn new_gc<T: 'static + Mark>(v: T) -> Gc<T> {
        _GC.with(|gc| gc.new_gc(v))
    }

    pub fn new_ref(o: Gc<dyn Mark>, r: Gc<dyn Mark>) {
        _GC.with(|gc| {
            gc.new_ref(o, r);
        });
    }

    pub fn mark(u: usize) {
        _GC.with(|gc| {
            gc.mark(u);
        });
    }

    pub fn sweep() {
        _GC.with(|gc| {
            gc.sweep();
        });
    }

    pub fn pause(b: bool) {
        _GC.with(|gc| {
            gc.pause(b);
        });
    }

    pub fn add_root(o: Gc<dyn Mark>) {
        _GC.with(|gc| {
            gc.add_root(o);
        });
    }

    pub fn rm_root(o: Gc<dyn Mark>) {
        _GC.with(|gc| {
            gc.rm_root(o);
        });
    }

    pub fn finalize() {
        _GC.with(|gc| {
            gc.finalize();
        });
    }
}

impl<T: Mark> Mark for Vec<Gc<T>> {
    fn mark(&self, gc: &mut InGcEnv) {
        for x in self.iter() {
            x.mark(gc);
        }
    }
}

impl Mark for String {}

use std::hash::Hash;
impl<K: Mark + Hash + Eq, V: Mark> Mark for hashlink::LinkedHashMap<K, V> {
    fn mark(&self, gc: &mut InGcEnv) {
        for (key, val) in self.iter() {
            key.mark(gc);
            val.mark(gc);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug)]
    struct A {
        i: u8,
    }

    impl Mark for A {}

    #[test]
    fn basic_test() {
        // initialize the garbage collector
        // gc is local so it has to be passed where it is needed
        let gc = GcEnv::new(false);

        let a = gc.new_gc(A { i: 1 });

        // b is a copy, the value is not moved
        let b = a;

        // the internal value can be accessed through borrow
        let c = a.borrow().i;

        assert_eq!(1, c);

        // we have also mutable borrow
        a.borrow_mut().i = 2;

        assert_eq!(2, b.borrow().i);

        assert_eq!(a.color(), GcColor::White);
    }

    // multi level struct
    #[derive(Debug)]
    struct B {
        a: Gc<A>, // garbage collected
        i: u8,
    }

    unsafe impl Send for B {}
    unsafe impl Sync for B {}
    unsafe impl Send for A {}
    unsafe impl Sync for A {}
    impl Mark for B {
        // the B struct has to implement the mark function
        // the function has to call mark_grey on the
        // internal references
        fn mark(&self, gc: &mut InGcEnv) {
            self.a.mark_grey(gc);
        }
    }

    #[test]
    fn root_test() {
        let gc = GcEnv::new(false);

        let a = gc.new_gc(A { i: 1 });

        gc.add_root(a);
        gc.sweep();
        gc.finalize();
    }

    #[test]
    fn mark_test() {
        let gc = GcEnv::new(false);

        let a = gc.new_gc(A { i: 1 });
        let b = gc.new_gc(B { i: 1, a: a });

        gc.add_root(b);

        gc.mark(100);

        gc.finalize();
    }

    #[test]
    fn sweep_test() {
        let gc = GcEnv::new(false);

        let a = gc.new_gc(A { i: 1 });
        let b = gc.new_gc(B { i: 1, a: a });

        gc.add_root(b);
        gc.sweep();
        gc.finalize();
    }

    #[test]
    fn release_test() {
        let gc = GcEnv::new(false);

        let a = gc.new_gc(A { i: 1 });

        gc.mark(100);
        gc.sweep();
        gc.finalize();
    }

    #[test]
    fn write_barrier_test() {
        let gc = GcEnv::new(false);

        let a = gc.new_gc(A { i: 1 });
        let b = gc.new_gc(B { i: 1, a: a });
        let c = gc.new_gc(A { i: 1 });

        // gc.add_root(b);
        gc.mark(100);

        // replace referred object
        b.borrow_mut().a = c;
        gc.new_ref(b, c);
        gc.finalize();
    }
}
