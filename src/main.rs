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
use jazzc::emit_file;
pub fn module_from_ctx(ctx: &mut Context) -> P<Module> {
    let mut m = Module::new(&ctx.cur_file);
    m.globals = vec![P(Value::Null); ctx.g.table.len()];
    let m = P(m);

    for (i, g) in ctx.g.table.iter().enumerate() {
        match g {
            Global::Func(off, nargs) => {
                let func = P(Function {
                    var: FuncVar::Offset(*off as usize),
                    nargs: *nargs,
                    env: P(Value::Array(P(vec![]))),
                    module: P(Module::new("_")),
                    jit: false,
                    yield_point: 0,
                });
                func.borrow_mut().module = m.clone();
                m.borrow_mut().globals[i] = P(Value::Func(func));
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
#[structopt(name = "jazzc", version = "0.0.1")]
pub struct Options {
    #[structopt(parse(from_os_str))]
    files: Vec<PathBuf>,
    #[structopt(short = "d", long = "disassemble")]
    /// Print bytecode to stdout
    dump_op: bool,
    #[structopt(short = "v", long = "verbose")]
    /// Show more information e.g current opcode, field lists etc
    verbose: bool,
}

use std::fs::File;
use std::io::Read;

fn main() {
    let mut buff = String::new();

    let ops = Options::from_args();

    for file in ops.files.iter() {
        let mut b = String::new();

        File::open(file).unwrap().read_to_string(&mut b).unwrap();
        buff.push_str(&b);
    }
    let reader = Reader::from_string(&buff);
    let mut ast = vec![];
    let mut parser = Parser::new(reader, &mut ast);

    parser.parse().unwrap();
    let mut ctx = compile_ast(ast);

    let mut m = module_from_ctx(&mut ctx);
    if ops.verbose && m.fields.len() != 0 {
        println!("Fields:");
        for (hash, name) in m.fields.iter() {
            println!("\t{}: \t0{:x}", name, *hash as u64);
        }
        println!("");
    }

    let code = ctx.finish();
    if ops.dump_op || ops.verbose {
        println!("Byteocde:");
        for (i, op) in code.iter().enumerate() {
            println!("{:04}: {:?}", i, op)
        }
        println!("");
    }

    emit_file::compile(&mut m);
    let mut vm = VM::new();
    jazzvm::fields::init_fields();
    register_builtins(&mut vm);
    vm.code = ctx.finish();
    *jazzvm::vm::VM_THREAD.borrow_mut() = vm;

    let start = time::PreciseTime::now();
    if ops.verbose {
        unsafe { jazzvm::VERBOSE = true };
    }
    jazzvm::vm::VM_THREAD.borrow_mut().interp(&mut m);
    let end = time::PreciseTime::now();

    if ops.verbose {
        println!(
            "Execution time: {} milliseconds",
            start.to(end).num_milliseconds()
        );
    }
}
