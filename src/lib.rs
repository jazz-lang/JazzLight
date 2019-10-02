pub mod acell;
pub mod builtins;
pub mod bytecode;
pub mod compiler;
pub mod interpreter;
pub mod thread;
pub mod value;

use thread::*;

#[derive(Debug)]
pub struct Module {
    pub code: Vec<bytecode::Op>,
    pub globals: Vec<value::Value>,
    pub exports: value::Value,
}

impl Module {
    pub fn new() -> Self {
        Self {
            code: vec![],
            globals: vec![],
            exports: Value::Null,
        }
    }
}

use std::collections::HashMap;

use parking_lot::Mutex;
use value::*;

pub struct GlobalState {
    pub static_variables: HashMap<Value, Value>,
    pub threads: Threads,
}

lazy_static::lazy_static!(
    pub static ref STATE: Mutex<GlobalState> = {
        let mut state = GlobalState {
            static_variables: HashMap::new(),
            threads: Threads::new()
        };
        let obj = Gc::new("Object".to_owned());
        let object = Gc::new(Object {
                proto: None,
                properties: Gc::new(vec![]),
                kind: ObjectKind::Ordinary
            });
        state.static_variables.insert(Value::String(obj),Value::Object(object));


        Mutex::new(state)
    };
);

/*
unsafe impl GcObject for GlobalState {
    fn references(&self) -> Vec<Gc<dyn GcObject>> {
        let mut v: Vec<Gc<dyn GcObject>> = vec![];
        for (key, val) in self.static_variables.iter() {
            v.extend(key.references());
            v.extend(val.references());
        }
        let _ = self.threads.each(|thread| {
            v.push(**thread);
        });
        v
    }
}
*/
pub fn init_builtins() {
    builtins::function::function_object();
    builtins::object::object_proto();
    builtins::array::array_object();
    builtins::builtin_fns();
    builtins::common::init_common();
}

pub fn run_module(module: Gc<Module>) -> Value {
    THREAD.with(|thread| {
        let thread = thread.borrow();
        let mut thread = thread.get_mut();
        let pc = thread.pc;
        let env = thread.env.clone();
        let this = thread.this.clone();
        let locals = thread.locals.clone();
        thread.locals = Gc::new(HashMap::new());
        thread.pc = 0;
        thread.env = Value::Null;
        thread.this = Value::Null;
        thread.exit_frame();
        let value = thread.run(module);
        thread.pc = pc;
        thread.env = env;
        thread.this = this;
        thread.locals = locals;
        value
    })
}

pub fn spawn_thread<T, F>(mut f: F) -> std::thread::JoinHandle<T>
where
    F: FnMut() -> T + Send + 'static,
    T: Send + 'static,
{
    use std::thread::Builder;
    Builder::new()
        .name(format!(
            "<thread 0x{:x}>",
            STATE.lock().threads.threads.lock().len()
        ))
        .spawn(move || {
            //gc_attach_current_thread();
            {
                let state = STATE.lock();
                state.threads.attach_current_thread();
            }
            let res = f();
            {
                let state = STATE.lock();
                state.threads.detach_current_thread();
            }
            res
        })
        .unwrap()
}

#[inline(always)]
pub fn unreachable() -> ! {
    #[cfg(debug_assertions)]
    {
        unreachable!()
    }
    #[cfg(not(debug_assertions))]
    {
        unsafe { std::hint::unreachable_unchecked() }
    }
}

use std::sync::Arc;

pub struct Gc<T: ?Sized> {
    val: Arc<acell::AtomicRefCell<T>>,
}
impl<T> Gc<T> {
    pub fn new(val: T) -> Self {
        Gc {
            val: Arc::new(acell::AtomicRefCell::new(val)),
        }
    }
}

impl<T: ?Sized> Gc<T> {
    pub fn ref_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.val, &other.val)
    }

    pub fn get(&self) -> acell::AtomicRef<'_, T> {
        self.val.borrow()
    }
    pub fn get_mut(&self) -> acell::AtomicRefMut<'_, T> {
        self.val.borrow_mut()
    }
}

impl<T: ?Sized> Clone for Gc<T> {
    fn clone(&self) -> Self {
        Self {
            val: self.val.clone(),
        }
    }
}

use std::fmt;
impl<T: ?Sized + fmt::Debug> fmt::Debug for Gc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.get())
    }
}

impl<T: fmt::Display> fmt::Display for Gc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self.get())
    }
}

impl<T: PartialEq> PartialEq for Gc<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get().eq(&other.get())
    }
}
impl<T: Eq> Eq for Gc<T> {}

impl<T: PartialOrd> PartialOrd for Gc<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

use std::hash::*;
impl<T: Hash> Hash for Gc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}
