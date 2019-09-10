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

pub fn builtin_apply(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::Function(_) => {
            let array = match &args[2] {
                Value::Array(array) => array.borrow(),
                _ => {
                    return Err(Value::String(Ref(
                        "apply: Array of arguments expected".to_owned()
                    )))
                }
            };
            return val_callex(args[0].clone(), args[1].clone(), &*array);
        }
        _ => Err(Value::String(Ref("apply: Function expected".to_owned()))),
    }
}

pub fn builtin_array(args: &[Value]) -> Result<Value, Value> {
    Ok(Value::Array(Ref(args.to_vec())))
}

pub fn builtin_amake(args: &[Value]) -> Result<Value, Value> {
    let array = vec![Value::Null; args[0].to_int().unwrap_or(0) as usize];
    Ok(Value::Array(Ref(array)))
}

pub fn builtin_asize(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::Array(array) => return Ok(Value::Int(array.borrow().len() as _)),
        _ => return Err(Value::String(Ref("Array expected".to_owned()))),
    }
}

pub fn builtin_apush(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::Array(array) => array.borrow_mut().push(args[1].clone()),
        _ => return Err(Value::String(Ref("Array expected".to_owned()))),
    }
    Ok(Value::Null)
}
pub fn builtin_apop(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::Array(array) => return Ok(array.borrow_mut().pop().unwrap_or(Value::Null)),
        _ => return Err(Value::String(Ref("Array expected".to_owned()))),
    }
}

pub fn builtin_acopy(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::Array(array) => Ok(Value::Array(Ref(array
            .borrow()
            .iter()
            .map(|x| x.clone())
            .collect::<Vec<_>>()))),
        _ => return Err(Value::String(Ref("acopy: Array expected".to_owned()))),
    }
}

pub fn builtin_scopy(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::String(s) => Ok(Value::String(Ref(s.borrow().to_owned()))),
        _ => return Err(Value::String(Ref("scopy: String expected".to_owned()))),
    }
}

pub fn builtin_schars(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::String(s) => Ok(Value::Array(Ref(s
            .borrow()
            .chars()
            .map(|x| Value::Char(x))
            .collect()))),
        _ => return Err(Value::String(Ref("schars: String expected".to_owned()))),
    }
}

pub fn builtin_str_from_chars(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::Array(array) => {
            let mut chars = vec![];

            for ch in array.borrow().iter() {
                match ch {
                    Value::Char(x) => chars.push(*x),
                    _ => return Ok(Value::Null),
                }
            }
            use std::iter::FromIterator;
            let s = String::from_iter(chars.iter());
            return Ok(Value::String(Ref(s)));
        }
        Value::String(_) => return Ok(builtin_scopy(&[args[0].clone()])?),
        _ => return Ok(Value::Null),
    }
}

pub fn builtin_sget(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::String(s) => Ok(s
            .borrow()
            .chars()
            .nth(args[1].to_int().unwrap() as usize)
            .map(|x| Value::String(Ref(x.to_string())))
            .unwrap_or(Value::Null)),
        _ => return Err(Value::String(Ref("sget: String expected".to_owned()))),
    }
}

pub fn builtin_sfind(args: &[Value]) -> Result<Value, Value> {
    let pat = format!("{}", args[1]);
    match &args[0] {
        Value::String(s) => match s.borrow().find(&pat) {
            Some(result) => return Ok(Value::Int(result as _)),
            None => return Ok(Value::Null),
        },
        _ => return Err(Value::String(Ref("sfind: String expected".to_owned()))),
    }
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
        ValTag::Char => "char",
        ValTag::Func => "function",
        ValTag::User(x) => x,
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
        Some(lpath) if std::path::Path::new(&format!("{}/{}.j", lpath, path)).exists() => {
            format!("{}/{}.j", lpath, path)
        }
        Some(_) => path,
        None => path,
    };
    let path = if std::path::Path::new(&format!("{}.j", path)).exists() {
        format!("{}.j", path)
    } else {
        path
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

pub fn builtin_load_function(args: &[Value]) -> Result<Value, Value> {
    use libloading::{Library, Symbol};
    let lib = format!("{}", args[0]);
    let name = format!("{}", args[1]);
    let argc = args[2].to_int().unwrap();

    let lib = Library::new(&lib);
    match lib {
        Ok(lib) => {
            let lib: Library = lib;
            unsafe {
                let entry_point: Result<Symbol<fn()>, _> =
                    lib.get(format!("__jazzlight_entry_point\0").as_bytes());
                match entry_point {
                    Ok(sym) => {
                        sym();
                    }
                    Err(e) => {
                        return Err(Value::String(Ref(format!(
                            "Failed to get entry point: {}",
                            e
                        ))))
                    }
                }
                let symbol: Result<Symbol<fn(&[Value]) -> Result<Value, Value>>, _> =
                    lib.get(format!("{}\0", name).as_bytes());
                match symbol {
                    Ok(sym) => {
                        return Ok(new_native_fn(*sym, argc as _));
                    }
                    Err(e) => {
                        return Err(Value::String(Ref(format!(
                            "Symbol '{}' not found: {}",
                            name, e
                        ))))
                    }
                }
            }
        }
        Err(e) => return Err(Value::String(Ref(e.to_string()))),
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
    map.insert("apush".to_owned(), new_native_fn(builtin_apush, 2));
    map.insert("apop".to_owned(), new_native_fn(builtin_apop, 0));
    map.insert("acopy".to_owned(), new_native_fn(builtin_acopy, 1));
    map.insert("nargs".to_owned(), new_native_fn(builtin_nargs, 1));
    map.insert("typeof".to_owned(), new_native_fn(builtin_typeof, 1));
    map.insert("string".to_owned(), new_native_fn(builtin_string, 1));
    map.insert("load".to_owned(), new_native_fn(builtin_load, 1));
    map.insert(
        "load_function".to_owned(),
        new_native_fn(builtin_load_function, 3),
    );

    map.insert("scopy".to_owned(), new_native_fn(builtin_scopy, 1));
    map.insert("sfind".to_owned(), new_native_fn(builtin_sfind, 2));
    map.insert("sget".to_owned(), new_native_fn(builtin_sget, 2));
    map.insert("schars".to_owned(), new_native_fn(builtin_schars, 1));
    map.insert(
        "str_from_chars".to_owned(),
        new_native_fn(builtin_str_from_chars, 1),
    );
    map.insert("apply".to_owned(), new_native_fn(builtin_apply, 3));
    return map;
}
