use crate::module::Module;
use crate::value::*;
use crate::vm::{FIELDS, VM};
use crate::P;

pub extern "C" fn load(_: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    use libloading as lib;
    if val_is_str(&args[0]) {
        let path = val_str(&args[0]);
        let symbol_name = val_str(&args[1]);
        let nargs = val_int(&args[2]);
        let lib = lib::Library::new(&path).unwrap();

        unsafe {
            let func: lib::Symbol<*mut u8> = lib.get(symbol_name.as_bytes()).unwrap();
            let func = Function {
                var: FuncVar::Native((*func) as *const u8),
                nargs: nargs as i32,
                jit: false,
                module: P(Module::new(&path)),
                env: P(Value::Null),
                yield_point: 0,
            };
            P(Value::Func(P(func)))
        }
    } else {
        panic!("String expected");
    }
}

pub extern "C" fn os_string(_: &mut VM, _args: Vec<P<Value>>) -> P<Value> {
    P(Value::Str(std::env::consts::OS.to_owned()))
}

pub fn val_string(vm: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    let mut buff = String::new();
    for val in args.iter() {
        match val.borrow() {
            Value::Int(i) => buff.push_str(&i.to_string()),
            Value::Int32(i) => buff.push_str(&i.to_string()),
            Value::Float(f) => buff.push_str(&f.to_string()),
            Value::Func(_) => buff.push_str("<function>"),
            Value::Bool(b) => buff.push_str(&b.to_string()),
            Value::Null => buff.push_str("null"),
            Value::Str(s) => buff.push_str(s),
            Value::Object(obj) => {
                let obj: &Object = obj.borrow();
                buff.push_str("{ ");
                for (idx, entry) in obj.entries.iter().enumerate() {
                    let name = FIELDS.borrow().get(&(entry.hash as u64)).unwrap();
                    buff.push_str(&format!("{} => ", name));
                    let entry = entry.borrow();
                    let val = entry.val.clone();
                    let s = val_string(vm, vec![val]);
                    if let Value::Str(s) = s.borrow() {
                        buff.push_str(s);
                    }
                    if idx != obj.entries.len() - 1 {
                        buff.push_str(", ");
                    }
                }
                buff.push_str(" }");
            }
            Value::Array(values) => {
                let arr = values.borrow();
                buff.push_str("[");
                for (idx, val) in arr.iter().enumerate() {
                    let s = val_string(vm, vec![val.clone()]);
                    if let Value::Str(s) = s.borrow() {
                        buff.push_str(s);
                        if idx != arr.len() - 1 {
                            buff.push_str(", ");
                        }
                    }
                }
                buff.push_str("]");
            }
            Value::Extern(ptr, name) => buff.push_str(&format!("<{}> at {:?}", name, ptr)),
        }
    }

    P(Value::Str(buff))
}

pub extern "C" fn print(vm: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    if let Value::Str(val) = val_string(vm, args).borrow() {
        println!("{}", val);
    }

    P(Value::Null)
}
#[no_mangle]
pub extern "C" fn loader_loadmodule(_: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    let name = if let Value::Str(s) = args[0].borrow() {
        s.to_owned()
    } else {
        println!("{:?}", args[0]);
        unreachable!();
    };
    //let vthis = args[1].clone();

    use crate::module::*;
    use std::fs::File;
    use std::io::Read;

    let mut reader = Reader {
        code: vec![],
        pc: 0,
    };

    let mut f = File::open(&name).unwrap();
    f.read_to_end(&mut reader.code).unwrap();

    let mut module = read_module(reader, &name);

    let mut vm = VM::new();
    vm.builtins = crate::vm::VM_THREAD.builtins.clone();
    vm.code = module.code.clone();
    vm.interp(&mut module);

    let exports = module.exports.clone();

    return exports;
}

pub extern "C" fn array(_: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    P(Value::Array(P(args)))
}

pub extern "C" fn alen(_: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    if val_is_array(&args[0]) {
        let array_p = val_array(&args[0]);
        let array = array_p.borrow();

        return P(Value::Int(array.len() as i64));
    } else {
        panic!("Array expected");
    }
}

pub extern "C" fn apush(_: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    if val_is_array(&args[0]) {
        let array_p = val_array(&args[0]);
        let array = array_p.borrow_mut();
        let val = args[1].clone();
        array.push(val);
    }

    P(Value::Null)
}

pub extern "C" fn apop(_: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    if val_is_array(&args[0]) {
        let array_p = val_array(&args[0]);
        let array = array_p.borrow_mut();
        array.pop().unwrap_or(P(Value::Null))
    } else {
        P(Value::Null)
    }
}

pub extern "C" fn aset(_: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    if val_is_array(&args[0]) {
        let array_p = val_array(&args[0]);
        let array = array_p.borrow_mut();
        let key = val_int(&args[1]);
        let val = args[2].clone();
        array[key as usize] = val;
    }
    // Throw error if val not array?
    P(Value::Null)
}

pub extern "C" fn aget(_: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    if val_is_array(&args[0]) {
        let array_p = val_array(&args[0]);
        let array = array_p.borrow();
        let key = val_int(&args[1]);
        return array.get(key as usize).unwrap_or(&P(Value::Null)).clone();
    }
    P(Value::Null)
}
use crate::Cell;
lazy_static::lazy_static!(
    pub static ref THREADS: Cell<fnv::FnvHashMap<i32,Cell<std::thread::JoinHandle<P<Value>>>>> = Cell::new(fnv::FnvHashMap::default());
);

pub extern "C" fn thread_spawn(_: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    if val_is_func(&args[0]) {
        use crate::vm::callex;
        let val = args[0].clone();

        let thread = std::thread::spawn(|| callex(P(Value::Null), val, vec![]));
        let id = thread.thread().id();
        let id: u64 = unsafe { std::mem::transmute(id) };

        THREADS.borrow_mut().insert(id as i32, Cell::new(thread));

        return P(Value::Int32(id as i32));
    }
    P(Value::Null)
}

pub extern "C" fn thread_join(_: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    let val = args[0].borrow();
    match &val {
        Value::Int32(idx) => {
            let thread = THREADS
                .borrow()
                .get(idx)
                .expect("Thread not found")
                .direct();

            thread.join().unwrap()
        }
        _ => P(Value::Null),
    }
}

macro_rules! new_builtin {
    ($vm: expr,$f: ident) => {
        let f = Function {
            nargs: -1,
            var: FuncVar::Native($f as *const u8),
            module: P(Module::new("__0")),
            env: P(Value::Null),
            jit: false,
            yield_point: 0,
        };
        $vm.builtins.push(P(Value::Func(P(f))));
    };
}

pub fn register_builtins(vm: &mut VM) {
    new_builtin!(vm, load);
    new_builtin!(vm, val_string);
    new_builtin!(vm, print);
    new_builtin!(vm, array);
    new_builtin!(vm, alen);
    new_builtin!(vm, apush);
    new_builtin!(vm, apop);
    new_builtin!(vm, aset);
    new_builtin!(vm, aget);
    new_builtin!(vm, os_string);
    new_builtin!(vm, thread_spawn);
    new_builtin!(vm, thread_join);
    new_builtin!(vm, loader_loadmodule);
}

pub fn loader(module: &P<Module>) -> P<Value> {
    let mut obj = Object { entries: vec![] };

    let f = Function {
        var: FuncVar::Native(loader_loadmodule as i64 as *const u8),
        env: P(Value::Array(P(vec![]))),
        module: module.clone(),
        nargs: 1,
        jit: false,
        yield_point: 0,
    };

    obj.insert(crate::fields::hash_str("loadmodule"), P(Value::Func(P(f))));

    P(Value::Object(P(obj)))
}
