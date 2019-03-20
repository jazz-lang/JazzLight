use jazzvm::frame::Frame;
use jazzvm::value::*;

macro_rules! func {
    ($compiler: expr ; function $name : ident ($framename: ident;$($arg:ident),*) $b:block ) => {
        {
        pub fn $name(frame_: &mut Frame,mut args: Vec<GcValue>) -> GcValue {
            args.reverse();
            $(
                let $arg = args.pop().unwrap();
                //let $arg: &mut Value = &mut _tmp_.get_mut();
            )*
            let $framename = frame_;
            $b
        }

        let f = Function {
            name: String::from(stringify!($name)),
            var: FuncVar::Native($name as i64),
        };
        let val = GcValue::new(Value::Func(f));
        let cmpl: &mut Compiler = $compiler;
        let idx = cmpl.vm.new_global(val);
        cmpl.globals.insert(String::from(stringify!($name)),idx);
        }
    };
}

use crate::compiler::Compiler;

pub fn string(frame: &mut Frame, args: Vec<GcValue>) -> GcValue
{
    let v_clon = args[0].clone();
    let val: &Value = &args[0].get();
    let string: String = match val
    {
        Value::Str(s) => s.clone(),
        Value::Float(f) => f.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_owned(),
        Value::Array(values) =>
        {
            let mut buff = String::new();
            let mut i = 0;
            buff.push('[');
            while i < values.len()
            {
                let s = string(frame, vec![values[i].clone()]);
                let s: &Value = &s.get();
                let vstr = if let Value::Str(s) = s
                {
                    s.clone()
                }
                else
                {
                    unreachable!()
                };
                buff.push_str(&vstr);
                if i != values.len() - 1
                {
                    buff.push(',');
                }
                i += 1;
            }
            buff.push(']');
            buff
        }
        Value::Object(obj) =>
        {
            let f = obj.find(&GcValue::new(Value::Str(format!("display"))));
            let result = frame.invoke(&f.clone(), v_clon, 0);
            let vref: &Value = &result.get();
            let string = if let Value::Str(s) = vref
            {
                s.to_owned()
            }
            else
            {
                panic!("String expected")
            };
            string
        }
        _ => unimplemented!(),
    };
    GcValue::new(Value::Str(string))
}

fn to_string(val: &GcValue) -> String
{
    let val: &Value = &val.get();
    if let Value::Str(s) = val
    {
        return s.to_string();
    }
    else
    {
        panic!("String value expected");
    }
}

pub fn println(frame: &mut Frame, args: Vec<GcValue>) -> GcValue
{
    let s = string(frame, args);
    let s_ref: &Value = &s.get();
    if let Value::Str(s) = s_ref
    {
        println!("{}", s);
    }
    else
    {
        panic!("String expected");
    }
    GcValue::new(Value::Null)
}

pub fn apush(_frame: &mut Frame, args: Vec<GcValue>) -> GcValue
{
    let vref = args[0].clone();
    let array: &mut Value = &mut vref.get_mut();
    let val = args[1].clone();
    if let Value::Array(values) = array
    {
        values.push(val);
    }
    GcValue::new(Value::Null)
}

pub fn apop(_frame: &mut Frame, args: Vec<GcValue>) -> GcValue
{
    let array: &mut Value = &mut args.get(0).unwrap().get_mut();
    if let Value::Array(values) = array
    {
        return values.pop().unwrap_or(GcValue::new(Value::Null));
    }
    else
    {
        panic!("Array expected; apop,found: {:?}", array);
    }
}

pub fn len(_frame: &mut Frame, args: Vec<GcValue>) -> GcValue
{
    let val: &Value = &args[0].get();

    match val
    {
        Value::Str(s) => GcValue::new(Value::Int(s.len() as i64)),
        Value::Array(arr) => GcValue::new(Value::Int(arr.len() as i64)),
        _ => GcValue::new(Value::Int(-1)),
    }
}
pub fn aget(_frame: &mut Frame, args: Vec<GcValue>) -> GcValue
{
    let array: &Value = &args[0].get();
    let idx: &Value = &args[1].get();
    let idx_usize = if let Value::Int(i) = idx
    {
        *i as usize
    }
    else
    {
        panic!("Integer expected")
    };
    if let Value::Array(values) = array
    {
        return values[idx_usize].clone();
    }
    else
    {
        panic!("Array expected")
    }
}

pub fn aset(_frame: &mut Frame, args: Vec<GcValue>) -> GcValue
{
    let vref = args[0].clone();
    let array: &mut Value = &mut vref.get_mut();
    let vref = args[1].clone();
    let idx: &Value = &vref.get();
    let val = args[2].clone();
    let idx_usize = if let Value::Int(i) = idx
    {
        *i as usize
    }
    else
    {
        panic!("Integer expected")
    };
    if let Value::Array(values) = array
    {
        values[idx_usize] = val;
    }
    else
    {
        panic!("Array expected");
    }
    GcValue::new(Value::Null)
}

pub fn strtrim(frame: &mut Frame, args: Vec<GcValue>) -> GcValue
{
    let s = to_string(&args[0]);
    GcValue::new(Value::Str(s.trim().to_owned()))
}

pub fn str2chars(frame: &mut Frame, args: Vec<GcValue>) -> GcValue
{
    let s = to_string(&args[0]);
    let mut buff = vec![];
    for ch in s.chars()
    {
        buff.push(GcValue::new(Value::Str(format!("{}", ch))));
    }

    GcValue::new(Value::Array(buff))
}

pub fn open_file(frame: &mut Frame, args: Vec<GcValue>) -> GcValue
{
    let s = to_string(&args[0]);
    let mut buff = String::new();
    use std::io::Read;
    std::fs::File::open(&s).unwrap()
                           .read_to_string(&mut buff)
                           .unwrap();
    return GcValue::new(Value::Str(buff));
}

pub fn builtin_sqrt(frame: &mut Frame, args: Vec<GcValue>) -> GcValue
{
    let val = &args[0];
    let val_ref: &Value = &val.get();

    let val = match val_ref
    {
        Value::Int(i) => Value::Float((*i as f64).sqrt()),
        Value::Float(f) => Value::Float(f.sqrt()),
        _ => unreachable!(),
    };
    GcValue::new(val)
}

pub fn builtin_sin(frame: &mut Frame, args: Vec<GcValue>) -> GcValue
{
    let val = &args[0];
    let val_ref: &Value = &val.get();
    let val = match val_ref
    {
        Value::Float(f) => Value::Float(f.sin()),
        Value::Int(i) => Value::Float((*i as f64).sin()),
        _ => unreachable!(),
    };
    GcValue::new(val)
}
pub fn builtins(cmpl: &mut Compiler)
{
    fn concat(frame: &mut Frame, args: Vec<GcValue>) -> GcValue
    {
        let mut buff = String::new();
        for arg in args.iter()
        {
            let s = string(frame, vec![arg.clone()]);
            let s: &Value = &s.get();
            if let Value::Str(ref s) = s
            {
                buff.push_str(s);
            }
            else
            {
                panic!("String expected");
            }
        }

        return GcValue::new(Value::Str(buff));
    }

    let f = Function { name: String::from("concat"),
                       var: FuncVar::Native(concat as i64) };
    let val = GcValue::new(Value::Func(f));
    let idx = cmpl.vm.new_global(val);
    cmpl.globals.insert(String::from("concat"), idx);

    let f = Function { name: String::from("string"),
                       var: FuncVar::Native(string as i64) };
    let val = GcValue::new(Value::Func(f));
    let idx = cmpl.vm.new_global(val);
    cmpl.globals.insert(String::from("string"), idx);

    let f = Function { name: String::from("println"),
                       var: FuncVar::Native(println as i64) };
    let val = GcValue::new(Value::Func(f));
    let idx = cmpl.vm.new_global(val);
    cmpl.globals.insert(String::from("println"), idx);

    macro_rules! reg_fn {
        ($compiler: expr,$name: ident) => {
            let f = Function { name: String::from(stringify!($name)),
                               var: FuncVar::Native($name as i64) };
            let val = GcValue::new(Value::Func(f));
            let cmpl: &mut Compiler = $compiler;
            let idx = cmpl.vm.new_global(val);
            cmpl.globals.insert(String::from(stringify!($name)), idx);
        };
    }
    reg_fn!(cmpl, apop);
    reg_fn!(cmpl, apush);
    reg_fn!(cmpl, aget);
    reg_fn!(cmpl, aset);
    reg_fn!(cmpl, len);
    reg_fn!(cmpl, str2chars);
    reg_fn!(cmpl, strtrim);
    reg_fn!(cmpl, open_file);
    reg_fn!(cmpl, builtin_sin);
    reg_fn!(cmpl, builtin_sqrt);
}
