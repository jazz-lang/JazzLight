#![feature(coerce_unsized)]
#![feature(unsize)]
pub mod atomic_ref;
pub mod builtins;
pub mod gc;
pub mod interp;
pub mod opcode;
pub mod reader;
pub mod value;
pub mod writer;

use mimalloc::MiMalloc;
#[global_allocator]
pub static GLOBAL: MiMalloc = MiMalloc;

pub use atomic_ref::AtomicRefCell as RefCell;
pub use std::sync::{Arc, Weak};

pub type Ref<T> = Arc<RefCell<T>>;
pub type WeakRef<T> = Weak<RefCell<T>>;

#[allow(non_snake_case)]
pub fn Ref<T>(x: T) -> Ref<T> {
    Arc::new(RefCell::new(x))
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
