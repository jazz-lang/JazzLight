use super::value::*;
use super::*;
use crate::token::Position;
//use crate::vm::runtime::array::array_object;

use crate::vm::runtime::math::math_object;

pub mod array;
pub mod console;
pub mod math;
pub mod object;

pub macro decl_fun {
    (function $name : ident ($frame: ident,$this: ident $($arg: ident),*)  $b: block ) => {
        pub fn $name ( $frame: &mut Frame<'_>,$this: Value,args: &[Value]) -> Result<Value,ValueData> {
            let mut ___i = 0;
            $(

                let $arg = args[___i].clone();
                ___i += 1;
            )*
            $b
        }
    };

}

pub fn builtin_instanceof(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    if args.len() < 2 {
        return Err(new_error(-1, None, "instanceof: two arguments expected"));
    }
    let x: &ValueData = &args[0].borrow();
    let y: &ValueData = &args[1].borrow();
    let (x, y) = match (x, y) {
        (ValueData::Object(x), ValueData::Object(y)) => (x, y),
        _ => {
            return Err(new_error(
                -1,
                None,
                "instanceof: expected objects as arguments",
            ))
        }
    };

    let check = instanceof(x, y);
    Ok(new_ref(ValueData::Bool(check)))
}

pub fn builtin_print(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    for arg in args.iter() {
        print!("{}", arg.borrow());
    }
    println!("");
    Ok(new_ref(ValueData::Nil))
}

pub fn new_exfunc(f: fn(&mut Frame<'_>, Value, &[Value]) -> Result<Value, ValueData>) -> Value {
    new_ref(ValueData::Function(new_ref(Function::Native(f as usize))))
}

pub fn builtin_gc(_: &mut Frame<'_>, _: Value, _: &[Value]) -> Result<Value, ValueData> {
    crate::ngc::gc_collect_not_par();
    //crate::gc::gc::sweep();
    Ok(new_ref(ValueData::Nil))
}

pub fn enable_stats(_: &mut Frame<'_>, _: Value, _: &[Value]) -> Result<Value, ValueData> {
    crate::ngc::gc_enable_stats();
    Ok(new_ref(ValueData::Nil))
}

pub fn builtin_spawn(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    if args.is_empty() {
        return Err(new_error(0, None, "function expected"));
    }

    let val = args[0].clone();
    let val: &ValueData = &val.borrow();
    match val {
        ValueData::Function(fun) => {
            let fun: &Function = &fun.borrow();
            match fun {
                Function::Regular {
                    environment,
                    args,
                    code,
                    addr,
                    //constants,
                    ..
                } => {
                    let func = Function::Regular {
                        environment: environment.clone(),
                        args: args.clone(),
                        code: code.clone(),
                        addr: *addr,
                        //constants: constants.clone(),
                        yield_env: new_object(),
                        yield_pos: None,
                    };

                    return Ok(new_ref(ValueData::Function(new_ref(func))));
                }
                _ => return Err(new_error(0, None, "regular function expected")),
            }
        }
        _ => return Err(new_error(0, None, "function expected")),
    }
}

pub fn type_of(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    let arg = args[0].clone();
    let val: &ValueData = &arg.borrow();
    let name = match val {
        ValueData::Number(_) => "number",
        ValueData::Nil => "nil",
        ValueData::Undefined => "undefined",
        ValueData::String(_) => "string",
        ValueData::Object(_) => "object",
        ValueData::Array(_) => "array",
        ValueData::Function(_) => "function",
        ValueData::Bool(_) => "bool",
        ValueData::Iterator(_) => "iterator",
    };
    Ok(new_ref(ValueData::String(name.to_owned())))
}

pub fn require(_frame: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    let name = String::from(args[0].borrow().clone());
    let cur_dir = std::env::current_dir()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let cur_path = format!("{}/{}", cur_dir, name);
    let path = if std::path::Path::new(&cur_path).exists() {
        cur_path
    } else {
        let home_dir = option_env!("JAZZ_HOME");
        if let Some(home_dir) = home_dir {
            format!("{}/{}", home_dir, name)
        } else {
            name
        }
    };
    use std::fs::File;
    use std::io::Read;
    let mut file = File::open(&path).unwrap();
    let mut m = Machine::new();
    let mut code = vec![];

    file.read_to_end(&mut code).unwrap();

    let mut reader = crate::decoder::BytecodeReader {
        machine: &mut m,
        bytecode: code,
        pc: 0,
    };

    let code = reader.read();
    let mut frame1 = Frame::new(&mut m);
    frame1.code = wrc::WRC::new(std::cell::RefCell::new(code));
    crate::vm::runtime::register_builtins(frame1.env.clone());
    frame1.execute();

    let exports = get_variable(&frame1.env, "exports", &Position::new(0, 0)).unwrap();
    Ok(exports)
}

pub fn len(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    let arg = args[0].clone();
    let arg: &ValueData = &arg.borrow();
    match arg {
        ValueData::Array(arr) => Ok(new_ref(ValueData::Number(arr.borrow().len() as f64))),
        ValueData::Object(object) => Ok(new_ref(ValueData::Number(
            object.borrow().table.len() as f64
        ))),
        ValueData::String(s) => Ok(new_ref(ValueData::Number(s.len() as f64))),
        _ => Ok(new_ref(ValueData::Nil)),
    }
}

pub fn range(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    let x = args[0].clone();
    let y = args[1].clone();
    let x = i64::from(x.borrow().clone());
    let y = i64::from(y.borrow().clone());
    let mut array = vec![];
    if x > y {
        for i in (y..x).rev() {
            array.push(new_ref(ValueData::Number(i as f64)));
        }
    } else {
        for i in x..y {
            array.push(new_ref(ValueData::Number(i as f64)));
        }
    }
    Ok(new_ref(ValueData::Iterator(new_ref(ValueIter {
        values: array,
    }))))
}

use std::fs::File;
use std::io::{Read, Write};

pub fn file(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    if args.len() < 1 {
        return Err(ValueData::String("file name expected".to_owned()));
    }
    let file_name = args[0].clone();
    let file_name: &ValueData = &file_name.borrow();
    let file_name = if let ValueData::String(s) = file_name {
        s.to_owned()
    } else {
        return Err(ValueData::String("file name should be string".to_owned()));
    };

    let file_object = new_object();
    file_object
        .borrow_mut()
        .set("name", ValueData::String(file_name.clone()));
    let file = File::open(&file_name);
    let mut file = match file {
        Ok(file) => file,
        Err(e) => return Err(new_error(-1, None, &e.to_string())),
    };

    let mut bytes = vec![];
    match file.read_to_end(&mut bytes) {
        Ok(_) => (),
        Err(e) => return Err(new_error(-1, None, &e.to_string())),
    }

    let bytes = bytes
        .iter()
        .map(|x| new_ref(ValueData::Number(*x as f64)))
        .collect::<Vec<Value>>();
    let mut string = String::new();
    // Read file to string,if there are no errors then set 'contents' field
    match file.read_to_string(&mut string) {
        Ok(_) => file_object
            .borrow_mut()
            .set("contents", ValueData::String(string)),
        Err(_) => {}
    }
    file_object
        .borrow_mut()
        .set("bytes", ValueData::Array(new_ref(bytes)));

    Ok(new_ref(ValueData::Object(file_object)))
}

fn val_int(v: &Value) -> i64 {
    return i64::from(v.borrow().clone());
}

fn val_array(v: &Value) -> Ref<Vec<Value>> {
    let v: &ValueData = &v.borrow();
    match v {
        ValueData::Array(array) => return array.clone(),
        _ => return new_ref(vec![]),
    }
}

pub fn int_from_bytes(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    let array = val_array(&args[0]);
    let array = array.borrow();
    unsafe {
        if array.len() == 1 {
            let val = val_int(&array[0]);
            return Ok(new_ref(ValueData::Number(val as f64)));
        } else if array.len() == 2 {
            let v1 = val_int(&array[0]);
            let v2 = val_int(&array[1]);
            let val: u16 = std::mem::transmute([v1 as u8, v2 as u8]);
            return Ok(new_ref(ValueData::Number(val as _)));
        } else if array.len() == 4 {
            let v1 = val_int(&array[0]);
            let v2 = val_int(&array[1]);
            let v3 = val_int(&array[0]);
            let v4 = val_int(&array[1]);
            let val: u32 = std::mem::transmute([v1 as u8, v2 as u8, v3 as u8, v4 as u8]);
            return Ok(new_ref(ValueData::Number(val as _)));
        } else if array.len() == 8 {
            let v1 = val_int(&array[0]);
            let v2 = val_int(&array[1]);
            let v3 = val_int(&array[0]);
            let v4 = val_int(&array[1]);
            let v5 = val_int(&array[0]);
            let v6 = val_int(&array[1]);
            let v7 = val_int(&array[0]);
            let v8 = val_int(&array[1]);
            let val: u64 = std::mem::transmute([
                v1 as u8, v2 as u8, v3 as u8, v4 as u8, v5 as u8, v6 as u8, v7 as u8, v8 as u8,
            ]);
            return Ok(new_ref(ValueData::Number(val as _)));
        } else {
            return Ok(new_ref(ValueData::Undefined));
        }
    }
}

pub fn float_from_bits(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    let val = val_int(&args[0]) as u64;
    return Ok(new_ref(ValueData::Number(f64::from_bits(val))));
}

pub fn char_to_num(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    let val = args[0].clone();
    let val: &ValueData = &val.borrow();
    match val {
        ValueData::String(s) => {
            return Ok(new_ref(ValueData::Number(
                s.chars().nth(0).unwrap_or('\0') as u32 as f64,
            )))
        }
        _ => return Ok(nil()),
    }
}

pub fn str_from_utf8(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    let s = String::from_utf8(
        val_array(&args[0])
            .borrow()
            .iter()
            .map(|x| val_int(x) as u8)
            .collect(),
    );
    match s {
        Ok(val) => return Ok(new_ref(ValueData::String(val))),
        Err(e) => return Err(new_error(-1, None, &e.to_string())),
    }
}

pub fn register_builtins(env: Ref<Object>) {
    let err = new_object();
    let pos = &Position::new(0, 0);
    err.borrow_mut().set("__name__", "JLRuntimeError");
    let obj = new_ref(ValueData::Object(err));
    //gc_add_root(obj.gc());
    declare_var(&env, "JLRuntimeError", obj, &pos).unwrap();
    declare_var(&env, "instanceof", new_exfunc(builtin_instanceof), &pos).unwrap();
    declare_var(
        &env,
        "exports",
        new_ref(ValueData::Object(new_object())),
        &pos,
    )
    .unwrap();
    declare_var(&env, "require", new_exfunc(require), &pos).unwrap();
    declare_var(&env, "print", new_exfunc(builtin_print), &pos).unwrap();
    declare_var(&env, "gc", new_exfunc(builtin_gc), &pos).unwrap();
    declare_var(&env, "gc_stats", new_exfunc(enable_stats), &pos).unwrap();
    declare_var(&env, "spawn", new_exfunc(builtin_spawn), &pos).unwrap();
    declare_var(&env, "typeof", new_exfunc(type_of), &pos).unwrap();
    declare_var(
        &env,
        "Math",
        new_ref(ValueData::Object(math_object())),
        &pos,
    )
    .unwrap();
    declare_var(
        &env,
        "object_keys",
        new_exfunc(crate::vm::runtime::object::object_keys),
        &pos,
    )
    .unwrap();
    declare_var(&env, "len", new_exfunc(len), &pos).unwrap();
    declare_var(&env, "range", new_exfunc(range), &pos).unwrap();
    declare_var(&env, "file", new_exfunc(file), &pos).unwrap();
    declare_var(&env, "int_from_bytes", new_exfunc(int_from_bytes), &pos).unwrap();
    declare_var(&env, "float_from_bits", new_exfunc(float_from_bits), &pos).unwrap();

    let str_obj = new_object();
    str_obj
        .borrow_mut()
        .set("from_utf8", new_exfunc(str_from_utf8));

    declare_var(&env, "String", new_ref(ValueData::Object(str_obj)), &pos).unwrap();
    let obj = new_object();
    obj.borrow_mut().set(
        "create",
        new_exfunc(crate::vm::runtime::object::object_create),
    );

    declare_var(&env, "Object", new_ref(ValueData::Object(obj)), &pos).unwrap();

    declare_var(&env, "char_to_num", new_exfunc(char_to_num), &pos).unwrap();
}
