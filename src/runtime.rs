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
}
