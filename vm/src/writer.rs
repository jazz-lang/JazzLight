use crate::*;
use byteorder::{LittleEndian, WriteBytesExt};
use value::*;

use crate::opcode::Op;
use crate::reader::{TAG_FLOAT, TAG_FUN, TAG_STRING};
use crate::value::{Function, ValTag};
use hashlink::LinkedHashMap;

pub struct BytecodeWriter {
    pub bytecode: Vec<u8>,
}

impl BytecodeWriter {
    pub fn write_u8(&mut self, x: u8) {
        self.bytecode.push(x);
    }
    pub fn write_u16(&mut self, x: u16) {
        self.bytecode.write_u16::<LittleEndian>(x).unwrap();
    }
    pub fn write_u32(&mut self, x: u32) {
        self.bytecode.write_u32::<LittleEndian>(x).unwrap();
    }
    pub fn write_u64(&mut self, x: u64) {
        self.bytecode.write_u64::<LittleEndian>(x).unwrap();
    }

    pub fn write_module(&mut self, m: Ref<Module>) {
        let mut strings = LinkedHashMap::new();
        let mut i = 0;
        for value in m.borrow().globals.iter() {
            if let Value::String(s) = value {
                strings.insert(s.borrow().clone(), i);
                i += 1;
            }
        }
        let mut globals = vec![];
        for value in m.borrow().globals.iter() {
            match value.tag() {
                ValTag::Func | ValTag::Str | ValTag::Float => globals.push(value.clone()),

                _ => (), // TODO: Add more values to globals
            }
        }

        self.write_u32(strings.len() as _);
        self.write_u32(globals.len() as _);
        self.write_u32(m.borrow().code.len() as _);
        self.write_u8(0);
        for (string, _) in strings.iter() {
            self.write_u32(string.len() as _);
            for byte in string.as_bytes() {
                self.write_u8(*byte);
            }
        }

        for i in 0..globals.len() {
            let global = globals[i].clone();
            match global {
                Value::String(s) => {
                    self.write_u8(TAG_STRING);
                    let idx = strings.get(&*s.borrow()).unwrap();
                    self.write_u32(*idx as _);
                }
                Value::Float(x) => {
                    self.write_u8(TAG_FLOAT);
                    self.write_u64(x.to_bits());
                }
                Value::Function(f) => {
                    let f: &Function = &f.borrow();
                    self.write_u8(TAG_FUN);
                    self.write_u32(f.address as u32);
                    self.write_u16(f.argc as _);
                }
                _ => (),
            }
        }

        for i in 0..m.borrow().code.len() {
            let op = m.borrow().code[i].clone();
            match op {
                Op::LoadNull => self.write_u8(0),
                Op::LoadTrue => self.write_u8(1),
                Op::LoadFalse => self.write_u8(2),
                Op::LoadInt(x) => {
                    self.write_u8(3);
                    self.write_u64(x as _);
                }
                Op::LoadGlobal(idx) => {
                    self.write_u8(4);
                    self.write_u32(idx);
                }
                Op::LoadEnv(idx) => {
                    self.write_u8(5);
                    self.write_u16(idx);
                }
                Op::LoadLocal(idx) => {
                    self.write_u8(6);
                    self.write_u16(idx);
                }
                Op::LoadBuiltin(name) => {
                    self.write_u8(7);
                    let idx = strings.get(&name).unwrap();
                    self.write_u32(*idx as _);
                }
                Op::LoadThis => self.write_u8(8),
                Op::Load => self.write_u8(9),
                Op::Store => self.write_u8(10),
                Op::StoreEnv(idx) => {
                    self.write_u8(11);
                    self.write_u16(idx);
                }
                Op::StoreLocal(idx) => {
                    self.write_u8(12);
                    self.write_u16(idx);
                }
                Op::StoreThis => self.write_u8(13),
                Op::Pop(count) => {
                    self.write_u8(14);
                    self.write_u16(count);
                }
                Op::Call(count) => {
                    self.write_u8(15);
                    self.write_u16(count);
                }
                Op::ObjCall(count) => {
                    self.write_u8(16);
                    self.write_u16(count);
                }
                Op::TailCall(count) => {
                    self.write_u8(17);
                    self.write_u16(count);
                }
                Op::Jump(to) => {
                    self.write_u8(18);
                    self.write_u32(to);
                }
                Op::JumpIf(to) => {
                    self.write_u8(19);
                    self.write_u32(to);
                }
                Op::JumpIfNot(to) => {
                    self.write_u8(20);
                    self.write_u32(to);
                }
                Op::CatchPush(addr) => {
                    self.write_u8(21);
                    self.write_u32(addr);
                }
                Op::Throw => self.write_u8(22),
                Op::Ret => self.write_u8(23),
                Op::MakeEnv(count) => {
                    self.write_u8(24);
                    self.write_u16(count);
                }
                Op::MakeArray(count) => {
                    self.write_u8(25);
                    self.write_u16(count);
                }
                Op::IsNull => self.write_u8(26),
                Op::IsNotNull => self.write_u8(27),
                Op::Add => self.write_u8(28),
                Op::Sub => self.write_u8(29),
                Op::Div => self.write_u8(30),
                Op::Mul => self.write_u8(31),
                Op::Mod => self.write_u8(32),
                Op::Shl => self.write_u8(33),
                Op::Shr => self.write_u8(34),
                Op::UShr => self.write_u8(35),
                Op::Or => self.write_u8(36),
                Op::And => self.write_u8(37),
                Op::Xor => self.write_u8(38),
                Op::Eq => self.write_u8(39),
                Op::Neq => self.write_u8(40),
                Op::Gt => self.write_u8(41),
                Op::Gte => self.write_u8(42),
                Op::Lt => self.write_u8(43),
                Op::Lte => self.write_u8(44),
                Op::Not => self.write_u8(45),
                Op::Neg => self.write_u8(46),
                Op::Hash => self.write_u8(47),
                Op::New => self.write_u8(48),
                Op::Nop => self.write_u8(49),
                Op::Last => self.write_u8(50),
            }
        }
    }
}
