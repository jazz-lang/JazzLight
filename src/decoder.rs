use crate::map::LinkedHashMap;
use crate::vm::opcodes::Opcode;
use crate::vm::value::*;
use crate::vm::Machine;
use byteorder::*;
use std::io::Cursor;
pub struct BytecodeReader<'a> {
    pub machine: &'a mut Machine,
    pub bytecode: Cursor<Vec<u8>>,
    pub pc: usize,
    pub count: usize,
}

impl<'a> BytecodeReader<'a> {
    fn read_u8(&mut self) -> u8 {
        self.pc += 1;
        self.bytecode.read_u8().unwrap()
    }
    fn read_u16(&mut self) -> u16 {
        self.pc += 2;
        self.bytecode.read_u16::<LittleEndian>().unwrap()
        
    }
    fn read_u32(&mut self) -> u32 {
        self.pc += 4;
        self.bytecode.read_u32::<LittleEndian>().unwrap()
    }
    fn read_u64(&mut self) -> u64 {
        self.pc += 8;
        self.bytecode.read_u64::<LittleEndian>().unwrap()
    }

    pub fn read(&mut self) -> Vec<Opcode> {
        let mut strings = vec![];
        let mut opcodes = vec![];
        let count = self.read_u32();
        for _ in 0..count {
            let len = self.read_u32() as usize;
            let mut bytes = vec![];
            for _ in 0..len {
                bytes.push(self.read_u8());
            }
            let s = String::from_utf8(bytes).unwrap();
            strings.push(s);
        }
        let count = self.read_u32();
        for _ in 0..count {
            let byte = self.read_u8();
            match byte {
                0x01 => {
                    let bits = self.read_u64();
                    self.machine
                        .constants
                        .borrow_mut()
                        .push(ValueData::Number(f64::from_bits(bits)))
                }
                0x02 => {
                    let val = self.read_u8();
                    let boolean = if val == 0 { false } else { true };
                    self.machine.constants.borrow_mut().push(ValueData::Bool(boolean));
                }
                0x03 => {
                    let idx = self.read_u32();
                    let s = strings[idx as usize].clone();
                    self.machine.constants.borrow_mut().push(ValueData::String(s));
                }

                0x04 => {
                    self.machine.constants.borrow_mut().push(ValueData::Nil);
                }
                0x05 => {
                    let addr = self.read_u32();
                    let argc = self.read_u16();
                    let sg = self.read_u8();
                    let set = sg & 0x01 != 0;
                    let get = sg & 0x02 != 0;
                    let mut args = vec![];
                    for _ in 0..argc {
                        let idx = self.read_u32();
                        args.push(strings[idx as usize].clone());
                    }
                    let fun = ValueData::Function(new_ref(Function::Regular {
                        environment: new_object(),
                        addr: addr as usize,
                        yield_pos: None,
                        args: args.clone(),
                        constants: self.machine.constants.clone(),
                        code: new_ref(vec![]),
                        yield_env: new_object(),
                        set,
                        get,
                    }));
                    self.machine.constants.borrow_mut().push(fun);
                }
                x => panic!("Unexpected {:x}({})", x, x),
            }
        }

        let mut byte = self.read_u8();
        while byte != 57 && self.pc < self.count {
            match byte {
                54 => {
                    let integer = self.read_u64() as i64;
                    opcodes.push(Opcode::LoadInt(integer))
                }

                1 => {
                    opcodes.push(Opcode::LoadNil);
                }
                2 => {
                    opcodes.push(Opcode::LoadUndef);
                }
                3 => {
                    opcodes.push(Opcode::LoadTrue);
                }
                4 => {
                    opcodes.push(Opcode::LoadFalse);
                }
                5 => {
                    let idx = self.read_u32();
                    opcodes.push(Opcode::LoadConst(idx));
                }
                6 => {
                    let idx = self.read_u32();
                    let s = crate::intern(&strings[idx as usize]);
                    opcodes.push(Opcode::LoadVar(s));
                }
                7 => {
                    opcodes.push(Opcode::Load);
                }
                8 => {
                    opcodes.push(Opcode::Store);
                }
                9 => {
                    opcodes.push(Opcode::NewObj);
                }
                10 => {
                    let count = self.read_u16() as u32;
                    opcodes.push(Opcode::ConstructArray(count));
                }
                11 => {
                    let idx = self.read_u32();
                    let s = crate::intern(&strings[idx as usize]);
                    opcodes.push(Opcode::DeclVar(s));
                }
                12 => {
                    let idx = self.read_u32();
                    let s = crate::intern(&strings[idx as usize]);
                    opcodes.push(Opcode::StoreVar(s));
                }
                13 => {
                    let entry = self.read_u32();
                    opcodes.push(Opcode::PushCatch(entry as _));
                }
                14 => {
                    opcodes.push(Opcode::PopCatch);
                }
                15 => {
                    opcodes.push(Opcode::Throw);
                }
                16 => {
                    let to = self.read_u32();
                    opcodes.push(Opcode::Jump(to));
                }
                17 => {
                    let to = self.read_u32();
                    opcodes.push(Opcode::JumpIf(to));
                }
                18 => {
                    let to = self.read_u32();
                    opcodes.push(Opcode::JumpIfFalse(to));
                }
                19 => {
                    let argc = self.read_u16() as u32;
                    opcodes.push(Opcode::Call(argc));
                }
                20 => {
                    let count = self.read_u16();
                    opcodes.push(Opcode::Pop(count as _));
                }
                21 => {
                    opcodes.push(Opcode::Dup);
                }
                22 => {
                    opcodes.push(Opcode::PopEnv);
                }
                23 => {
                    opcodes.push(Opcode::PushEnv);
                }
                24 => {
                    opcodes.push(Opcode::InitEnv);
                }
                25 => {
                    opcodes.push(Opcode::Label);
                }
                26 => {
                    opcodes.push(Opcode::Yield);
                }
                27 => {
                    opcodes.push(Opcode::Return);
                }
                28 => {
                    opcodes.push(Opcode::NewIter);
                }
                29 => {
                    opcodes.push(Opcode::IterHasNext);
                }
                30 => {
                    opcodes.push(Opcode::IterNext);
                }
                31 => opcodes.push(Opcode::Add),
                32 => opcodes.push(Opcode::Sub),
                33 => opcodes.push(Opcode::Div),
                34 => opcodes.push(Opcode::Mul),
                35 => opcodes.push(Opcode::Rem),
                36 => opcodes.push(Opcode::Shr),
                37 => opcodes.push(Opcode::Shl),
                38 => opcodes.push(Opcode::Gt),
                39 => opcodes.push(Opcode::Lt),
                40 => opcodes.push(Opcode::Ge),
                41 => opcodes.push(Opcode::Le),
                42 => opcodes.push(Opcode::Eq),
                43 => opcodes.push(Opcode::Ne),
                44 => opcodes.push(Opcode::And),
                45 => opcodes.push(Opcode::Or),
                46 => opcodes.push(Opcode::BitXor),
                47 => opcodes.push(Opcode::BitOr),
                48 => opcodes.push(Opcode::BitAnd),
                49 => opcodes.push(Opcode::Not),
                50 => opcodes.push(Opcode::Neg),
                51 => opcodes.push(Opcode::BlockEnd),
                52 => opcodes.push(Opcode::BlockStart),
                x => panic!("{}", x),
            }
            byte = self.read_u8();
        }

        for c in self.machine.constants.borrow().iter() {
            match c {
                ValueData::Function(func) => {
                    let func: &mut Function = &mut func.borrow_mut();
                    match func {
                        Function::Regular { code, .. } => {
                            *code = new_ref(opcodes.clone());
                        }
                        _ => unreachable!(),
                    }
                }
                _ => (),
            }
        }
        opcodes
    }
}
