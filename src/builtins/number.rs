use super::*;
use crate::*;
use value::*;

pub extern "C" fn num_round(this: Value, _: &[Value]) -> Result<Value, Value> {
    let number = this.to_number();
    Ok(Value::Number(number.round()))
}

pub extern "C" fn num_floor(this: Value, _: &[Value]) -> Result<Value, Value> {
    let number = this.to_number();
    Ok(Value::Number(number.floor()))
}

pub extern "C" fn num_is_inf(this: Value, _: &[Value]) -> Result<Value, Value> {
    let number = this.to_number();
    Ok(Value::Bool(number.is_infinite()))
}
pub fn number_object() {
    let object = Gc::new(Object {
        proto: None,
        properties: Gc::new(vec![]),
        kind: ObjectKind::Ordinary,
    });

    object.get().set_property(
        Value::String(Gc::new("floor".to_owned())),
        new_builtin_fn(num_floor as _, 0),
    );
    object.get().set_property(
        Value::String(Gc::new("round".to_owned())),
        new_builtin_fn(num_round as _, 0),
    );
    object.get().set_property(
        Value::String(Gc::new("is_infinite".to_owned())),
        new_builtin_fn(num_is_inf as _, 0),
    );
    let mut state = STATE.lock();

    state.static_variables.insert(
        Value::String(Gc::new("Number".to_owned())),
        Value::Object(object),
    );
}
