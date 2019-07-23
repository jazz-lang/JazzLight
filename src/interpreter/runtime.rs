use super::value::*;
use super::*;
use crate::token::Position;

pub fn builtin_instanceof(_: Value,args: &[Value]) -> Result<Value,ValueData> {
    if args.len() < 2 {
        return Err(new_error(-1,None,"instanceof: two arguments expected"));
    }
    let x: &ValueData = &args[0].borrow();
    let y: &ValueData = &args[1].borrow();
    let (x,y) = match (x,y) {
        (ValueData::Object(x),ValueData::Object(y)) => (x,y),
        _ => return Err(new_error(-1,None,"instanceof: expected objects as arguments"))
    };

    let check = instanceof(x,y);
    Ok(new_ref(ValueData::Bool(check)))
}

pub fn builtin_print(_: Value,args: &[Value]) -> Result<Value,ValueData> {
    for arg in args.iter() {
        print!("{}",arg.borrow());
    }
    println!("");
    Ok(new_ref(ValueData::Nil))
}

pub fn new_exfunc(f: fn(Value,&[Value]) -> Result<Value,ValueData>) -> Value {
    new_ref(
        ValueData::Function(
            new_ref(Function::Native(f as usize))
        )
    )
}


pub fn register_builtins(interp: &mut Interpreter) {
    let err = new_object();
    let pos = &Position::new(0,0);
    err.borrow_mut().set("__name__","JLRuntimeError");
    declare_var(&interp.env, "JLRuntimeError", new_ref(ValueData::Object(err)),&pos ).unwrap();
    declare_var(&interp.env, "instanceof", new_exfunc(builtin_instanceof),&pos).unwrap();
    declare_var(&interp.env, "print",new_exfunc(builtin_print),&pos).unwrap();
}

