pub mod array;
pub mod function;
pub mod object;

use crate::value::*;
use crate::*;
use pgc::*;

pub fn new_builtin_fn(f: usize, argc: i32) -> Value {
    let state = STATE.lock();
    let object = state
        .get()
        .static_variables
        .get(&Value::String(Rooted::new("Function".to_owned()).inner()))
        .cloned()
        .unwrap();
    let object_proto = state
        .get()
        .static_variables
        .get(&Value::String(Rooted::new("Object".to_owned()).inner()))
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
    let fun = Rooted::new(Function {
        module: None,
        addr: f,
        is_native: true,
        argc,
        env: Value::Null,
        prototype: Value::Object(object_proto),
    });
    let func = ObjectKind::Function(fun.inner());
    let function = Value::Object(
        Rooted::new(Object {
            proto: Some(object),
            kind: func,
            properties: Rooted::new(vec![]).inner(),
        })
        .inner(),
    );

    function
}

pub fn new_func(fun: Gc<Function>, argc: i32) -> Value {
    let state = STATE.lock();
    let object = state
        .get()
        .static_variables
        .get(&Value::String(Rooted::new("Function".to_owned()).inner()))
        .cloned()
        .unwrap();
    let object_proto = state
        .get()
        .static_variables
        .get(&Value::String(Rooted::new("Object".to_owned()).inner()))
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
    fun.get_mut().env = Value::Null;
    fun.get_mut().argc = argc;

    let func = ObjectKind::Function(fun);
    let function = Value::Object(
        Rooted::new(Object {
            proto: Some(object),
            kind: func,
            properties: Rooted::new(vec![]).inner(),
        })
        .inner(),
    );

    function
}
