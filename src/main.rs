extern crate jazzc;

use jazzc::compile::compile_ast;
use jazzc::compile::Context;
use jazzc::compile::Global;
use jazzc::parser::Parser;
use jazzc::reader::Reader;
use jazzvm::builtins::register_builtins;
use jazzvm::module::Module;
use jazzvm::value::*;
use jazzvm::vm::VM;
use jazzvm::P;

pub fn module_from_ctx(ctx: &Context) -> P<Module> {
    let mut m = Module::new(&ctx.cur_file);
    m.globals = vec![P(Value::Null); ctx.g.table.len()];
    let m = P(m);

    for (i, g) in ctx.g.table.iter().enumerate() {
        match g {
            Global::Func(off, nargs) => {
                let val = Value::Func(P(Function {
                    var: FuncVar::Offset(*off as usize),
                    nargs: *nargs,
                    env: P(Value::Array(P(vec![]))),
                    module: m.clone(),
                }));

                m.borrow_mut().globals[i] = P(val);
            }
            _ => (),
        };
    }

    // fix global variables
    for (i, g) in ctx.g.table.iter().enumerate() {
        match g {
            Global::Var(name) => {
                let idx = ctx.g.globals.get(&Global::Var(name.to_owned())).unwrap();

                m.borrow_mut().globals[i] = m.borrow().globals[*idx as usize].clone();
            }
            _ => (),
        }
    }
    m.borrow_mut().code = ctx.finish();
    for (hash, field) in ctx.fields.iter() {
        m.borrow_mut().fields.insert(*hash, field.clone());
    }
    m
}

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Options {
    #[structopt(name = "FILE", parse(from_os_str))]
    file: Option<PathBuf>,
}

fn main() {
    let ops = Options::from_args();
    let reader =
        Reader::from_file(ops.file.expect("Filename").as_os_str().to_str().unwrap()).unwrap();
    let mut ast = vec![];
    let mut parser = Parser::new(reader, &mut ast);
    parser.parse().unwrap();
    let ctx = compile_ast(ast);
    let code = ctx.finish();
    println!("{:#?}", ctx.g.table);

    for (i, op) in code.iter().enumerate() {
        println!("{:04}: {:?}", i, op)
    }

    let mut m = module_from_ctx(&ctx);

    let mut vm = VM::new();
    register_builtins(&mut vm);
    vm.code = ctx.finish();
    println!("{:?}", vm.interp(&mut m));
}
