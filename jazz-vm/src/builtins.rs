use crate::module::Module;
use crate::value::*;
use crate::vm::{jazz_func, FIELDS, VM};
use crate::P;

pub extern "C" fn load(_: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    use libloading as lib;
    if val_is_str(&args[0]) {
        let path = val_str(&args[0]);
        let symbol_name = val_str(&args[1]);
        let nargs = val_int(&args[2]);
        let lib = lib::Library::new(&path).unwrap();
        unsafe {
            let func: lib::Symbol<jazz_func> = lib.get(symbol_name.as_bytes()).unwrap();
            let func = Function {
                var: FuncVar::Native((*func) as *const u8),
                nargs: nargs as i32,
                module: P(Module::new(&path)),
                env: P(Value::Null),
            };
            P(Value::Func(P(func)))
        }
    } else {
        panic!("String expected");
    }
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

pub extern "C" fn array(_: &mut VM, args: Vec<P<Value>>) -> P<Value> {
    P(Value::Array(P(args)))
}

macro_rules! new_builtin {
    ($vm: expr,$f: ident) => {
        let f = Function {
            nargs: -1,
            var: FuncVar::Native($f as *const u8),
            module: P(Module::new("__0")),
            env: P(Value::Null),
        };
        $vm.builtins.push(P(Value::Func(P(f))));
    };
}

pub fn register_builtins(vm: &mut VM) {
    new_builtin!(vm, load);
    new_builtin!(vm, val_string);
    new_builtin!(vm, print);
    new_builtin!(vm, array);
}
