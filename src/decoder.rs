use crate::map::LinkedHashMap;
use crate::vm::opcodes::Opcode;
use crate::vm::value::*;
use crate::vm::Machine;

pub struct BytecodeReader<'a> {
    pub machine: &'a mut Machine,
    pub bytecode: Vec<u8>,
    pub pc: usize,
}

impl<'a> BytecodeReader<'a> {
    fn read_u8(&mut self) -> u8 {
        self.pc += 1;
        self.bytecode[self.pc - 1]
    }
    fn read_u16(&mut self) -> u16 {
        unsafe { std::mem::transmute([self.read_u8(), self.read_u8()]) }
    }
    fn read_u32(&mut self) -> u32 {
        unsafe { std::mem::transmute([self.read_u16(), self.read_u16()]) }
    }
    fn read_u64(&mut self) -> u64 {
        unsafe { std::mem::transmute([self.read_u32(), self.read_u32()]) }
    }

    pub fn read(&mut self) -> Vec<Opcode> {
        let mut strings = LinkedHashMap::new();
        let mut opcodes = vec![];
        self.read_u8();
        while self.bytecode[self.pc] != 0x24 {
            let idx = self.read_u32();
            let len = self.read_u32() as usize;
            let mut bytes = vec![];
            for _ in 0..len {
                bytes.push(self.read_u8());
            }
            let s = String::from_utf8(bytes).unwrap();
            strings.insert(idx, s);
        }
        self.read_u8();
        let count = self.read_u32();
        for _ in 0..count {
            let byte = self.read_u8();
            match byte {
                0x01 => {
                    let bits = self.read_u64();
                    self.machine
                        .constants
                        .push(ValueData::Number(f64::from_bits(bits)))
                }
                0x02 => {
                    let val = self.read_u8();
                    let boolean = if val == 0 { false } else { true };
                    self.machine.constants.push(ValueData::Bool(boolean));
                }
                0x03 => {
                    let idx = self.read_u32();
                    let s = strings.get(&idx).unwrap().clone();
                    self.machine.constants.push(ValueData::String(s));
                }

                0x04 => {
                    self.machine.constants.push(ValueData::Nil);
                }
                0x05 => {
                    let addr = self.read_u32();
                    let argc = self.read_u16();
                    let mut args = vec![];
                    for _ in 0..argc {
                        let idx = self.read_u32();
                        args.push(strings.get(&idx).unwrap().clone());
                    }
                    let fun = ValueData::Function(new_ref(Function::Regular {
                        environment: new_object(),
                        addr: addr as usize,
                        yield_pos: None,
                        args: args.clone(),
                        //constants: ref_,
                        code: wrc::WRC::new(std::cell::RefCell::new(vec![])),
                        yield_env: new_object(),
                    }));
                    self.machine.constants.push(fun);
                }
                x => panic!("Unexpected {:x}({})", x, x),
            }
        }

        let mut byte = self.read_u8();
        while byte != 53 && self.pc < self.bytecode.len() {
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
                    let s = crate::intern(strings.get(&idx).unwrap());
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
                    let s = crate::intern(strings.get(&idx).unwrap());
                    opcodes.push(Opcode::DeclVar(s));
                }
                12 => {
                    let idx = self.read_u32();
                    let s = crate::intern(strings.get(&idx).unwrap());
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
                _ => {
                    // we can transmute there because other opcodes is one byte size
                    let op: Opcode = unsafe { std::mem::transmute_copy(&byte) };
                    opcodes.push(op);
                }
            }
            if self.pc == self.bytecode.len() {
                break;
            }
            byte = self.read_u8();
        }

        for c in self.machine.constants.iter() {
            match c {
                ValueData::Function(func) => {
                    let func: &mut Function = &mut func.borrow_mut();
                    match func {
                        Function::Regular { code, .. } => {
                            *code = wrc::WRC::new(std::cell::RefCell::new(opcodes.clone()));
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
