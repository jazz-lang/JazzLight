use crate::{value::Value, P};

#[derive(Clone, Debug)]
pub struct Module {
    pub name: P<Value>,
    pub globals: Vec<P<Value>>,
    pub loader: P<Value>,
    pub exports: P<Value>,
    pub code: Vec<crate::opcode::Opcode>,
    pub fields: fnv::FnvHashMap<u64, String>,
}

impl Module {
    pub fn new(name: &str) -> Module {
        Module {
            name: P(Value::Str(name.to_owned())),
            globals: vec![],
            loader: P(Value::Null),
            exports: P(Value::Null),
            code: vec![],
            fields: fnv::FnvHashMap::default(),
        }
    }
}

use byteorder::{ByteOrder, LittleEndian};

pub struct Reader {
    pub code: Vec<u8>,
    pub pc: usize,
}

impl Reader {
    pub fn read_u8(&mut self) -> u8 {
        let b = self.code[self.pc];
        self.pc += 1;
        b
    }
    pub fn read_u16(&mut self) -> u16 {
        let short: [u8; 2] = [self.read_u8(), self.read_u8()];
        unsafe { std::mem::transmute(short) }
    }
    pub fn read_u32(&mut self) -> u32 {
        let int: [u16; 2] = [self.read_u16(), self.read_u16()];
        unsafe { std::mem::transmute(int) }
    }

    pub fn read_u64(&mut self) -> u64 {
        let long: [u32; 2] = [self.read_u32(), self.read_u32()];
        unsafe { std::mem::transmute(long) }
    }
}

use crate::opcode::Opcode;
use crate::value::*;
pub fn read_module(mut reader: Reader, name: &str) -> P<Module> {
    let nglobals = reader.read_u32();
    let nfields = reader.read_u32();
    let ncodesize = reader.read_u32();

    let module = P(Module::new(name));
    for _ in 0..nglobals {
        let b = reader.read_u8();
        match b {
            1 => {
                let off = reader.read_u32() as usize;
                let nargs = reader.read_u16() as i32;
                let f = Function {
                    var: FuncVar::Offset(off),
                    nargs: nargs,
                    env: P(Value::Array(P(vec![]))),
                    module: module.clone(),
                    jit: false,
                    yield_point: 0,
                };
                module.borrow_mut().globals.push(P(Value::Func(P(f))));
            }
            _ => unimplemented!(),
        }
    }

    for _ in 0..nfields {
        let key = reader.read_u64();
        let len = reader.read_u16();
        let mut buf = vec![];
        for _ in 0..len {
            buf.push(reader.read_u8());
        }
        let s = String::from_utf8(buf).unwrap();
        module.borrow_mut().fields.insert(key, s);
    }
    let mut code = vec![];
    for _ in 0..ncodesize {
        let op = reader.read_u8();
        match op {
            0 => {
                let int = reader.read_u64();
                code.push(Opcode::LdInt(int as i64));
            }
            1 => {
                let float = reader.read_u64();
                code.push(Opcode::LdFloat(f64::from_bits(float)));
            }
            2 => {
                let len = reader.read_u16();
                let mut buf = vec![];
                for _ in 0..len {
                    buf.push(reader.read_u8());
                }
                let s = String::from_utf8(buf).unwrap();
                code.push(Opcode::LdStr(s));
            }
            3 => {
                let b2 = reader.read_u8();
                let op = match b2 {
                    0 => Opcode::LdFalse,
                    1 => Opcode::LdTrue,
                    2 => Opcode::LdNull,
                    3 => Opcode::LdThis,
                    4 => {
                        let h = reader.read_u64();
                        Opcode::LdField(h)
                    }
                    5 => {
                        let id = reader.read_u32();
                        Opcode::LdLocal(id)
                    }
                    6 => {
                        let id = reader.read_u32();
                        Opcode::LdGlobal(id)
                    }
                    7 => {
                        let id = reader.read_u32();
                        Opcode::LdEnv(id)
                    }
                    8 => {
                        let id = reader.read_u32();
                        Opcode::LdBuiltin(id)
                    }
                    9 => {
                        let id = reader.read_u32();
                        Opcode::LdIndex(id)
                    }
                    10 => Opcode::LdArray,
                    _ => unreachable!(),
                };
                code.push(op);
            }
            4 => {
                let b2 = reader.read_u8();
                let op = match b2 {
                    0 => {
                        let id = reader.read_u32();
                        Opcode::SetLocal(id)
                    }
                    1 => {
                        let id = reader.read_u32();
                        Opcode::SetGlobal(id)
                    }
                    2 => {
                        let id = reader.read_u32();
                        Opcode::SetEnv(id)
                    }
                    3 => {
                        let id = reader.read_u64();
                        Opcode::SetField(id)
                    }
                    4 => Opcode::SetArray,
                    5 => {
                        let idx = reader.read_u32();
                        Opcode::SetIndex(idx)
                    }
                    6 => Opcode::SetThis,
                    _ => unreachable!(),
                };
                code.push(op);
            }
            5 => {
                let count = reader.read_u16();
                code.push(Opcode::Pop(count as u32));
            }
            6 => {
                let count = reader.read_u16();
                code.push(Opcode::Apply(count as _));
            }
            7 => {
                let count = reader.read_u16();
                code.push(Opcode::Call(count as _));
            }
            8 => {
                let count = reader.read_u16();
                code.push(Opcode::TailCall(count as _));
            }
            9 => {
                let count = reader.read_u16();
                code.push(Opcode::ObjCall(count as _));
            }
            10 => {
                let b2 = reader.read_u8();
                match b2 {
                    0 => {
                        let dest = reader.read_u32();
                        code.push(Opcode::Jump(dest));
                    }
                    1 => {
                        let dest = reader.read_u32();
                        code.push(Opcode::JumpIf(dest));
                    }
                    2 => {
                        let dest = reader.read_u32();
                        code.push(Opcode::JumpIfNot(dest));
                    }
                    _ => unreachable!(),
                }
            }
            11 => code.push(Opcode::Ret),
            12 => {
                let c = reader.read_u32();
                code.push(Opcode::MakeEnv(c));
            }
            13 => {
                let c = reader.read_u32();
                code.push(Opcode::MakeArray(c));
            }
            14 => {
                code.push(Opcode::Neg);
            }
            15 => code.push(Opcode::Bool),
            16 => code.push(Opcode::Not),
            17 => code.push(Opcode::IsNull),
            18 => code.push(Opcode::IsNotNull),
            19 => {
                let b2 = reader.read_u8();
                use Opcode::*;
                let op = match b2 {
                    0 => Add,
                    1 => Sub,
                    2 => Mul,
                    3 => Div,
                    4 => Rem,
                    5 => Shl,
                    6 => Shr,
                    7 => UShr,
                    8 => Or,
                    9 => And,
                    10 => Xor,
                    11 => Eq,
                    12 => Neq,
                    13 => Lt,
                    14 => Lte,
                    15 => Gt,
                    16 => Gte,
                    _ => unreachable!(),
                };
                code.push(op);
            }
            20 => code.push(Opcode::Nop),
            21 => code.push(Opcode::TypeOf),
            22 => code.push(Opcode::Hash),
            23 => code.push(Opcode::New),
            24 => code.push(Opcode::Yield),
            25 => code.push(Opcode::Last),
            _ => unimplemented!(),
        }
    }

    for (k, v) in module.fields.iter() {
        crate::vm::FIELDS.borrow_mut().insert(*k, v.clone());
    }

    module.borrow_mut().code = code;
    module
}
