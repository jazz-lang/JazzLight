#![feature(coerce_unsized)]
#![feature(unsize)]
#[macro_use]
pub mod interp;
pub mod atomic_ref;
pub mod builtins;
pub mod gc;

pub mod jit;
pub mod opcode;
pub mod reader;
pub mod value;
pub mod writer;

use mimalloc::MiMalloc;
#[global_allocator]
pub static GLOBAL: MiMalloc = MiMalloc;

pub use std::cell::RefCell;
pub use std::rc::{Rc, Weak};

pub type Ref<T> = std::rc::Rc<RefCell<T>>;
pub type WeakRef<T> = Weak<RefCell<T>>;

pub use std::result::Result;

#[allow(non_snake_case)]
pub fn Ref<T>(x: T) -> Ref<T> {
    Rc::new(RefCell::new(x))
}

use std::collections::HashMap;
use value::Value;

pub struct Module {
    pub exports: Value,
    pub code: Vec<opcode::Op>,
    pub globals: Vec<Value>,
    pub trace_info: HashMap<u32, (usize, String)>,
}

use parking_lot::RwLock;
lazy_static::lazy_static! {
    pub static ref FIELDS: RwLock<HashMap<u64,String>> = RwLock::new(HashMap::new());
}

pub fn get_field(h: u64) -> Option<String> {
    FIELDS.read().get(&h).cloned()
}
