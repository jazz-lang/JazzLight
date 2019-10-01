use super::*;
use crate::*;
use pgc::*;
use value::*;

pub fn to_string(this: Value, _: &[Value]) -> Result<Value, Value> {
    Ok(Value::String(Gc::new(this.to_string())))
}

pub fn ctor(_: Value, args: &[Value]) -> Result<Value, Value> {
    println!("ctor");
    let state = STATE.lock();
    let object: Gc<Object> = match state
        .get()
        .static_variables
        .get(&Value::String(Rooted::new("Object".to_owned()).inner()))
        .unwrap()
    {
        Value::Object(object) => object.clone(),
        _ => crate::unreachable(),
    };
    drop(state);
    if let Value::Object(proto) = args[0] {
        return Ok(Value::Object(
            Rooted::new(Object {
                proto: Some(proto),
                properties: Rooted::new(vec![]).inner(),
                kind: ObjectKind::Ordinary,
            })
            .inner(),
        ));
    } else {
        return Ok(Value::Object(
            Rooted::new(Object {
                proto: Some(object),
                properties: Rooted::new(vec![]).inner(),
                kind: ObjectKind::Ordinary,
            })
            .inner(),
        ));
    }
}

pub fn object_proto() {
    let state = STATE.lock();
    let object: Gc<Object> = match state
        .get()
        .static_variables
        .get(&Value::String(Rooted::new("Object".to_owned()).inner()))
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
