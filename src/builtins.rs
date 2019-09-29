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
        .get(&Value::String(Gc::new("Function".to_owned())))
        .cloned()
        .unwrap();
    let object = match object {
        Value::Object(object) => object,
        _ => unreachable!(),
    };
    let function = Value::Object(Gc::new(Object {
        proto: Some(object),
        kind: ObjectKind::Function(
            Rooted::new(Function {
                module: None,
                addr: f,
                is_native: true,
                argc,
                env: Value::Null,
            })
            .inner(),
        ),
        properties: Rooted::new(vec![]).inner(),
    }));

    function
}
