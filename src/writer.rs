use byteorder::{LittleEndian, WriteBytesExt};

use crate::map::LinkedHashMap;
use crate::vm::opcodes::Opcode;
use crate::vm::value::*;
use crate::vm::Machine;

pub struct Writer<'a> {
    pub machine: &'a mut Machine,
    pub code: Vec<Opcode>,
    pub bytecode: Vec<u8>,
    pub names: LinkedHashMap<String, u32>,
}

impl<'a> Writer<'a> {
    pub fn get_str_id(&mut self, x: &str) -> u32 {
        if let Some(id) = self.names.get(x) {
            return *id;
        } else {
            let id = self.names.len();
            self.names.insert(x.to_owned(), id as u32);
            return id as u32;
        }
    }

    fn write_u8(&mut self, x: u8) {
        self.bytecode.push(x);
    }
    fn write_u16(&mut self, x: u16) {
        self.bytecode.write_u16::<LittleEndian>(x).unwrap();
    }
    fn write_u32(&mut self, x: u32) {
        self.bytecode.write_u32::<LittleEndian>(x).unwrap();
    }
    fn write_u64(&mut self, x: u64) {
        self.bytecode.write_u64::<LittleEndian>(x).unwrap();
    }

    pub fn emit(&mut self) {
        for i in 0..self.code.len() {
            let c = self.code[i];
            match c {
                Opcode::LoadVar(name) | Opcode::DeclVar(name) | Opcode::StoreVar(name) => {
                    let string = crate::str(name);
                    self.get_str_id(&string);
                }
                _ => (),
            }
        }
        for i in 0..self.machine.constants.len() {
            let c = self.machine.constants[i].clone();
            if let ValueData::String(s) = &c {
                self.get_str_id(s);
            }
        }
        self.write_u32(self.names.len() as u32);
        for (string, idx) in self.names.clone().iter() {
            self.write_u32(*idx);
            self.write_u32(string.len() as _);
            for byte in string.as_bytes() {
                self.write_u8(*byte);
            }
        }
        self.write_u32(self.machine.constants.len() as _);

        for i in 0..self.machine.constants.len() {
            let c = self.machine.constants[i].clone();

            match &c {
                ValueData::Number(x) => {
                    self.write_u8(0x01);
                    let bits = x.to_bits();
                    self.write_u64(bits);
                }
                ValueData::Bool(boolean) => {
                    self.write_u8(0x02);
                    self.write_u8(*boolean as u8);
                }
                ValueData::String(s) => {
                    let id = self.get_str_id(s);
                    self.write_u8(0x03);
                    self.write_u32(id);
                }
                ValueData::Nil => {
                    self.write_u8(0x04);
                }
                ValueData::Function(func) => {
                    let func: &Function = &func.borrow();
                    match func {
                        Function::Regular { addr, args, .. } => {
                            self.write_u8(0x05);
                            self.write_u32(*addr as u32);
                            self.write_u16(args.len() as _);
                            self.write_u8(0x00);
                            for arg in args.iter() {
                                let id = self.get_str_id(arg);
                                self.write_u32(id);
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }
        }

        for i in 0..self.code.len() {
            let op = self.code[i];

            match op {
                Opcode::LoadInt(x) => {
                    self.write_u8(54);
                    self.write_u64(x as u64);
                }
                Opcode::LoadNil => {
                    self.write_u8(1);
                }
                Opcode::LoadUndef => {
                    self.write_u8(2);
                }
                Opcode::LoadTrue => {
                    self.write_u8(3);
                }
                Opcode::LoadFalse => {
                    self.write_u8(4);
                }
                Opcode::LoadConst(idx) => {
                    self.write_u8(5);
                    self.write_u32(idx);
                }
                Opcode::LoadVar(name) => {
                    let string = crate::str(name);
                    let id = self.get_str_id(&string);
                    self.write_u8(6);
                    self.write_u32(id);
                }
                Opcode::Load => {
                    self.write_u8(7);
                }
                Opcode::Store => {
                    self.write_u8(8);
                }
                Opcode::NewObj => {
                    self.write_u8(9);
                }
                Opcode::ConstructArray(count) => {
                    self.write_u8(10);
                    self.write_u16(count as u16);
                }
                Opcode::DeclVar(name) => {
                    let string = crate::str(name);
                    let id = self.get_str_id(&string);
                    self.write_u8(11);
                    self.write_u32(id);
                }
                Opcode::StoreVar(name) => {
                    let string = crate::str(name);
                    let id = self.get_str_id(&string);
                    self.write_u8(12);
                    self.write_u32(id);
                }
                Opcode::PushCatch(entry) => {
                    self.write_u8(13);
                    self.write_u32(entry as u32);
                }
                Opcode::PopCatch => {
                    self.write_u8(14);
                }
                Opcode::Throw => {
                    self.write_u8(15);
                }
                Opcode::Jump(to) => {
                    self.write_u8(16);
                    self.write_u32(to);
                }
                Opcode::JumpIf(to) => {
                    self.write_u8(17);
                    self.write_u32(to);
                }
                Opcode::JumpIfFalse(to) => {
                    self.write_u8(18);
                    self.write_u32(to);
                }
                Opcode::Call(argc) => {
                    self.write_u8(19);
                    self.write_u16(argc as u16);
                }
                Opcode::Pop(count) => {
                    self.write_u8(20);
                    self.write_u16(count as _);
                }
                Opcode::Dup => {
                    self.write_u8(21);
                }
                Opcode::PopEnv => {
                    self.write_u8(22);
                }
                Opcode::PushEnv => {
                    self.write_u8(23);
                }
                Opcode::InitEnv => {
                    self.write_u8(24);
                }
                Opcode::Label => {
                    self.write_u8(25);
                }
                Opcode::Yield => {
                    self.write_u8(26);
                }
                Opcode::Return => {
                    self.write_u8(27);
                }
                Opcode::NewIter => {
                    self.write_u8(28);
                }
                Opcode::IterHasNext => {
                    self.write_u8(29);
                }
                Opcode::IterNext => {
                    self.write_u8(30);
                }
                Opcode::Add => {
                    self.write_u8(31);
                }
                Opcode::Sub => {
                    self.write_u8(32);
                }
                Opcode::Div => {
                    self.write_u8(33);
                }
                Opcode::Mul => {
                    self.write_u8(34);
                }
                Opcode::Rem => {
                    self.write_u8(35);
                }
                Opcode::Shr => {
                    self.write_u8(36);
                }
                Opcode::Shl => {
                    self.write_u8(37);
                }
                Opcode::Gt => {
                    self.write_u8(38);
                }
                Opcode::Lt => {
                    self.write_u8(39);
                }
                Opcode::Ge => {
                    self.write_u8(40);
                }
                Opcode::Le => {
                    self.write_u8(41);
                }
                Opcode::Eq => {
                    self.write_u8(42);
                }
                Opcode::Ne => {
                    self.write_u8(43);
                }
                Opcode::And => {
                    self.write_u8(44);
                }
                Opcode::Or => {
                    self.write_u8(45);
                }
                Opcode::BitXor => {
                    self.write_u8(46);
                }
                Opcode::BitOr => {
                    self.write_u8(47);
                }
                Opcode::BitAnd => {
                    self.write_u8(48);
                }
                Opcode::Not => {
                    self.write_u8(49);
                }
                Opcode::Neg => {
                    self.write_u8(50);
                }
                Opcode::BlockEnd => {
                    self.write_u8(51);
                }
                Opcode::BlockStart => {
                    self.write_u8(52);
                }
            }
        }
        self.write_u8(57);
        //self.write_u8(0);
    }
}
