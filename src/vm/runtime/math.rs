//use crate::vm::runtime::decl_fun;
use crate::vm::runtime::new_exfunc;
use crate::vm::value::*;
use crate::vm::{nil, Frame};

pub fn math_floor(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    if args.is_empty() {
        return Err(new_error(-1, None, "Math.floor: expected 1 argument"));
    }
    let val = args[0].clone();
    let val: &ValueData = &val.borrow();
    let floating = f64::from(val.clone());
    Ok(new_ref(ValueData::Number(floating.floor())))
}

pub fn math_tanh(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    if args.is_empty() {
        return Err(new_error(-1, None, "Math.floor: expected 1 argument"));
    }
    let val = args[0].clone();
    let val: &ValueData = &val.borrow();
    let floating = f64::from(val.clone());
    Ok(new_ref(ValueData::Number(floating.tanh())))
}

pub fn math_random(_: &mut Frame<'_>, _: Value, _: &[Value]) -> Result<Value, ValueData> {
    let num = rand::random::<f64>();
    Ok(new_ref(ValueData::Number(num)))
}

pub fn math_random_int(
    frame: &mut Frame<'_>,
    _: Value,
    args: &[Value],
) -> Result<Value, ValueData> {
    if args.len() < 2 {
        return Err(new_error(-1, None, "Math.randomInt: expected 2 arguments"));
    }
    let min = args[0].clone();
    let max = args[1].clone();
    let min = min.borrow().clone();
    let max = max.borrow().clone();
    let random_val = math_random(frame, nil(), &[])?;
    let val = random_val.borrow().clone();
    Ok(new_ref(
        math_floor(frame, nil(), &[new_ref(val * (max - min.clone()))])?
            .borrow()
            .clone()
            + min,
    ))
}

pub fn math_exp(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    if args.is_empty() {
        return Err(new_error(-1, None, "Math.exp: expected 1 argument"));
    }
    let val = f64::from(args[0].borrow().clone());
    return Ok(new_ref(ValueData::Number(val.exp())));
}

pub fn math_abs(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    if args.is_empty() {
        return Err(new_error(-1, None, "Math.abs: expected 1 argument"));
    }
    let val = f64::from(args[0].borrow().clone());
    return Ok(new_ref(ValueData::Number(val.abs())));
}

pub fn math_pow(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    if args.len() < 2 {
        return Err(new_error(-1, None, "Math.pow: expected 2 arguments"));
    }
    let val = f64::from(args[0].borrow().clone());
    let exp = f64::from(args[1].borrow().clone());
    return Ok(new_ref(ValueData::Number(val.powf(exp))));
}

pub fn math_sqrt(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    if args.is_empty() {
        return Err(new_error(-1, None, "Math.sqrt: expected 1 argument"));
    }
    let val = f64::from(args[0].borrow().clone());
    return Ok(new_ref(ValueData::Number(val.sqrt())));
}

pub fn math_object() -> Ref<Object> {
    let object = new_object();
    object.borrow_mut().set("pow", new_exfunc(math_pow));
    object.borrow_mut().set("sqrt", new_exfunc(math_sqrt));
    object.borrow_mut().set("abs", new_exfunc(math_abs));
    object.borrow_mut().set("floor", new_exfunc(math_floor));
    object.borrow_mut().set("tanh", new_exfunc(math_tanh));
    object.borrow_mut().set("random", new_exfunc(math_random));
    object
        .borrow_mut()
        .set("randomInt", new_exfunc(math_random_int));
    object
        .borrow_mut()
        .set("E", new_ref(ValueData::Number(std::f64::consts::E)));
    object
        .borrow_mut()
        .set("PI", new_ref(ValueData::Number(std::f64::consts::PI)));
    object
}
