extern crate vmm;
use bytecode::*;
use pgc::*;
use value::*;
use vmm::*;

fn main() {
    pgc::enable_gc_stats();
    init_builtins();
    let mut module = Module::new();
    /*let s = Rooted::new("Object".to_owned());
    let s2 = Rooted::new("toString".to_owned());
    module.globals.push(Value::String(s.inner()));
    module.globals.push(Value::String(s2.inner()));
    module.code.push(Op::LoadGlobal(1));
    module.code.push(Op::LoadGlobal(0));
    module.code.push(Op::LoadStatic);
    module.code.push(Op::LoadField);
    module.code.push(Op::Return);*/

    module.code.push(Op::ConstInt(0));
    module.code.push(Op::StoreLocal(0));
    module.code.push(Op::LoadLocal(0));
    module.code.push(Op::ConstInt(100000));
    module.code.push(Op::CmpGt);
    module.code.push(Op::BranchIfFalse(11));
    module.code.push(Op::LoadLocal(0));
    module.code.push(Op::ConstInt(1));
    module.code.push(Op::Add);
    module.code.push(Op::StoreLocal(0));
    module.code.push(Op::Branch(2));
    module.code.push(Op::LoadLocal(0));
    module.code.push(Op::Return);
    let gc_module = Rooted::new(module);
    let res = run_module(gc_module.inner());

    println!("{}", res);
    pgc::gc_collect();
    pgc::gc_summary();
}
