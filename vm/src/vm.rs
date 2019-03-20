pub static mut VM_PTR: *mut VM = 0 as *mut VM;

#[inline]
pub fn get_vm<'a>() -> &'a mut VM {
    unsafe { &mut *VM_PTR }
}

use crate::value::*;
use fnv::FnvHashMap;

pub struct VirtualMachine {
    pub globals: FnvHashMap<u32, GcValue>,
}

impl VirtualMachine {
    pub fn new_global(&mut self, val: GcValue) -> u32 {
        let idx = self.globals.len() as u32;
        self.globals.insert(idx, val);
        return idx;
    }

    pub fn new() -> VirtualMachine {
        Self {
            globals: FnvHashMap::default(),
        }
    }

    pub fn run_function(&mut self, idx: u32) -> GcValue {
        let val = self.globals.get_mut(&idx);
        let val = val.unwrap().clone();
        use crate::frame::Frame;
        let val: &Value = &val.get();
        match val {
            Value::Func(f) => match &f.var {
                FuncVar::Code(code, max_locals) => {
                    let nt = GcValue::new(Value::Null);
                    let mut frame =

                        Frame::new(self, code.clone(), *max_locals, &nt);
                    frame.run()
                }
                _ => unimplemented!(),
            },
            _ => unimplemented!(),
        }
    }
}

pub type VM = VirtualMachine;
