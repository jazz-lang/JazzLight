#![feature(allocator_api)]

use std::sync::Arc;
pub type P<T> = Arc<Cell<T>>;

#[allow(non_snake_case)]
pub fn P<T>(value: T) -> P<T> {
    P::new(Cell::new(value))
}

pub static mut VERBOSE: bool = false;
pub static mut PRINT_EXECUTION_PROCESS: bool = false;

pub mod builtins;
pub mod fields;
pub mod hash;
pub mod jit;
pub mod module;

pub mod opcode;
pub mod value;
#[macro_use]
pub mod vm;

pub struct Cell<T> {
    val: *mut T,
}

use vm::*;
use module::*;
use builtins::*;
use fields::*;

pub fn initialize(args: Vec<String>) {
    

    let mut vm = VM::new();
    init_fields();
    register_builtins(&mut vm);
    
    use std::io::Read;
    
    let mut f = std::fs::File::open(&args[1]).unwrap();
    let mut buf = vec![];
    f.read_to_end(&mut buf).unwrap();
    let reader = Reader {  
        code: buf,
        pc: 0,
    };  
    use std::path::Path;
    let p = Path::new(&args[1]);
    let f = p.file_stem().unwrap().to_str().unwrap().to_owned();

    let mut module = read_module(reader, &f);
    
    vm.code = module.code.clone();
    *VM_THREAD.borrow_mut() = vm;
    VM_THREAD.borrow_mut().interp(&mut module);

}


unsafe impl<T: Sync> Sync for Cell<T> {}
unsafe impl<T: Send> Send for Cell<T> {}

impl<T> Cell<T> {
    pub fn new(val: T) -> Cell<T> {
        let boxed = Box::new(val);

        Cell {
            val: Box::into_raw(boxed) as *mut T,
        }
    }
    #[inline]
    pub fn borrow_mut(&self) -> &mut T {
        unsafe {
            let ptr = self.val as *const T as *mut T;
            &mut *ptr
        }
    }
    #[inline]
    pub fn borrow(&self) -> &T {
        unsafe {
            let ptr = self.val as *const T as *mut T;
            &*ptr
        }
    }

    pub fn direct(&self) -> Box<T> {
        unsafe { Box::from_raw(self.val) }
    }

    #[inline]
    pub fn raw(&self) -> *mut T {
        self.val
    }
}

impl<T> Clone for Cell<T> {
    fn clone(&self) -> Self {
        Self { val: self.val }
    }
}
use std::fmt;

impl<T: fmt::Debug> fmt::Debug for Cell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.borrow())
    }
}
use std::hash::{Hash, Hasher};

impl<T: Hash> Hash for Cell<T> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.borrow().hash(h);
    }
}

use std::ops::{Deref, DerefMut};

impl<T> Deref for Cell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.borrow()
    }
}
impl<T> DerefMut for Cell<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.borrow_mut()
    }
}


impl<T> Drop for Cell<T> {
    fn drop(&mut self) {
        let val = self.borrow();
        drop(val);
           // std::alloc::dealloc(self.val as *mut u8, std::alloc::Layout::new::<T>());
        
    }
}