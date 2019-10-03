use super::*;
use crate::*;
use value::*;

pub extern "C" fn math_pow(_: Value, args: &[Value]) -> Result<Value, Value> {
    let x: f64 = args[0].to_number();
    let y: f64 = args[1].to_number();

    if x.is_nan() || y.is_nan() || x.is_infinite() || y.is_infinite() {
        return Ok(Value::Null);
    }

    return Ok(Value::Number(x.powf(y)));
}

pub fn math_object() {
    let object = Gc::new(Object {
        proto: None,
        properties: Gc::new(vec![]),
        kind: ObjectKind::Ordinary,
    });

    object.get().set_property(
        Value::String(Gc::new("pow".to_owned())),
        new_builtin_fn(math_pow as _, 2),
    );

    let mut state = STATE.lock();

    state.static_variables.insert(
        Value::String(Gc::new("Math".to_owned())),
        Value::Object(object),
    );
}
