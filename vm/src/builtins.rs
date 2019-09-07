use crate::value::*;
use crate::*;
use std::collections::HashMap;

thread_local! {
    pub static BUILTINS: HashMap<String,Value> = builtins_init();
}


pub fn get_builtin(field: &str) -> Option<Value> {
    BUILTINS.with(|builtins| builtins.get(field).cloned())
}

pub fn builtin_print(args: &[Value]) -> Result<Value,Value> {
    for val in args.iter() {
        print!("{}",val);
    }
    Ok(Value::Null)
}

pub fn builtin_array(args: &[Value]) -> Result<Value,Value> {
    Ok(Value::Array(Ref(args.to_vec())))
}

pub fn builtin_amake(args: &[Value]) -> Result<Value,Value> {
    let array = vec![Value::Null;args[0].to_int().unwrap_or(0) as usize];
    Ok(Value::Array(Ref(array)))
}

pub fn builtin_string(args: &[Value]) -> Result<Value,Value> {
    let value = args[0].to_string();
    return Ok(Value::String(Ref(value)));
}


fn new_native_fn(x: fn(&[Value]) -> Result<Value,Value>,argc: i32) -> Value {
    Value::Function(
        Ref(
            Function {
                native: true,
                address: x as usize,
                env: Value::Null,
                module: None,
                argc
            }
        )
    )
}

pub fn builtins_init() -> HashMap<String,Value> {
    let mut map = HashMap::new();

    map.insert("print".to_owned(),new_native_fn(builtin_print,-1));
    map.insert("array".to_owned(),new_native_fn(builtin_array,-1));
    map.insert("amake".to_owned(),new_native_fn(builtin_amake,1));
    map.insert("string".to_owned(),new_native_fn(builtin_string,1));
    return map;
}