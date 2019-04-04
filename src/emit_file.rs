use byteorder::*;
use jazzvm::module::Module;
use jazzvm::opcode::Opcode;
use jazzvm::value::*;
use jazzvm::P;
pub fn compile(m: &mut P<Module>) -> Result<Vec<u8>, std::io::Error> {
    let mut code: Vec<u8> = vec![];
    //let codesize;
    let mut globals_size: u32 = 0;

    code.write_u32::<LittleEndian>(0)?; // globals size
    code.write_u32::<LittleEndian>(0)?; // fields size
    code.write_u32::<LittleEndian>(0)?; // code size
    for global in m.globals.iter() {
        let g: &P<Value> = global;
        match g.borrow() {
            Value::Func(f) => {
                code.push(1);
                match f.var {
                    FuncVar::Offset(off) => {
                        code.write_u32::<LittleEndian>(off as u32).unwrap();
                    }
                    _ => panic!("No native functions in compile module"),
                }
                code.write_u16::<LittleEndian>(f.nargs as u16).unwrap();
                globals_size += 1;
            }
            _ => unimplemented!(),
        }
    }
    let be = globals_size.to_le_bytes();
    code[0] = be[0];
    code[1] = be[1];
    code[2] = be[2];
    code[3] = be[3];
    let mut fields_size: u32 = 0;
    for (key, field) in m.fields.iter() {
        code.write_u64::<LittleEndian>(*key)?;

        for byte in field.as_bytes().iter() {
            code.write_u8(*byte)?;
        }
        code.write_u8(b'\0')?;
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
                c.write_u64::<LittleEndian>(*i as u64)?;
            }
            Opcode::LdFloat(f) => {
                c.push(1);
                c.write_u64::<LittleEndian>(f.to_bits())?;
            }
            Opcode::LdStr(s) => {
                c.push(2);
                //c.write_u16::<LittleEndian>(s.len() as u16)?;
                for byte in s.as_bytes().iter() {
                    c.push(*byte);
                }
                c.push(b'\0');
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
                c.write_u64::<LittleEndian>(*field)?;
            }
            Opcode::LdLocal(u) => {
                c.push(3);
                c.push(5);
                c.write_u32::<LittleEndian>(*u)?;
            }
            Opcode::LdGlobal(u) => {
                c.push(3);
                c.push(6);
                c.write_u32::<LittleEndian>(*u)?;
            }
            Opcode::LdEnv(u) => {
                c.push(3);
                c.push(7);
                c.write_u32::<LittleEndian>(*u)?;
            }
            Opcode::LdBuiltin(u) => {
                c.push(3);
                c.push(8);
                c.write_u32::<LittleEndian>(*u)?;
            }
            Opcode::LdIndex(u) => {
                c.push(3);
                c.push(9);
                c.write_u32::<LittleEndian>(*u)?;
            }
            Opcode::LdArray => {
                c.push(3);
                c.push(10);
            }
            Opcode::SetLocal(u) => {
                c.push(4);
                c.push(0);
                c.write_u32::<LittleEndian>(*u)?;
            }
            Opcode::SetGlobal(u) => {
                c.push(4);
                c.push(1);
                c.write_u32::<LittleEndian>(*u)?;
            }
            Opcode::SetEnv(u) => {
                c.push(4);
                c.push(2);
                c.write_u32::<LittleEndian>(*u)?;
            }
            Opcode::SetField(u) => {
                c.push(4);
                c.push(3);
                c.write_u64::<LittleEndian>(*u)?;
            }
            Opcode::SetArray => {
                c.push(4);
                c.push(4);
            }
            Opcode::SetIndex(idx) => {
                c.push(4);
                c.push(5);
                c.write_u32::<LittleEndian>(*idx)?;
            }
            Opcode::SetThis => {
                c.push(4);
                c.push(6);
            }
            Opcode::Pop(count) => {
                c.push(5);
                c.write_u16::<LittleEndian>(*count as u16)?;
            }
            Opcode::Apply(count) => {
                c.push(6);
                c.write_u16::<LittleEndian>(*count as u16)?;
            }
            Opcode::Call(count) => {
                c.push(7);
                c.write_u16::<LittleEndian>(*count as u16)?;
            }
            Opcode::TailCall(count) => {
                c.push(8);
                c.write_u16::<LittleEndian>(*count as u16)?;
            }
            Opcode::ObjCall(count) => {
                c.push(9);
                c.write_u16::<LittleEndian>(*count as u16)?;
            }
            Opcode::Jump(j) => {
                c.push(10);
                c.push(0);
                c.write_u32::<LittleEndian>(*j)?;
            }
            Opcode::JumpIf(j) => {
                c.push(10);
                c.push(1);
                c.write_u32::<LittleEndian>(*j)?;
            }
            Opcode::JumpIfNot(j) => {
                c.push(10);
                c.push(2);
                c.write_u32::<LittleEndian>(*j)?;
            }
            Opcode::Ret => {
                c.push(11);
            }
            Opcode::MakeEnv(e) => {
                c.push(12);
                c.write_u32::<LittleEndian>(*e)?;
            }
            Opcode::MakeArray(arr) => {
                c.push(13);
                c.write_u32::<LittleEndian>(*arr)?;
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
