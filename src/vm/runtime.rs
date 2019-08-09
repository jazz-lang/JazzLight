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

pub fn regex(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    match regex::Regex::new(&val_str(&args[0])) {
        Ok(regex) => Ok(new_ref(ValueData::Regex(new_ref(crate::vm::value::Regex(
            regex,
        ))))),
        Err(e) => return Err(new_error(-1, None, &e.to_string())),
    }
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
                    set,
                    get,
                    constants,
                    ..
                } => {
                    let func = Function::Regular {
                        environment: environment.clone(),
                        args: args.clone(),
                        code: code.clone(),
                        addr: *addr,
                        constants: constants.clone(),
                        yield_env: new_object(),
                        yield_pos: None,
                        set: *set,
                        get: *get,
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
        ValueData::Regex(_) => "regex",
    };
    Ok(new_ref(ValueData::String(name.to_owned())))
}

pub fn require(frame: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
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
    let mut file = File::open(&path).unwrap();
    let mut m = Machine::new();
    let mut code = vec![];

    file.read_to_end(&mut code).unwrap();
    let c = code.len();
    let mut reader = crate::decoder::BytecodeReader {
        machine: &mut m,
        bytecode: std::io::Cursor::new(code),
        pc: 0,
        count: c
    };

    let code = reader.read();
    let mut frame1 = Frame::new(&mut m);
    frame1.code = new_ref(code);
    crate::vm::runtime::register_builtins(frame1.env.clone());
    frame1.execute();
    /*let mut ids = std::collections::HashMap::new();
    for (i,c) in frame1.m.constants.iter().enumerate() {
        let new_id = frame.m.constants.len();
        ids.insert(i,new_id);
        frame.m.constants.borrow_mut().push(c.clone());
    }
    for opcode in frame1.code.borrow_mut().iter_mut() {
        match opcode {
            Opcode::LoadConst(id) => {
                let new_id = *ids.get(&(*id as usize)).unwrap();
                *id = new_id as u32;
            }
            _ => ()
        }
    }*/

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
        .set("name", ValueData::String(file_name.clone()))?;

    if !std::path::Path::new(&file_name).exists() {
        match std::fs::File::create(&file_name) {
            Ok(_) => (),
            Err(e) => return Err(new_error(-1, None, &e.to_string())),
        }
    }
    let file = File::open(&file_name);
    let mut file = match file {
        Ok(file) => file,
        Err(e) => return Err(new_error(-1, None, &e.to_string())),
    };
    file_object
        .borrow_mut()
        .set("path", ValueData::String(file_name.clone()))?;

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
            .set("contents", ValueData::String(string))?,
        Err(_) => {}
    }
    file_object
        .borrow_mut()
        .set("bytes", ValueData::Array(new_ref(bytes)))?;

    fn write(_: &mut Frame<'_>, this: Value, args: &[Value]) -> Result<Value, ValueData> {
        let this: &ValueData = &this.borrow();
        match this {
            ValueData::Object(object) => {
                let path = val_str(
                    &object
                        .borrow()
                        .get(&ValueData::String("path".to_owned()))
                        .unwrap_or(Property::new("", nil()))
                        .value,
                );
                let contents = String::from(args[0].borrow().clone());
                if !std::path::Path::new(&path).exists() {
                    match std::fs::File::create(&path) {
                        Ok(_) => (),
                        Err(e) => return Err(new_error(-1, None, &e.to_string())),
                    }
                }
                let file = std::fs::OpenOptions::new().write(true).open(&path);
                match file {
                    Ok(mut file) => {
                        let bytes = contents.into_bytes();

                        match file.write_all(&bytes) {
                            Ok(_) => return Ok(nil()),
                            Err(e) => return Err(new_error(-1, None, &e.to_string())),
                        }
                    }
                    Err(e) => return Err(new_error(-1, None, &e.to_string())),
                }
            }
            _ => return Err(new_error(-1, None, "File object expected")),
        }
    }

    fn clear(_: &mut Frame<'_>, this: Value, _: &[Value]) -> Result<Value, ValueData> {
        let this: &ValueData = &this.borrow();
        match this {
            ValueData::Object(object) => {
                let path = val_str(
                    &object
                        .borrow()
                        .get(&ValueData::String("path".to_owned()))
                        .unwrap_or(Property::new("", nil()))
                        .value,
                );

                if !std::path::Path::new(&path).exists() {
                    return Ok(nil());
                }
                let file = std::fs::OpenOptions::new().write(true).open(&path);
                match file {
                    Ok(file) => {
                        match file.set_len(0) {
                            Ok(_) => (),
                            Err(e) => return Err(new_error(-1, None, &e.to_string())),
                        }
                        object
                            .borrow_mut()
                            .set("bytes", new_ref(ValueData::Array(new_ref(vec![]))))
                            .unwrap();
                        object
                            .borrow_mut()
                            .set("contents", new_ref(ValueData::String("".to_owned())))
                            .unwrap();
                        return Ok(nil());
                    }
                    Err(e) => return Err(new_error(-1, None, &e.to_string())),
                }
            }
            _ => return Err(new_error(-1, None, "File object expected")),
        }
    }

    fn write_bytes(_: &mut Frame<'_>, this: Value, args: &[Value]) -> Result<Value, ValueData> {
        let this: &ValueData = &this.borrow();
        match this {
            ValueData::Object(object) => {
                let path = val_str(
                    &object
                        .borrow()
                        .get(&ValueData::String("path".to_owned()))
                        .unwrap_or(Property::new("", nil()))
                        .value,
                );
                let contents = args[0].borrow().clone();
                let bytes = match contents {
                    ValueData::String(s) => s.into_bytes(),
                    ValueData::Array(array) => {
                        array.borrow().iter().map(|x| val_int(x) as u8).collect()
                    }
                    _ => return Err(new_error(-1, None, "File.write: string or array expected")),
                };
                if !std::path::Path::new(&path).exists() {
                    match std::fs::File::create(&path) {
                        Ok(_) => (),
                        Err(e) => return Err(new_error(-1, None, &e.to_string())),
                    }
                }
                let file = std::fs::OpenOptions::new().write(true).open(&path);
                match file {
                    Ok(mut file) => match file.write_all(&bytes) {
                        Ok(_) => return Ok(nil()),
                        Err(e) => return Err(new_error(-1, None, &e.to_string())),
                    },
                    Err(e) => return Err(new_error(-1, None, &e.to_string())),
                }
            }
            _ => return Err(new_error(-1, None, "File object expected")),
        }
    }

    file_object
        .borrow_mut()
        .set("write_string", new_exfunc(write))
        .unwrap();
    file_object
        .borrow_mut()
        .set("write_bytes", new_exfunc(write_bytes))
        .unwrap();
    file_object
        .borrow_mut()
        .set("clear", new_exfunc(clear))
        .unwrap();

    Ok(new_ref(ValueData::Object(file_object)))
}

fn val_int(v: &Value) -> i64 {
    return i64::from(v.borrow().clone());
}

fn val_str(v: &Value) -> String {
    let v: &ValueData = &v.borrow();
    match v {
        ValueData::String(s) => return s.clone(),
        _ => String::new(),
    }
}

pub fn str_trim(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    let val = val_str(&args[0]);
    return Ok(new_ref(ValueData::String(val.trim().to_owned())));
}

pub fn str_split(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    let val = val_str(&args[0]);
    let val2 = val_str(&args[1]);
    let s = val
        .split(val2.chars().nth(0).unwrap_or(' '))
        .collect::<Vec<_>>();
    let val = new_ref(
        s.iter()
            .map(|x| new_ref(ValueData::String(x.to_string())))
            .collect::<Vec<_>>(),
    );
    return Ok(new_ref(ValueData::Array(val)));
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
pub fn str_chars(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    return Ok(new_ref(ValueData::Array(new_ref(
        val_str(&args[0])
            .chars()
            .map(|x| new_ref(ValueData::String(x.to_string())))
            .collect(),
    ))));
}

pub fn new_object_f(f: &mut Frame<'_>,t: Value,args: &[Value]) -> Result<Value,ValueData> {
    crate::vm::runtime::object::object_create(f,t,args)
}

pub fn register_builtins(env: Ref<Object>) {
    let err = new_object();
    let pos = &Position::new(0, 0);
    err.borrow_mut().set("__name__", "JLRuntimeError").unwrap();
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
    declare_var(&env, "File", new_exfunc(file), &pos).unwrap();
    declare_var(&env, "int_from_bytes", new_exfunc(int_from_bytes), &pos).unwrap();
    declare_var(&env, "float_from_bits", new_exfunc(float_from_bits), &pos).unwrap();

    let str_obj = new_object();
    str_obj
        .borrow_mut()
        .set("from_utf8", new_exfunc(str_from_utf8))
        .unwrap();

    declare_var(&env, "String", new_ref(ValueData::Object(str_obj)), &pos).unwrap();
    let obj = new_object();
    obj.borrow_mut()
        .set(
            "create",
            new_exfunc(crate::vm::runtime::object::object_create),
        )
        .unwrap();

    declare_var(&env, "Object", new_ref(ValueData::Object(obj)), &pos).unwrap();

    declare_var(&env, "char_to_num", new_exfunc(char_to_num), &pos).unwrap();
    declare_var(&env, "str_split", new_exfunc(str_split), &pos).unwrap();
    declare_var(&env, "str_trim", new_exfunc(str_trim), &pos).unwrap();
    declare_var(&env, "str_chars", new_exfunc(str_chars), &pos).unwrap();
    declare_var(&env, "json", json().unwrap(), &pos).unwrap();
    declare_var(&env, "regex", new_exfunc(regex), &pos).unwrap();
    declare_var(&env, "parseInt", new_exfunc(parse_int), &pos).unwrap();
    declare_var(&env, "parseFloat", new_exfunc(parse_float), &pos).unwrap();
    declare_var(&env, "new_object", new_exfunc(new_object_f), &pos).unwrap();
}

pub fn regex_is_match(_: &mut Frame<'_>, this: Value, args: &[Value]) -> Result<Value, ValueData> {
    let text = val_str(&args[0]);
    let val: &ValueData = &this.borrow();
    match val {
        ValueData::Regex(regex) => {
            return Ok(new_ref(ValueData::Bool(regex.borrow().is_match(&text))))
        }
        _ => return Ok(nil()),
    }
}
pub fn regex_find(_: &mut Frame<'_>, this: Value, args: &[Value]) -> Result<Value, ValueData> {
    let text = val_str(&args[0]);
    let val: &ValueData = &this.borrow();
    match val {
        ValueData::Regex(regex) => {
            let match_: Option<regex::Match> = regex.borrow().find(&text);
            match match_ {
                Some(match_) => {
                    let obj = new_object();

                    obj.borrow_mut()
                        .set("start", new_ref(ValueData::Number(match_.start() as f64)))
                        .unwrap();
                    obj.borrow_mut()
                        .set("end", new_ref(ValueData::Number(match_.end() as f64)))
                        .unwrap();
                    obj.borrow_mut()
                        .set(
                            "text",
                            new_ref(ValueData::String(match_.as_str().to_string())),
                        )
                        .unwrap();
                    return Ok(new_ref(ValueData::Object(obj)));
                }
                None => return Ok(nil()),
            }
        }
        _ => return Ok(nil()),
    }
}

pub fn parse_int(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    let text = val_str(&args[0]);
    match text.parse::<i64>() {
        Ok(num) => return Ok(new_ref(ValueData::Number(num as f64))),
        Err(e) => return Err(new_error(-1, None, &e.to_string())),
    }
}
pub fn parse_float(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    let text = val_str(&args[0]);
    match text.parse::<f64>() {
        Ok(num) => return Ok(new_ref(ValueData::Number(num))),
        Err(e) => return Err(new_error(-1, None, &format!("Failed to parse '{}': {}",text,e.to_string()))),
    }
}

pub fn str_slice(_: &mut Frame<'_>,this: Value,args: &[Value]) -> Result<Value,ValueData> {
    let s = val_str(&this);
    let start = val_int(&args[0]);
    let end = val_int(&args[1]);
    return Ok(new_ref(
        ValueData::String(s[start as usize..end as usize].to_string())
    ));
}

pub fn regex_captures(_: &mut Frame<'_>, this: Value, args: &[Value]) -> Result<Value, ValueData> {
    let val: &ValueData = &this.borrow();
    match val {
        ValueData::Regex(regex) => {
            let regex: &regex::Regex = &regex.borrow();
            let text = val_str(&args[0]);
            let captures = regex.captures(&text);
            match captures {
                Some(captures) => {
                    return Ok(new_ref(ValueData::Array(new_ref(
                        captures
                            .iter()
                            .map(|x| match x {
                                Some(match_) => {
                                    let obj = new_object();

                                    obj.borrow_mut()
                                        .set(
                                            "start",
                                            new_ref(ValueData::Number(match_.start() as f64)),
                                        )
                                        .unwrap();
                                    obj.borrow_mut()
                                        .set("end", new_ref(ValueData::Number(match_.end() as f64)))
                                        .unwrap();
                                    obj.borrow_mut()
                                        .set(
                                            "text",
                                            new_ref(ValueData::String(match_.as_str().to_string())),
                                        )
                                        .unwrap();
                                    new_ref(ValueData::Object(obj))
                                }
                                None => nil(),
                            })
                            .collect(),
                    ))))
                }
                None => return Ok(nil()),
            }
        }
        _ => return Ok(nil()),
    }
}

pub fn json() -> Result<Value, ValueData> {
    fn deserialize(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
        let val = val_str(&args[0]);

        let value: Value = match serde_json::from_str(&val) {
            Ok(v) => v,
            Err(e) => return Err(new_error(-1, None, &e.to_string())),
        };

        return Ok(value);
    }

    fn serialize(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
        let string = serde_json::to_string(&args[0]);
        match string {
            Ok(string) => Ok(new_ref(ValueData::String(string))),
            Err(e) => Err(new_error(-1, None, &e.to_string())),
        }
    }

    let json = new_object();
    json.borrow_mut()
        .set("serialize", new_exfunc(serialize))
        .unwrap();
    json.borrow_mut()
        .set("deserialize", new_exfunc(deserialize))
        .unwrap();
    return Ok(new_ref(ValueData::Object(json)));
}
