use std::cell::Cell;
use std::fmt;
use std::mem;
use std::ops::Deref;
use std::rc::{Rc, Weak};

pub trait Trace {
    /// Trace all contained `Handle`s to other GC objects by calling
    /// `tracer.trace_handle`.
    fn trace(&self, _: &mut Tracer) {}
}

pub struct Tracer {
    traced: bool,
    worklist: Vec<Handle<dyn Trace + 'static>>,
}

use std::marker::Unsize;
use std::ops::CoerceUnsized;
pub struct Handle<T: Trace + ?Sized> {
    inner: Weak<GcData<T>>,
}

impl<T: Trace + ?Sized + Unsize<U> + CoerceUnsized<U>, U: Trace + ?Sized> CoerceUnsized<GcData<U>>
    for GcData<T>
{
}
impl<T: Trace + ?Sized + Unsize<U> + CoerceUnsized<U>, U: Trace + ?Sized> CoerceUnsized<Handle<U>>
    for Handle<T>
{
}

impl<T: Trace + ?Sized + Unsize<U> + CoerceUnsized<U>, U: Trace + ?Sized> CoerceUnsized<Rooted<U>>
    for Rooted<T>
{
}

/// GC metadata maintained for every object on the heap.
struct Metadata {
    /// The "color" bit, indicating whether we've already traced this object
    /// during this collection, used to prevent unnecessary work.
    traced: Cell<bool>,
}
struct GcData<T: Trace + ?Sized> {
    metadata: Metadata,
    object: T,
}

impl Tracer {
    /// Enqueue the object behind `handle` for marking and tracing.
    pub fn trace_handle(&mut self, handle: Handle<dyn Trace>) {
        let traced = handle.with(|gc| gc.traced() == self.traced);

        if !traced {
            self.worklist.push(handle.clone());
        }
    }

    /// Starting with the root set in `self.worklist`, marks all transitively
    /// reachable objects by setting their `traced` metadata field to
    /// `self.traced`.
    fn mark_all(&mut self) {
        let mut worklist = mem::replace(&mut self.worklist, Vec::new());

        while !worklist.is_empty() {
            for handle in worklist.drain(..) {
                handle.with(|gc| {
                    if gc.traced() != self.traced {
                        // Hasn't been traced yet
                        gc.metadata.traced.set(self.traced);
                        gc.object.trace(self);
                    }
                });
            }

            mem::swap(&mut self.worklist, &mut worklist);
        }
    }
}

impl<T: Trace + ?Sized> Handle<T> {
    fn from_weak(weak: Weak<GcData<T>>) -> Self {
        Self { inner: weak }
    }

    fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(Rc<GcData<T>>) -> R,
    {
        f(self.inner.upgrade().expect("use after free"))
    }
}

impl<T: Trace + ?Sized> GcData<T> {
    /// Gets the value of the `traced` metadata field.
    ///
    /// The meaning of this value changes every collection and depends on GC
    /// state.
    fn traced(&self) -> bool {
        self.metadata.traced.get()
    }
}

impl Trace for Handle<dyn Trace> {
    fn trace(&self, tracer: &mut Tracer) {
        tracer.trace_handle(self.clone());
    }
}

impl<T: Trace + ?Sized> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Handle {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Trace> fmt::Pointer for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.with(|rc| write!(f, "{:p}", rc))
    }
}

pub struct Rooted<T: Trace + ?Sized> {
    inner: Rc<GcData<T>>,
}

impl<T: Trace> Rooted<T> {
    /// Creates a new garbage-collected unrooted handle to the object.
    ///
    /// As long as `self` still exists, the handle will not be invalidated.
    pub fn new_handle(&self) -> Handle<T> {
        Handle::from_weak(Rc::downgrade(&self.inner))
    }
}

impl<T: Trace> Deref for Rooted<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner.object
    }
}

/// A garbage collector managing objects of type `T`.
///
/// When dropped, all unrooted objects will be destroyed. Any rooted objects
/// should no longer be used.
pub struct Gc {
    /// All allocated objects owned by this GC.
    ///
    /// This is basically a large set of `Rooted` that will be culled during
    /// collection.
    objs: Vec<Rc<GcData<dyn Trace>>>,
    traced_color: bool,
    /// Total number of objects allocated after which we do the next collection.
    next_gc: usize,
}

impl Gc {
    pub fn new() -> Self {
        Self {
            objs: Vec::new(),
            traced_color: true,
            next_gc: 32,
        }
    }

    pub fn allocate<T: 'static + Trace>(&mut self, t: T) -> Rooted<T> {
        let root = self.allocate_nocollect(t);

        if self.estimate_heap_size() >= self.next_gc {
            self.do_collect();

            // Do the next collection after the *remaining* heap has doubled
            self.next_gc = self.estimate_heap_size() * 2;
        }

        root
    }

    /// Allocate `t` on the garbage-collected heap without triggering a
    /// collection.
    pub fn allocate_nocollect<T: Trace + Sized + 'static>(&mut self, t: T) -> Rooted<T> {
        let rc = Rc::new(GcData {
            metadata: Metadata {
                traced: Cell::new(!self.traced_color), // initially not traced
            },
            object: t,
        });
        self.objs.push(rc.clone());

        Rooted { inner: rc }
    }

    /// Collect each and every garbage object, atomically.
    ///
    /// The root set is determined by taking all objects whose strong reference
    /// count `>1`. This only happens during active access (which implies that
    /// the object is still reachable) and because there's a `Rooted` instance
    /// pointing to the object.
    ///
    /// As an optimization, whenever we come across an object with a weak count
    /// of 0, we know that it has no traced reference pointing to it. If that
    /// object also has a strong count of 1, that object isn't rooted (the only
    /// strong reference coming from the GC itself) and can be freed immediately
    /// without having to finish the mark phase. Note that this might in turn
    /// drop the weak count to other objects to 0 and make them collectible.
    pub fn force_full_collect(&mut self) {
        let _size_before_collect = self.estimate_heap_size();

        // Keep all objects that are rooted or have references pointing to them
        // TODO split this into 2 generations (and maybe an additional root list?)
        for _ in 1.. {
            let before = self.objs.len();
            self.objs
                .retain(|obj| Rc::strong_count(obj) > 1 || Rc::weak_count(obj) > 0);

            // Run until fixpoint
            if self.objs.len() == before {
                break;
            }
        }

        // Do cycle collection via mark-and-sweep GC

        // Determine root set
        let worklist = self
            .roots()
            .map(Rc::downgrade)
            .map(Handle::from_weak)
            .collect::<Vec<_>>();

        // Mark
        let mut tracer = Tracer {
            traced: self.traced_color,
            worklist,
        };
        tracer.mark_all();

        // Sweep. Retain only the objects marked with the current color
        let traced_color = self.traced_color;
        self.objs.retain(|obj| obj.traced() == traced_color);

        // Flip colors
        self.traced_color = !self.traced_color;
    }

    fn do_collect(&mut self) {
        // TODO employ smart heuristics for how much to collect
        self.force_full_collect();
    }

    fn estimate_heap_size(&self) -> usize {
        self.objs.len()
    }

    /// Returns an iterator over all rooted objects.
    ///
    /// An object is rooted when it has a strong count of at least 2.
    fn roots(&self) -> impl Iterator<Item = &Rc<GcData<dyn Trace>>> {
        self.objs.iter().filter(|rc| Rc::strong_count(rc) > 1)
    }
}

impl<T: Trace> Trace for Vec<T> {
    fn trace(&self, tracer: &mut Tracer) {
        for item in self.iter() {
            item.trace(tracer);
        }
    }
}

macro_rules! trace_for_simple {
    ($($t: ty),*) => {
        $(
        impl Trace for $t {
            fn trace(&self,_: &mut Tracer) {}
        }
        )*
    };
}

trace_for_simple!(u8, u16, u32, u64, bool, i8, i16, i32, i64, i128, u128, f32, f64, String);

impl<K: Trace, V: Trace> Trace for std::collections::HashMap<K, V> {
    fn trace(&self, tracer: &mut Tracer) {
        for (key, val) in self.iter() {
            key.trace(tracer);
            val.trace(tracer);
        }
    }
}

impl<K: Trace, V: Trace> Trace for hashlink::LinkedHashMap<K, V> {
    fn trace(&self, tracer: &mut Tracer) {
        for (key, val) in self.iter() {
            key.trace(tracer);
            val.trace(tracer);
        }
    }
}

use std::cell::RefCell;

thread_local! {
    static COLLECTOR: RefCell<Gc> = RefCell::new(Gc::new());
}

pub fn gc_alloc<X: Trace + 'static>(x: X) -> Rooted<X> {
    COLLECTOR.with(|gc: &RefCell<Gc>| gc.borrow_mut().allocate(x))
}

pub fn gc_collect() {
    COLLECTOR.with(|gc: &RefCell<Gc>| {
        gc.borrow_mut().force_full_collect();
    })
}

impl<T: Trace> Trace for RefCell<T> {
    fn trace(&self, t: &mut Tracer) {
        self.borrow().trace(t);
    }
}
