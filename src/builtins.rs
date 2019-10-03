pub mod array;
pub mod common;
pub mod function;
pub mod math;
pub mod number;
pub mod object;

use crate::value::*;
use crate::*;

pub fn new_builtin_fn(f: usize, argc: i32) -> Value {
    let state = STATE.lock();
    let object = state
        .static_variables
        .get(&Value::String(Gc::new("Function".to_owned())))
        .cloned()
        .unwrap();
    let object_proto = state
        .static_variables
        .get(&Value::String(Gc::new("Object".to_owned())))
        .cloned()
        .unwrap();
    let object_proto = match object_proto {
        Value::Object(object) => object,
        _ => crate::unreachable(),
    };
    let object = match object {
        Value::Object(object) => object,
        _ => crate::unreachable(),
    };
    let fun = Gc::new(Function {
        module: None,
        addr: f,
        is_native: true,
        argc,
        env: Value::Null,
        prototype: Value::Object(object_proto),
    });
    let func = ObjectKind::Function(fun);
    let function = Value::Object(Gc::new(Object {
        proto: Some(object),
        kind: func,
        properties: Gc::new(vec![]),
    }));

    function
}

pub fn new_func(fun: Gc<Function>, argc: i32) -> Value {
    let state = STATE.lock();
    let object = state
        .static_variables
        .get(&Value::String(Gc::new("Function".to_owned())))
        .cloned()
        .unwrap();
    let object_proto = state
        .static_variables
        .get(&Value::String(Gc::new("Object".to_owned())))
        .cloned()
        .unwrap();
    let object_proto = match object_proto {
        Value::Object(object) => object,
        _ => crate::unreachable(),
    };
    let object = match object {
        Value::Object(object) => object,
        _ => crate::unreachable(),
    };

    fun.get_mut().prototype = Value::Object(object_proto);
    fun.get_mut().env = Value::Object(Gc::new(Object {
        kind: ObjectKind::Array(Gc::new(vec![])),
        properties: Gc::new(vec![]),
        proto: None,
    }));
    fun.get_mut().argc = argc;

    let func = ObjectKind::Function(fun);
    let function = Value::Object(Gc::new(Object {
        proto: Some(object),
        kind: func,
        properties: Gc::new(vec![]),
    }));

    function
}

pub extern "C" fn println(_: Value, args: &[Value]) -> Result<Value, Value> {
    for arg in args.iter() {
        print!("{}", arg);
    }
    println!();
    Ok(Value::Null)
}

pub fn builtin_fns() {
    let println = new_builtin_fn(println as _, -1);
    let mut state = STATE.lock();
    state
        .static_variables
        .insert(Value::String(Gc::new("println".to_owned())), println);
}
