use super::value::*;
use super::*;
use crate::token::Position;
use crate::vm::runtime::array::array_object;
use crate::vm::runtime::math::math_object;
use crate::reader::Reader;
use crate::parser::Parser;
use crate::compiler::Compiler;

pub mod array;
pub mod math;
pub mod object;

pub macro decl_fun {
    (function $name : ident ($frame: ident,$this: ident $($arg: ident),*)  $b: block ) => {
        pub fn $name ( $frame: &mut Frame<'_>,$this: Value,args: &[Value]) -> Result<Value,ValueData> {
            let mut ___i = 0;
            $(

                let $arg = args[___i].clone();
                ___i += 1;
            )*
            $b
        }
    };

}

pub fn builtin_instanceof(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
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

pub fn builtin_print(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    for arg in args.iter() {
        print!("{}", arg.borrow());
    }
    println!("");
    Ok(new_ref(ValueData::Nil))
}

pub fn new_exfunc(f: fn(&mut Frame<'_>, Value, &[Value]) -> Result<Value, ValueData>) -> Value {
    new_ref(ValueData::Function(new_ref(Function::Native(f as usize))))
}

pub fn builtin_gc(_: &mut Frame<'_>, _: Value, _: &[Value]) -> Result<Value, ValueData> {
    crate::ngc::gc_collect_not_par();
    //crate::gc::gc::sweep();
    Ok(new_ref(ValueData::Nil))
}

pub fn enable_stats(_: &mut Frame<'_>, _: Value, _: &[Value]) -> Result<Value, ValueData> {
    crate::ngc::gc_enable_stats();
    Ok(new_ref(ValueData::Nil))
}

pub fn builtin_spawn(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    if args.is_empty() {
        return Err(new_error(0, None, "function expected"));
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
                    //constants,
                    ..
                } => {
                    let func = Function::Regular {
                        environment: environment.clone(),
                        args: args.clone(),
                        code: code.clone(),
                        addr: *addr,
                        //constants: constants.clone(),
                        yield_env: new_object(),
                        yield_pos: None,
                    };

                    return Ok(new_ref(ValueData::Function(new_ref(func))));
                }
                _ => return Err(new_error(0, None, "regular function expected")),
            }
        }
        _ => return Err(new_error(0, None, "function expected")),
    }
}

pub fn type_of(_: &mut Frame<'_>, _: Value, args: &[Value]) -> Result<Value, ValueData> {
    let arg = args[0].clone();
    let val: &ValueData = &arg.borrow();
    let name = match val {
        ValueData::Number(_) => "number",
        ValueData::Nil => "nil",
        ValueData::Undefined => "undefined",
        ValueData::String(_) => "string",
        ValueData::Object(_) => "object",
        ValueData::Array(_) => "array",
        ValueData::Function(_) => "function",
        ValueData::Bool(_) => "bool",
        ValueData::Iterator(_) => "iterator",
    };
    Ok(new_ref(ValueData::String(name.to_owned())))
}

pub fn require(frame: &mut Frame<'_>,_: Value,args: &[Value]) -> Result<Value,ValueData> {
    let name = String::from(args[0].borrow().clone());
    let cur_dir = std::env::current_dir().unwrap().to_str().unwrap().to_string();
    let cur_path = format!("{}/{}",cur_dir,name);
    let path = if std::path::Path::new(&cur_path).exists() {
        cur_path
    } else {
        let home_dir = option_env!("JAZZ_HOME");
        if let Some(home_dir) = home_dir {
            format!("{}/{}", home_dir, name)
        } else {
            name
        }
    };

    let r = match Reader::from_file(&path) {
        Ok(r) => r,
        Err(e) => return Err(new_error(-1,None,&format!("{}",e)))
    };
    let mut ast = vec![];
    let mut p = Parser::new(r,&mut ast);
    match p.parse() {
        Ok(_) => (),
        Err(e) => return Err(new_error(-1,None,&format!("{}",e)))
    }

    let mut f = Frame::new(frame.m);
    let mut c = Compiler::new(&mut f);
    c.compile_ast(&ast,true);
    c.frame.execute();
    let exports = get_variable(&c.frame.env,"exports",&Position::new(0,0)).unwrap();
    Ok(exports)
}

pub fn len(_: &mut Frame<'_>,_: Value,args: &[Value]) -> Result<Value,ValueData> {
    let arg = args[0].clone();
    let arg: &ValueData = &arg.borrow();
    match arg {
        ValueData::Array(arr) => Ok(new_ref(ValueData::Number(arr.borrow().len() as f64))),
        ValueData::Object(object) => Ok(new_ref(ValueData::Number(object.borrow().table.len() as f64))),
        ValueData::String(s) => Ok(new_ref(ValueData::Number(s.len() as f64))),
        _ => Ok(new_ref(ValueData::Nil))


    }

}

pub fn register_builtins(interp: &mut Frame<'_>) {
    let err = new_object();
    let pos = &Position::new(0, 0);
    err.borrow_mut().set("__name__", "JLRuntimeError");
    let obj = new_ref(ValueData::Object(err));
    //gc_add_root(obj.gc());
    declare_var(&interp.env, "JLRuntimeError", obj, &pos).unwrap();
    declare_var(
        &interp.env,
        "instanceof",
        new_exfunc(builtin_instanceof),
        &pos,
    )
    .unwrap();
    declare_var(&interp.env, "exports",new_ref(ValueData::Object(new_object())),&pos).unwrap();
    declare_var(&interp.env, "require",new_exfunc(require),&pos).unwrap();
    declare_var(&interp.env, "print", new_exfunc(builtin_print), &pos).unwrap();
    declare_var(&interp.env, "gc", new_exfunc(builtin_gc), &pos).unwrap();
    declare_var(&interp.env, "gc_stats", new_exfunc(enable_stats), &pos).unwrap();
    declare_var(&interp.env, "spawn", new_exfunc(builtin_spawn), &pos).unwrap();
    declare_var(&interp.env, "typeof", new_exfunc(type_of), &pos).unwrap();
    declare_var(&interp.env, "Math", new_ref(ValueData::Object(math_object())),&pos).unwrap();
    declare_var(&interp.env,"object_keys",new_exfunc(crate::vm::runtime::object::object_keys),&pos).unwrap();
    declare_var(&interp.env,"len",new_exfunc(len),&pos).unwrap();
}
