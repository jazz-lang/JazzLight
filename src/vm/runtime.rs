use super::value::*;
use super::*;
use crate::token::Position;

pub fn builtin_instanceof(_: &mut Frame<'_>,_: Value, args: &[Value]) -> Result<Value, ValueData> {
    if args.len() < 2 {
        return Err(new_error(-1, None, "instanceof: two arguments expected"));
    }
    let x: &ValueData = &args[0].borrow();
    let y: &ValueData = &args[1].borrow();
    let (x, y) = match (x, y) {
        (ValueData::Object(x), ValueData::Object(y)) => (x, y),
        _ => {
            return Err(new_error(
                -1,
                None,
                "instanceof: expected objects as arguments",
            ))
        }
    };

    let check = instanceof(x, y);
    Ok(new_ref(ValueData::Bool(check)))
}

pub fn builtin_print(_: &mut Frame<'_>,_: Value, args: &[Value]) -> Result<Value, ValueData> {
    for arg in args.iter() {
        print!("{}", arg.borrow());
    }
    println!("");
    Ok(new_ref(ValueData::Nil))
}

pub fn new_exfunc(f: fn(&mut Frame<'_>,Value, &[Value]) -> Result<Value, ValueData>) -> Value {
    new_ref(ValueData::Function(new_ref(Function::Native(f as usize))))
}

pub fn builtin_gc(_: &mut Frame<'_>,_: Value, _: &[Value]) -> Result<Value, ValueData> {
    crate::gc::gc::mark(100);
    //crate::gc::gc::sweep();
    Ok(new_ref(ValueData::Nil))
}

pub fn builtin_spawn(_: &mut Frame<'_>,_: Value,args: &[Value]) -> Result<Value,ValueData> {
    if args.is_empty() {
        return Err(new_error(0,None,"function expected"));
    }

    let val = args[0].clone();
    let val: &ValueData = &val.borrow();
    match val {
        ValueData::Function(fun) => {
            let fun: &Function = &fun.borrow(); 
            match fun {
                Function::Regular {
                    environment,
                    args,
                    code,
                    addr,
                    ..
                }  => {
                    let func = Function::Regular {
                        environment: environment.clone(),
                        args: args.clone(),
                        code: code.clone(),
                        addr: *addr,     
                        yield_env: new_object(),
                        yield_pos: None,
                    };

                    return Ok(new_ref(ValueData::Function(new_ref(func))));
                }
                _ => return Err(new_error(0,None,"regular function expected"))
            }
        }
        _ => return Err(new_error(0,None,"function expected"))
    }
}

pub fn register_builtins(interp: &mut Frame<'_>) {
    let err = new_object();
    let pos = &Position::new(0, 0);
    err.borrow_mut().set("__name__", "JLRuntimeError");
    declare_var(
        &interp.env,
        "JLRuntimeError",
        new_ref(ValueData::Object(err)),
        &pos,
    )
    .unwrap();
    declare_var(
        &interp.env,
        "instanceof",
        new_exfunc(builtin_instanceof),
        &pos,
    )
    .unwrap();
    declare_var(&interp.env, "print", new_exfunc(builtin_print), &pos).unwrap();
    declare_var(&interp.env, "gc", new_exfunc(builtin_gc), &pos).unwrap();

    declare_var(&interp.env, "spawn", new_exfunc(builtin_spawn), &pos).unwrap();
    
}
