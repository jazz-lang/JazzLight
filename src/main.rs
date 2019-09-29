extern crate vmm;
use bytecode::*;
use pgc::*;
use value::*;
use vmm::*;

fn main() {
    init_builtins();
    println!("Hi!");
    let mut module = Module::new();
    module
        .globals
        .push(Value::String(Gc::new("Object".to_owned())));
    module.code.push(Op::LoadGlobal(0));
    module.code.push(Op::LoadStatic);
    let gc_module = Rooted::new(module);
    let handle = spawn_thread(move || run_module(gc_module.inner()));

    println!("{}", handle.join().unwrap());
}
