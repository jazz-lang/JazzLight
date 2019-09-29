#[macro_use]
extern crate pgc_derive;

pub mod acell;
pub mod builtins;
pub mod bytecode;
pub mod interpreter;
pub mod thread;
pub mod value;
use pgc::*;
use thread::*;

#[derive(GcObject, Debug)]
pub struct Module {
    #[unsafe_ignore_trace]
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
use value::Value;

#[derive(GcObject)]
pub struct GlobalState {
    pub static_variables: HashMap<Value, Value>,
    pub threads: Threads,
}

lazy_static::lazy_static!(
    pub static ref STATE: Mutex<Gc<GlobalState>> = {
        let state = Gc::new(GlobalState {
            static_variables: HashMap::new(),
            threads: Threads::new()
        });
        add_root(state);


        Mutex::new(state)
    };
);

pub fn init_builtins() {
    builtins::function::function_object();
    dbg!("Obj");
    builtins::object::object_proto();
    dbg!("Array");
    builtins::array::array_object();
}

pub fn run_module(module: Gc<Module>) -> Value {
    THREAD.with(|thread| {
        let thread = thread.borrow();
        let thread: &mut JThread = thread.get_mut();
        let pc = thread.pc;
        let env = thread.env.clone();
        let this = thread.this.clone();
        let locals = thread.locals;
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

pub fn spawn_thread<T, F>(f: F) -> std::thread::JoinHandle<T>
where
    F: FnMut() -> T + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(f)
}
