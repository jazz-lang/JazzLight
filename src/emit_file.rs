use jazzvm::module::Module;
use jazzvm::opcode::Opcode;
use jazzvm::value::*;
use jazzvm::P;

pub trait Encoder {
    fn write_u8(&mut self, b: u8);
    fn write_u16(&mut self, b: u16);
    fn write_u32(&mut self, b: u32);
    fn write_u64(&mut self, b: u64);
}

impl Encoder for Vec<u8> {
    fn write_u8(&mut self, b: u8) {
        self.push(b);
    }
    fn write_u16(&mut self, b: u16) {
        let bytes: [u8; 2] = unsafe { std::mem::transmute(b) };
        self.write_u8(bytes[0]);
        self.write_u8(bytes[1]);
    }

    fn write_u32(&mut self, b: u32) {
        let bytes: [u8; 4] = unsafe { std::mem::transmute(b) };
        self.write_u8(bytes[0]);
        self.write_u8(bytes[1]);
        self.write_u8(bytes[2]);
        self.write_u8(bytes[3]);
    }
    fn write_u64(&mut self, b: u64) {
        let bytes: [u8; 8] = unsafe { std::mem::transmute(b) };
        self.write_u8(bytes[0]);
        self.write_u8(bytes[1]);
        self.write_u8(bytes[2]);
        self.write_u8(bytes[3]);
        self.write_u8(bytes[4]);
        self.write_u8(bytes[5]);
        self.write_u8(bytes[6]);
        self.write_u8(bytes[7]);
    }
}

pub fn compile(m: &mut P<Module>) -> Result<Vec<u8>, std::io::Error> {
    let mut code: Vec<u8> = vec![];
    //let codesize;
    let mut globals_size: u32 = 0;

    code.write_u32(0); // globals size
    code.write_u32(0); // fields size
    code.write_u32(0); // code size
    for global in m.globals.iter() {
        let g: &P<Value> = global;
        match g.borrow() {
            Value::Func(f) => {
                code.push(1);
                match f.var {
                    FuncVar::Offset(off) => {
                        code.write_u32(off as u32);
                    }
                    _ => panic!("No native functions in compile module"),
                }
                code.write_u16(f.nargs as u16);
                globals_size += 1;
            }
            _ => (),
        }
    }
    let be = globals_size.to_le_bytes();
    code[0] = be[0];
    code[1] = be[1];
    code[2] = be[2];
    code[3] = be[3];
    let mut fields_size: u32 = 0;
    for (key, field) in m.fields.iter() {
        code.write_u64(*key);

        for byte in field.as_bytes().iter() {
            code.write_u8(*byte);
        }
        code.write_u8(b'\0');
        fields_size += 1;
    }
    let be = fields_size.to_le_bytes();
    code[4] = be[0];
    code[5] = be[1];
    code[6] = be[2];
    code[7] = be[3];

    let mut c = vec![];

    for op in m.code.iter() {
        match op {
            Opcode::LdInt(i) => {
                c.push(0);
                if *i < 0 {
                    c.push(1);
                } else {
                    c.push(0);
                }
                c.write_u64(*i as u64);
            }
            Opcode::LdFloat(f) => {
                c.push(1);
                c.write_u64(f.to_bits());
            }
            Opcode::LdStr(s) => {
                c.push(2);
                c.write_u32(s.len() as u32);
                for byte in s.as_bytes().iter() {
                    c.push(*byte);
                }
            }
            Opcode::LdTrue => {
                c.push(3); // ld
                c.push(1); // true
            }
            Opcode::LdFalse => {
                c.push(3); // ld
                c.push(0); // false
            }

            Opcode::LdNull => {
                c.push(3); // ld
                c.push(2); // null
            }
            Opcode::LdThis => {
                c.push(3); // ld
                c.push(3); // this
            }
            Opcode::LdField(field) => {
                c.push(3);
                c.push(4);
                c.write_u64(*field);
            }
            Opcode::LdLocal(u) => {
                c.push(3);
                c.push(5);
                c.write_u32(*u);
            }
            Opcode::LdGlobal(u) => {
                c.push(3);
                c.push(6);
                c.write_u32(*u);
            }
            Opcode::LdEnv(u) => {
                c.push(3);
                c.push(7);
                c.write_u32(*u);
            }
            Opcode::LdBuiltin(u) => {
                c.push(3);
                c.push(8);
                c.write_u32(*u);
            }
            Opcode::LdIndex(u) => {
                c.push(3);
                c.push(9);
                c.write_u32(*u);
            }
            Opcode::LdArray => {
                c.push(3);
                c.push(10);
            }
            Opcode::SetLocal(u) => {
                c.push(4);
                c.push(0);
                c.write_u32(*u);
            }
            Opcode::SetGlobal(u) => {
                c.push(4);
                c.push(1);
                c.write_u32(*u);
            }
            Opcode::SetEnv(u) => {
                c.push(4);
                c.push(2);
                c.write_u32(*u);
            }
            Opcode::SetField(u) => {
                c.push(4);
                c.push(3);
                c.write_u64(*u);
            }
            Opcode::SetArray => {
                c.push(4);
                c.push(4);
            }
            Opcode::SetIndex(idx) => {
                c.push(4);
                c.push(5);
                c.write_u32(*idx);
            }
            Opcode::SetThis => {
                c.push(4);
                c.push(6);
            }
            Opcode::Pop(count) => {
                c.push(5);
                c.write_u16(*count as u16);
            }
            Opcode::Apply(count) => {
                c.push(6);
                c.write_u16(*count as u16);
            }
            Opcode::Call(count) => {
                c.push(7);
                c.write_u16(*count as u16);
            }
            Opcode::TailCall(count) => {
                c.push(8);
                c.write_u16(*count as u16);
            }
            Opcode::ObjCall(count) => {
                c.push(9);
                c.write_u16(*count as u16);
            }
            Opcode::Jump(j) => {
                c.push(10);
                c.push(0);
                c.write_u32(*j);
            }
            Opcode::JumpIf(j) => {
                c.push(10);
                c.push(1);
                c.write_u32(*j);
            }
            Opcode::JumpIfNot(j) => {
                c.push(10);
                c.push(2);
                c.write_u32(*j);
            }
            Opcode::Ret => {
                c.push(11);
            }
            Opcode::MakeEnv(e) => {
                c.push(12);
                c.write_u32(*e);
            }
            Opcode::MakeArray(arr) => {
                c.push(13);
                c.write_u32(*arr);
            }
            Opcode::Neg => {
                c.push(14);
            }
            Opcode::Bool => {
                c.push(15);
            }
            Opcode::Not => {
                c.push(16);
            }
            Opcode::IsNull => {
                c.push(17);
            }
            Opcode::IsNotNull => {
                c.push(18);
            }
            Opcode::Add => {
                c.push(19);
                c.push(0);
            }
            Opcode::Sub => {
                c.push(19);
                c.push(1);
            }
            Opcode::Mul => {
                c.push(19);
                c.push(2);
            }
            Opcode::Div => {
                c.push(19);
                c.push(3);
            }
            Opcode::Rem => {
                c.push(19);
                c.push(4);
            }
            Opcode::Shl => {
                c.push(19);
                c.push(5);
            }
            Opcode::Shr => {
                c.push(19);
                c.push(6);
            }
            Opcode::Nop => {
                c.push(20);
            }
            Opcode::UShr => {
                c.push(19);
                c.push(7);
            }
            Opcode::Or => {
                c.push(19);
                c.push(8);
            }
            Opcode::And => {
                c.push(19);
                c.push(9);
            }
            Opcode::Xor => {
                c.push(19);
                c.push(10);
            }
            Opcode::Eq => {
                c.push(19);
                c.push(11);
            }
            Opcode::Neq => {
                c.push(19);
                c.push(12);
            }
            Opcode::Lt => {
                c.push(19);
                c.push(13);
            }
            Opcode::Lte => {
                c.push(19);
                c.push(14);
            }
            Opcode::Gt => {
                c.push(19);
                c.push(15);
            }
            Opcode::Gte => {
                c.push(19);
                c.push(16);
            }
            Opcode::TypeOf => {
                c.push(21);
            }
            Opcode::Hash => {
                c.push(22);
            }
            Opcode::New => {
                c.push(23);
            }
            Opcode::Yield => {
                c.push(24);
            }
            Opcode::Last => {
                c.push(25);
            }
            Opcode::JumpTable => unimplemented!(),
        }
    }

    let bytes: [u8; 4] = unsafe { std::mem::transmute(m.code.len() as u32) };
    code[8] = bytes[0];
    code[9] = bytes[1];
    code[10] = bytes[2];
    code[11] = bytes[3];

    code.extend(c.iter());

    Ok(code)
}
