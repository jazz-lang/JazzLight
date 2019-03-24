use jazzvm::{api::*, frame::Frame, value::*};
use sdl2_sys::*;

use crate::compiler::Compiler;

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

fn val_as_cstr(val: &GcValue) -> *const i8 {
    use std::ffi::CString;

    val.map(&mut |val| match val {
        Value::Str(s) => s.as_bytes().as_ptr() as *const i8,
        _ => panic!("String value expected"),
    })
}

pub fn rsdl_create_window(_: &mut Frame, args: Vec<GcValue>) -> GcValue {
    unsafe {
        let window = SDL_CreateWindow(
            val_as_cstr(&args[0]),     // title
            val_as_int(&args[1]) as _, // x
            val_as_int(&args[2]) as _, // y
            val_as_int(&args[3]) as _, // w
            val_as_int(&args[4]) as _, // h
            val_as_int(&args[5]) as _, // flags
        );

        if window.is_null() {
            panic!("Failed to create SDL2 window")
        }

        return GcValue::new(Value::Int(window as i64));
    }
}

pub fn rsdl_init_video(_: &mut Frame, _: Vec<GcValue>) -> GcValue {
    unsafe {
        SDL_Init(SDL_INIT_VIDEO);
    };
    GcValue::new(Value::Null)
}

pub fn rsdl_init_everything(_: &mut Frame, _: Vec<GcValue>) -> GcValue {
    unsafe {
        SDL_Init(SDL_INIT_EVERYTHING);
    }
    GcValue::new(Value::Null)
}

pub fn string(frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let v_clon = args[0].clone();
    let val: &Value = &args[0].get();
    let string: String = match val {
        Value::Str(s) => s.clone(),
        Value::Float(f) => f.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_owned(),
        Value::Array(values) => {
            let mut buff = String::new();
            let mut i = 0;
            buff.push('[');
            while i < values.len() {
                let s = string(frame, vec![values[i].clone()]);
                let s: &Value = &s.get();
                let vstr = if let Value::Str(s) = s {
                    s.clone()
                } else {
                    unreachable!()
                };
                buff.push_str(&vstr);
                if i != values.len() - 1 {
                    buff.push(',');
                }
                i += 1;
            }
            buff.push(']');
            buff
        }
        Value::Object(obj) => {
            let f = obj.find(&GcValue::new(Value::Str(format!("display"))));
            let result = frame.invoke(&f.clone(), v_clon, 0);
            let vref: &Value = &result.get();
            let string = if let Value::Str(s) = vref {
                s.to_owned()
            } else {
                panic!("String expected")
            };
            string
        }
        _ => unimplemented!(),
    };
    GcValue::new(Value::Str(string))
}

fn to_string(val: &GcValue) -> String {
    let val: &Value = &val.get();
    if let Value::Str(s) = val {
        return s.to_string();
    } else {
        panic!("String value expected");
    }
}

pub fn print(frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let s = string(frame, args);
    let s_ref: &Value = &s.get();
    if let Value::Str(s) = s_ref {
        print!("{}", s);
    } else {
        panic!("String expected");
    }
    GcValue::new(Value::Null)
}

pub fn println(frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let s = string(frame, args);
    let s_ref: &Value = &s.get();
    if let Value::Str(s) = s_ref {
        println!("{}", s);
    } else {
        panic!("String expected");
    }
    GcValue::new(Value::Null)
}

pub fn error(frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    print!("Error: ");
    println(frame, args);
    std::process::exit(-1);
}

pub fn apush(_frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let vref = args[0].clone();
    let array: &mut Value = &mut vref.get_mut();
    let val = args[1].clone();
    if let Value::Array(values) = array {
        values.push(val);
    }
    GcValue::new(Value::Null)
}

pub fn apop(_frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let array: &mut Value = &mut args.get(0).unwrap().get_mut();
    if let Value::Array(values) = array {
        return values.pop().unwrap_or(GcValue::new(Value::Null));
    } else {
        panic!("Array expected; apop,found: {:?}", array);
    }
}

pub fn len(_frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let val: &Value = &args[0].get();

    match val {
        Value::Str(s) => GcValue::new(Value::Int(s.len() as i64)),
        Value::Array(arr) => GcValue::new(Value::Int(arr.len() as i64)),
        _ => GcValue::new(Value::Int(-1)),
    }
}
pub fn aget(_frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let array: &Value = &args[0].get();
    let idx: &Value = &args[1].get();
    let idx_usize = if let Value::Int(i) = idx {
        *i as usize
    } else {
        panic!("Integer expected")
    };
    if let Value::Array(values) = array {
        return values[idx_usize].clone();
    } else {
        panic!("Array expected")
    }
}

pub fn aset(_frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let vref = args[0].clone();
    let array: &mut Value = &mut vref.get_mut();
    let vref = args[1].clone();
    let idx: &Value = &vref.get();
    let val = args[2].clone();
    let idx_usize = if let Value::Int(i) = idx {
        *i as usize
    } else {
        panic!("Integer expected")
    };
    if let Value::Array(values) = array {
        values[idx_usize] = val;
    } else {
        panic!("Array expected");
    }
    GcValue::new(Value::Null)
}

pub fn strtrim(frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let s = to_string(&args[0]);
    GcValue::new(Value::Str(s.trim().to_owned()))
}

pub fn str2chars(frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let s = to_string(&args[0]);
    let mut buff = vec![];
    for ch in s.chars() {
        buff.push(GcValue::new(Value::Str(format!("{}", ch))));
    }

    GcValue::new(Value::Array(buff))
}

pub fn open_file(frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let s = to_string(&args[0]);
    let mut buff = String::new();
    use std::io::Read;
    std::fs::File::open(&s)
        .unwrap()
        .read_to_string(&mut buff)
        .unwrap();
    return GcValue::new(Value::Str(buff));
}

pub fn builtin_sqrt(frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let val = &args[0];
    let val_ref: &Value = &val.get();

    let val = match val_ref {
        Value::Int(i) => Value::Float((*i as f64).sqrt()),
        Value::Float(f) => Value::Float(f.sqrt()),
        _ => unreachable!(),
    };
    GcValue::new(val)
}

pub fn builtin_readline(_: &mut Frame, _: Vec<GcValue>) -> GcValue {
    use std::io::stdin;
    use std::io::Read;
    let mut buff = String::new();

    stdin()
        .read_to_string(&mut buff)
        .expect("Failed to read string");

    GcValue::new(Value::Str(buff))
}

pub fn builtin_clear(_: &mut Frame, _: Vec<GcValue>) -> GcValue {
    use crossterm::ClearType;
    use crossterm::Terminal;

    let term = Terminal::new();

    term.clear(ClearType::All)
        .expect("Failed to clear terminal");

    GcValue::new(Value::Null)
}

pub fn rand_int(_: &mut Frame, _: Vec<GcValue>) -> GcValue {
    use rand::random;
    GcValue::new(Value::Int(random()))
}

pub fn rand_range(_: &mut Frame, args: Vec<GcValue>) -> GcValue {
    use rand::{thread_rng, Rng};
    let start: i64 = args[0].map(&mut |val| match val {
        Value::Int(i) => *i,
        _ => panic!("Int value expected"),
    });
    let end: i64 = args[1].map(&mut |val| match val {
        Value::Int(i) => *i,
        _ => panic!("Int value expected"),
    });
    let mut rng = thread_rng();
    GcValue::new(Value::Int(rng.gen_range(start, end)))
}

pub fn rand_float(_: &mut Frame, _: Vec<GcValue>) -> GcValue {
    use rand::random;
    GcValue::new(Value::Float(random()))
}

pub fn builtin_console_set_size(_: &mut Frame, args: Vec<GcValue>) -> GcValue {
    use crossterm::Terminal;
    let x: i16 = args[0].map(&mut |val| match val {
        Value::Int(i) => *i as i16,
        Value::Float(f) => *f as i16,
        _ => unimplemented!(),
    });
    let y: i16 = args[1].map(&mut |val| match val {
        Value::Int(i) => *i as i16,
        Value::Float(f) => *f as i16,
        _ => unimplemented!(),
    });

    let term = Terminal::new();
    term.set_size(x, y).expect("Failed to set size");
    GcValue::new(Value::Null)
}

pub fn exit(_: &mut Frame, _: Vec<GcValue>) -> GcValue {
    std::process::exit(0);
}

pub fn assert(_: &mut Frame, args: Vec<GcValue>) -> GcValue {
    args[0].map(&mut |val| {
        match val {
            Value::Bool(b) => assert!(b),
            _ => panic!("Assertion failed"),
        };
        0
    });
    GcValue::new(Value::Null)
}

pub fn hash(frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    use jazzvm::hash::hash_bytes;
    use std::mem::transmute;
    let val = args[0].clone();
    let v_clon = val.clone();
    let val_ref: &Value = &val.get();

    let hash = match val_ref {
        Value::Int(i) => {
            let bytes: [u8; 8] = unsafe { transmute(*i) };
            hash_bytes(&bytes) as i64
        }
        Value::Float(f) => {
            let bytes: [u8; 8] = unsafe { transmute(*f) };
            hash_bytes(&bytes) as i64
        }
        Value::Str(s) => hash_bytes(s.as_bytes()) as i64,
        Value::Bool(b) => {
            let bytes: [u8; 1] = unsafe { transmute(*b) };
            hash_bytes(&bytes) as i64
        }
        Value::Null => hash_bytes(&[0]) as i64,
        Value::Object(obj) => {
            let field = obj
                .find(&GcValue::new(Value::Str("_hash_".to_owned())))
                .clone();
            let h = frame.invoke(&field, v_clon, 0);
            let hash = h.map(&mut |val| match val {
                Value::Int(i) => *i,
                _ => unimplemented!(),
            });
            hash as i64
        }
        Value::Array(arr) => {
            let mut res = 0;
            for elem in arr.iter() {
                let h = hash(frame, vec![elem.clone()]);
                h.map::<i32>(&mut |val| {
                    match val {
                        Value::Int(i) => res += i,
                        _ => unimplemented!(),
                    }
                    0
                });
            }
            res as i64
        }
        Value::Func(_) => panic!("Can't hash function"),
    };
    GcValue::new(Value::Int(hash))
}

pub fn builtin_sin(frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let val = &args[0];
    let val_ref: &Value = &val.get();
    let val = match val_ref {
        Value::Float(f) => Value::Float(f.sin()),
        Value::Int(i) => Value::Float((*i as f64).sin()),
        _ => unreachable!(),
    };
    GcValue::new(val)
}

pub fn clone(frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
    let v_clon = args[0].clone();
    let val: &Value = &args[0].get();
    match val {
        Value::Object(obj) => {
            let f = obj
                .find(&GcValue::new(Value::Str("clone".to_owned())))
                .clone();
            let v = frame.invoke(&f, v_clon, 0);
            return v;
        }
        v => GcValue::new(v.clone()),
    }
}

pub fn builtins(cmpl: &mut Compiler) {
    fn concat(frame: &mut Frame, args: Vec<GcValue>) -> GcValue {
        let mut buff = String::new();
        for arg in args.iter() {
            let s = string(frame, vec![arg.clone()]);
            let s: &Value = &s.get();
            if let Value::Str(ref s) = s {
                buff.push_str(s);
            } else {
                panic!("String expected");
            }
        }

        return GcValue::new(Value::Str(buff));
    }

    let f = Function {
        name: String::from("concat"),
        var: FuncVar::Native(concat as i64),
    };
    let val = GcValue::new(Value::Func(f));
    let idx = cmpl.vm.new_global(val);
    cmpl.globals.insert(String::from("concat"), idx);

    let f = Function {
        name: String::from("string"),
        var: FuncVar::Native(string as i64),
    };
    let val = GcValue::new(Value::Func(f));
    let idx = cmpl.vm.new_global(val);
    cmpl.globals.insert(String::from("string"), idx);

    let f = Function {
        name: String::from("println"),
        var: FuncVar::Native(println as i64),
    };
    let val = GcValue::new(Value::Func(f));
    let idx = cmpl.vm.new_global(val);
    cmpl.globals.insert(String::from("println"), idx);

    macro_rules! reg_fn {
        ($compiler: expr,$name: ident) => {
            let f = Function {
                name: String::from(stringify!($name)),
                var: FuncVar::Native($name as i64),
            };
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
    reg_fn!(cmpl, print);
    reg_fn!(cmpl, builtin_readline);
    reg_fn!(cmpl, builtin_clear);
    reg_fn!(cmpl, builtin_console_set_size);
    reg_fn!(cmpl, rand_int);
    reg_fn!(cmpl, rand_float);
    reg_fn!(cmpl, rand_range);
    reg_fn!(cmpl, hash);
    reg_fn!(cmpl, exit);
    reg_fn!(cmpl, assert);
    reg_fn!(cmpl, clone);
    reg_fn!(cmpl, error);
    reg_fn!(cmpl, rsdl_init_everything);
    reg_fn!(cmpl, rsdl_init_video);
    reg_fn!(cmpl, rsdl_create_window);
}
