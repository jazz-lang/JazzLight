use crate::value::{Function, Object};
use crate::*;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;
use value::*;

pub struct BytecodeReader<'a> {
    pub bytes: Cursor<&'a [u8]>,
}

pub const TAG_STRING: u8 = 0;
pub const TAG_FLOAT: u8 = 1;
pub const TAG_DBGINFO: u8 = 2;
pub const TAG_FUN: u8 = 3;

impl<'a> BytecodeReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes: Cursor::new(bytes),
        }
    }

    pub fn read_u8(&mut self) -> u8 {
        self.bytes.read_u8().unwrap()
    }
    pub fn read_u16(&mut self) -> u16 {
        self.bytes.read_u16::<LittleEndian>().unwrap()
    }
    pub fn read_u32(&mut self) -> u32 {
        self.bytes.read_u32::<LittleEndian>().unwrap()
    }
    pub fn read_u64(&mut self) -> u64 {
        self.bytes.read_u64::<LittleEndian>().unwrap()
    }
    /// Read debug information
    pub fn read_dbginfo(
        &mut self,
        strings: &Vec<String>,
        csize: usize,
    ) -> HashMap<u32, (usize, String)> {
        let mut map = HashMap::new();
        for i in 0..csize {
            let line = self.read_u32() as usize;
            let string_id = self.read_u32() as usize;
            let string = strings[string_id].clone();
            map.insert(i as _, (line, string));
        }
        map
    }

    pub fn read_module(&mut self) -> Ref<Module> {
        let m = Ref(Module {
            exports: Value::Object(Ref(Object {
                prototype: None,
                table: Default::default(),
            })),
            trace_info: HashMap::new(),
            code: vec![],
            globals: vec![],
        });
        let mut strings = Vec::new();
        let count_strings = self.read_u32();
        let count_globals = self.read_u32();
        let code_size = self.read_u32();
        let has_dbginfo = self.read_u8();
        for _ in 0..count_strings {
            let len = self.read_u32();
            let mut bytes = vec![];
            for _ in 0..len {
                bytes.push(self.read_u8());
            }
            strings.push(String::from_utf8(bytes).unwrap());
        }

        if has_dbginfo == 1 {
            m.borrow_mut().trace_info = self.read_dbginfo(&strings, code_size as _);
        }

        for _ in 0..count_globals {
            let tag = self.read_u8();
            match tag {
                TAG_STRING => {
                    let idx = self.read_u32() as usize;
                    m.borrow_mut()
                        .globals
                        .push(Value::String(Ref(strings[idx].clone())));
                }
                TAG_FLOAT => {
                    let bits = self.read_u64();
                    let float = f64::from_bits(bits);
                    m.borrow_mut().globals.push(Value::Float(float));
                }
                TAG_FUN => {
                    let at = self.read_u32();
                    let argc = self.read_u16();
                    let fun = Function {
                        address: at as _,
                        native: false,
                        env: Value::Array(Ref(vec![])),
                        argc: argc as _,
                        module: Some(m.clone()),
                    };
                    m.borrow_mut().globals.push(Value::Function(Ref(fun)));
                }
                TAG_DBGINFO => {
                    m.borrow_mut().trace_info = self.read_dbginfo(&strings, code_size as _);
                }
                _ => unreachable!(),
            }
        }
        use opcode::Op;
        for _ in 0..code_size {
            let op = self.read_u8();
            let opcode = match op {
                0 => Op::LoadNull,
                1 => Op::LoadTrue,
                2 => Op::LoadFalse,
                3 => {
                    let int = self.read_u64() as i64;
                    Op::LoadInt(int)
                }
                4 => {
                    let idx = self.read_u32();
                    Op::LoadGlobal(idx)
                }
                5 => {
                    let idx = self.read_u16();
                    Op::LoadEnv(idx)
                }
                6 => {
                    let idx = self.read_u16();
                    Op::LoadLocal(idx)
                }
                7 => {
                    let name = self.read_u32() as usize;
                    let name = strings[name].clone();
                    Op::LoadBuiltin(name)
                }
                8 => Op::LoadThis,
                9 => Op::Load,
                10 => Op::Store,
                11 => {
                    let idx = self.read_u16();
                    Op::StoreEnv(idx)
                }
                12 => {
                    let idx = self.read_u16();
                    Op::StoreLocal(idx)
                }
                13 => Op::StoreThis,
                14 => {
                    let count = self.read_u16();
                    Op::Pop(count)
                }
                15 => {
                    let count = self.read_u16();
                    Op::Call(count)
                }
                16 => {
                    let count = self.read_u16();
                    Op::ObjCall(count)
                }
                17 => {
                    let count = self.read_u16();
                    Op::TailCall(count)
                }
                18 => {
                    let to = self.read_u32();
                    Op::Jump(to)
                }
                19 => {
                    let to = self.read_u32();
                    Op::JumpIf(to)
                }
                20 => {
                    let to = self.read_u32();
                    Op::JumpIfNot(to)
                }
                21 => {
                    let addr = self.read_u32();
                    Op::CatchPush(addr)
                }
                22 => Op::Throw,
                23 => Op::Ret,
                24 => {
                    let count = self.read_u16();
                    Op::MakeEnv(count)
                }
                25 => {
                    let count = self.read_u16();
                    Op::MakeArray(count)
                }
                26 => Op::IsNull,
                27 => Op::IsNotNull,
                28 => Op::Add,
                29 => Op::Sub,
                30 => Op::Div,
                31 => Op::Mul,
                32 => Op::Mod,
                33 => Op::Shl,
                34 => Op::Shr,
                35 => Op::UShr,
                36 => Op::Or,
                37 => Op::And,
                38 => Op::Xor,
                39 => Op::Eq,
                40 => Op::Neq,
                41 => Op::Gt,
                42 => Op::Gte,
                43 => Op::Lt,
                44 => Op::Lte,
                45 => Op::Not,
                46 => Op::Neg,
                47 => Op::Hash,
                48 => Op::New,
                49 => Op::Nop,
                50 => Op::Last,
                _ => unreachable!(),
            };
            m.borrow_mut().code.push(opcode);
        }

        m
    }
}
