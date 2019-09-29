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
    let object = match object {
        Value::Object(object) => object,
        _ => unreachable!(),
    };
    let fun = Rooted::new(Function {
        module: None,
        addr: f,
        is_native: true,
        argc,
        env: Value::Null,
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
