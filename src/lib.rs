#[macro_use]
extern crate pgc_derive;

pub mod acell;
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
