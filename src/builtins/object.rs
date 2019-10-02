use super::*;
use crate::*;

use value::*;

pub extern "C" fn to_string(this: Value, _: &[Value]) -> Result<Value, Value> {
    Ok(Value::String(Gc::new(this.to_string())))
}

pub extern "C" fn ctor(_: Value, args: &[Value]) -> Result<Value, Value> {
    let state = STATE.lock();
    let object: Gc<Object> = match state
        .static_variables
        .get(&Value::String(Gc::new("Object".to_owned())))
        .unwrap()
    {
        Value::Object(object) => object.clone(),
        _ => crate::unreachable(),
    };
    drop(state);
    if let Value::Object(proto) = &args[0] {
        return Ok(Value::Object(Gc::new(Object {
            proto: Some(proto.clone()),
            properties: Gc::new(vec![]),
            kind: ObjectKind::Ordinary,
        })));
    } else {
        return Ok(Value::Object(Gc::new(Object {
            proto: Some(object),
            properties: Gc::new(vec![]),
            kind: ObjectKind::Ordinary,
        })));
    }
}

pub fn object_proto() {
    let state = STATE.lock();
    let object: Gc<Object> = match state
        .static_variables
        .get(&Value::String(Gc::new("Object".to_owned())))
        .unwrap()
    {
        Value::Object(object) => object.clone(),
        _ => crate::unreachable(),
    };
    drop(state);
    let fun = new_builtin_fn(to_string as usize, 0);
    object
        .get_mut()
        .set_property(Value::String(Gc::new("toString".to_owned())), fun);
    object.get_mut().set_property(
        Value::String(Gc::new("constructor".to_owned())),
        new_builtin_fn(ctor as _, 1),
    );
}
