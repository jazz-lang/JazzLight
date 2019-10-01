use super::*;
use crate::*;
use pgc::*;
use value::*;

pub fn to_string(this: Value, _: &[Value]) -> Result<Value, Value> {
    Ok(Value::String(Gc::new(this.to_string())))
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
        _ => unreachable!(),
    };
    drop(state);
    let fun = new_builtin_fn(to_string as usize, 0);
    object
        .get_mut()
        .set_property(Value::String(Gc::new("toString".to_owned())), fun);
}
