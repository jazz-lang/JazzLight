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

pub extern "C" fn math_random(_: Value, _: &[Value]) -> Result<Value, Value> {
    Ok(Value::Number(rand::random()))
}



define_global!(
    MathObject "Math"; {
        function random() {
            Ok(Value::Number(rand::random()))
        };


        function atan (x) {
            Ok(Value::Number(x.to_number().atan()))
        };
        function atanh(x) {
            Ok(Value::Number(x.to_number().atanh()))
        };

        E = Value::Number(std::f64::consts::E);
        PI = Value::Number(std::f64::consts::PI);
        LN2 = Value::Number(std::f64::consts::LN_2);
        LN10 = Value::Number(std::f64::consts::LN_10);
        SQRT2 = Value::Number(std::f64::consts::SQRT_2);


        function abs(x) {
            Ok(Value::Number(x.to_number().abs()))
        };
        function acos(x) {
            Ok(Value::Number(x.to_number().acos()))
        };
        function acosh(x) {
            Ok(Value::Number(x.to_number().acosh()))
        };
        function asin(x) {
            Ok(Value::Number(x.to_number().asin()))
        };
        function atan2(x,y) {
            Ok(Value::Number(x.to_number().atan2(y.to_number())))
        };
        function cbrt(x) {
            Ok(Value::Number(x.to_number().cbrt()))
        };

        function pow(x,y) {
            let x: f64 = x.to_number();
            let y: f64 = y.to_number();

            if x.is_nan() || y.is_nan() || x.is_infinite() || y.is_infinite() {
                return Ok(Value::Null);
            }

            return Ok(Value::Number(x.powf(y)));
        };
        function ceil(x) {
            Ok(Value::Number(x.to_number().ceil()))
        };
        function cos(x) {
            Ok(Value::Number(x.to_number().cos()))
        };
        function cosh(x) {
            Ok(Value::Number(x.to_number().cosh()))
        };
        function exp(x) {
            Ok(Value::Number(x.to_number().exp()))
        };
        function imul(x,y) {
            let x = x.to_number() as i32;
            let y = y.to_number() as i32;
            let res = x.wrapping_mul(y);  
            Ok(Value::Number(res as _))
        };
        function floor(x) {
            Ok(Value::Number(x.to_number().floor()))
        };
        function round(x) {
            Ok(Value::Number(x.to_number().round()))
        };
        function sin(x) {
            Ok(Value::Number(x.to_number().sin()))
        };
        function sinh(x) {
            Ok(Value::Number(x.to_number().sinh()))
        };
        function sqrt(x) {
            Ok(Value::Number(x.to_number().sqrt()))
        };
        function tan(x) {
            Ok(Value::Number(x.to_number().tan()))
        };
        function tanh(x) {
            Ok(Value::Number(x.to_number().tanh()))
        };
        function trunc(x) {
            Ok(Value::Number(x.to_number().trunc()))
        };


    } 
);


pub fn math_object() {
    MathObject::init();
    /*let object = Gc::new(Object {
        proto: None,
        properties: Gc::new(vec![]),
        kind: ObjectKind::Ordinary,
    });

    object.get().set_property(
        Value::String(Gc::new("pow".to_owned())),
        new_builtin_fn(math_pow as _, 2),
    );

    object.get().set_property(
        Value::String(Gc::new("random".to_owned())),
        new_builtin_fn(math_random as _, 0),
    );
    object.get().set_property(
        Value::String(Gc::new("PI".to_owned())),
        Value::Number(std::f64::consts::PI),
    );

    let mut state = STATE.lock();

    state.static_variables.insert(
        Value::String(Gc::new("Math".to_owned())),
        Value::Object(object),
    );*/
}
