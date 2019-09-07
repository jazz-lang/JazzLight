use crate::interp::*;
use crate::value::*;
use crate::*;
use std::collections::HashMap;

thread_local! {
    pub static BUILTINS: HashMap<String,Value> = builtins_init();
}

pub fn get_builtin(field: &str) -> Option<Value> {
    BUILTINS.with(|builtins| builtins.get(field).cloned())
}

pub fn builtin_print(args: &[Value]) -> Result<Value, Value> {
    for val in args.iter() {
        print!("{}", val);
    }
    Ok(Value::Null)
}

pub fn builtin_array(args: &[Value]) -> Result<Value, Value> {
    Ok(Value::Array(Ref(args.to_vec())))
}

pub fn builtin_amake(args: &[Value]) -> Result<Value, Value> {
    let array = vec![Value::Null; args[0].to_int().unwrap_or(0) as usize];
    Ok(Value::Array(Ref(array)))
}

pub fn builtin_asize(args: &[Value]) -> Result<Value, Value> {
    let array = args[0].clone();
    match array {
        Value::Array(array) => return Ok(Value::Int(array.borrow().len() as _)),
        _ => return Err(Value::String(Ref("Array expected".to_owned()))),
    }
}

pub fn builtin_apush(args: &[Value]) -> Result<Value, Value> {
    let array = args[0].clone();
    match array {
        Value::Array(array) => array.borrow_mut().push(args[1].clone()),
        _ => return Err(Value::String(Ref("Array expected".to_owned()))),
    }
    Ok(Value::Null)
}

pub fn builtin_string(args: &[Value]) -> Result<Value, Value> {
    let value = args[0].to_string();
    return Ok(Value::String(Ref(value)));
}
pub fn builtin_typeof(args: &[Value]) -> Result<Value, Value> {
    let tag = args[0].tag();
    Ok(Value::String(Ref(match tag {
        ValTag::Array => "array",
        ValTag::Null => "null",
        ValTag::Float => "float",
        ValTag::Int => "int",
        ValTag::Str => "string",
        ValTag::Bool => "bool",
        ValTag::Object => "object",
        ValTag::Func => "function",
    }
    .to_owned())))
}

pub fn builtin_nargs(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::Function(fun) => Ok(Value::Int(fun.borrow().argc as _)),
        _ => Ok(Value::Null),
    }
}
pub fn builtin_load(args: &[Value]) -> Result<Value, Value> {
    let path = args[0].to_string();

    let libs_path: Option<&'static str> = option_env!("JAZZLIGHT_PATH");
    let path = match libs_path {
        Some(lpath) if std::path::Path::new(&format!("{}/{}", lpath, path)).exists() => {
            format!("{}/{}", lpath, path)
        }
        Some(_) => path,
        None => path,
    };
    let contents = std::fs::read(&path);
    match contents {
        Ok(contents) => {
            use crate::reader::BytecodeReader;
            let mut r = BytecodeReader {
                bytes: std::io::Cursor::new(&contents),
            };

            let m = r.read_module();

            let mut vm = Vm::new();
            vm.save_state_exit();
            vm.interp(m.clone());

            return Ok(m.borrow().exports.clone());
        }
        Err(e) => {
            return Err(Value::String(Ref(format!(
                "load: failed to load module at '{}': {}",
                path, e
            ))))
        }
    }
}

fn new_native_fn(x: fn(&[Value]) -> Result<Value, Value>, argc: i32) -> Value {
    Value::Function(Ref(Function {
        native: true,
        address: x as usize,
        env: Value::Null,
        module: None,
        argc,
    }))
}

pub fn builtins_init() -> HashMap<String, Value> {
    let mut map = HashMap::new();

    map.insert("print".to_owned(), new_native_fn(builtin_print, -1));
    map.insert("array".to_owned(), new_native_fn(builtin_array, -1));
    map.insert("amake".to_owned(), new_native_fn(builtin_amake, 1));
    map.insert("asize".to_owned(), new_native_fn(builtin_asize, 1));
    map.insert("apush".to_owned(), new_native_fn(builtin_apush, 1));
    map.insert("nargs".to_owned(), new_native_fn(builtin_nargs, 1));
    map.insert("typeof".to_owned(), new_native_fn(builtin_typeof, 1));
    map.insert("string".to_owned(), new_native_fn(builtin_string, 1));
    map.insert("load".to_owned(), new_native_fn(builtin_load, 1));
    return map;
}
