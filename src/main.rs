extern crate jazzc;

use jazzc::compile::compile_ast;
use jazzc::compile::Context;
use jazzc::compile::Global;
use jazzc::parser::Parser;
use jazzc::reader::Reader;
use jazzvm::module::Module;
use jazzvm::value::*;
use jazzvm::vm::VM;
use jazzvm::P;

pub fn module_from_ctx(ctx: &Context) -> P<Module> {
    let mut m = Module::new(&ctx.cur_file);
    let m = P(m);
    for g in ctx.g.table.iter() {
        let val = match g {
            Global::Func(off, nargs) => Value::Func(P(Function {
                var: FuncVar::Offset(*off as usize),
                nargs: *nargs,
                env: P(Value::Array(P(vec![]))),
                module: m.clone(),
            })),

            v => panic!("{:?}", v),
        };
        m.borrow_mut().globals.push(P(val));
    }
    m.borrow_mut().code = ctx.ops.clone();

    m
}
fn main() {
    let reader = Reader::from_string(
        "
        var f = function(x) -> return x + 2

        return f(3)
            
    ",
    );
    let mut ast = vec![];
    let mut parser = Parser::new(reader, &mut ast);
    parser.parse().unwrap();
    let ctx = compile_ast(ast);

    for (i, op) in ctx.ops.iter().enumerate() {
        println!("{:04}: {:?}", i, op)
    }

    let mut m = module_from_ctx(&ctx);

    let mut vm = VM::new();
    vm.code = ctx.ops.clone();
    println!("{:?}", vm.interp(&mut m));
}
