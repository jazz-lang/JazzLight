use super::*;
use crate::interpreter::*;
use crate::*;

use value::*;

pub extern "C" fn apply(this: Value, args: &[Value]) -> Result<Value, Value> {
    let this_val = args[0].clone();
    match &args[1] {
        Value::Object(object) => match &object.get().kind {
            ObjectKind::Array(args) => {
                return call_value(this, this_val, &args.get());
            }
            _ => return Ok(Value::Null),
        },
        _ => return Ok(Value::Null),
    }
}

pub fn function_object() {
    {
        let object = Gc::new(Object {
            kind: ObjectKind::Ordinary,
            proto: None,
            properties: Gc::new(vec![]),
        });
        let mut state = STATE.lock();

        state.static_variables.insert(
            Value::String(Gc::new("Function".to_owned())),
            Value::Object(object),
        );
    }
    function_proto_reg_fns();
}

fn function_proto_reg_fns() {
    let object = {
        let state = STATE.lock();
        state
            .static_variables
            .get(&Value::String(Gc::new("Function".to_owned())))
            .cloned()
            .unwrap()
    };
    match object {
        Value::Object(object) => {
            object.get_mut().set_property(
                Value::String(Gc::new("apply".to_owned())),
                new_builtin_fn(apply as usize, 2),
            );
        }
        _ => crate::unreachable(),
    }
}
