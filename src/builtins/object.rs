use super::*;
use crate::*;
use pgc::*;
use value::*;

pub fn to_string(this: Value, _: &[Value]) -> Result<Value, Value> {
    Ok(Value::String(Gc::new(this.to_string())))
}

pub fn object_proto() {
    // we need object to be rooted since it may be deleted by GC.
    let object = Rooted::new(Object {
        kind: ObjectKind::Ordinary,
        proto: None,
        properties: Gc::new(vec![]),
    });
    let fun = new_builtin_fn(to_string as usize, 0);
    object
        .get_mut()
        .set_property(Value::String(Gc::new("toString".to_owned())), fun);
    let state = STATE.lock();

    state.get_mut().static_variables.insert(
        Value::String(Rooted::new("Object".to_owned()).inner()),
        Value::Object(object.inner()), // now 'Array' object unrooted,but since global state is rooted it's fine.
    );
}
