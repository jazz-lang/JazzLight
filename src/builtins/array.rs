use super::*;
use crate::*;

use value::*;

pub extern "C" fn pop(this: Value, _: &[Value]) -> Result<Value, Value> {
    match this {
        Value::Object(object) => match &object.get().kind {
            ObjectKind::Array(array) => return Ok(array.get_mut().pop().unwrap_or(Value::Null)),
            _ => Err(Value::Null),
        },
        _ => Err(Value::Null),
    }
}

pub extern "C" fn ctor(_: Value, args: &[Value]) -> Result<Value, Value> {
    let array = args.to_vec();
    let state = STATE.lock();
    let proto = state
        .static_variables
        .get(&Value::String(Gc::new("Array".to_owned())))
        .unwrap()
        .unwrap_object();

    Ok(Value::Object(Gc::new(Object {
        kind: ObjectKind::Array(Gc::new(array)),
        properties: Gc::new(vec![]),
        proto: Some(proto),
    })))
}

pub fn array_object() {
    let object = Gc::new(Object {
        kind: ObjectKind::Ordinary,
        proto: None,
        properties: Gc::new(vec![]),
    });
    let pop_ = Gc::new("pop".to_owned());
    object
        .get_mut()
        .set_property(Value::String(pop_), new_builtin_fn(pop as usize, 0));
    object.get_mut().set_property(
        Value::String(Gc::new("constructor".to_owned())),
        new_builtin_fn(ctor as usize, -1),
    );
    let mut state = STATE.lock();
    state.static_variables.insert(
        Value::String(Gc::new("Array".to_owned())),
        Value::Object(object),
    );
}
