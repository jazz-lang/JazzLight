extern crate vmm;
use bytecode::*;
use pgc::*;
use value::*;
use vmm::*;

fn main() {
    init_builtins();
    let mut module = Module::new();
    let s = Rooted::new("Object".to_owned());
    module.globals.push(Value::String(s.inner()));
    module.code.push(Op::LoadGlobal(0));
    module.code.push(Op::LoadStatic);
    module.code.push(Op::Return);
    let gc_module = Rooted::new(module);
    let handle = spawn_thread(move || run_module(gc_module.inner()));

    println!("{}", handle.join().unwrap());
}
