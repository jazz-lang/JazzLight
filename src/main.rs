extern crate jazzc;

use jazzc::compile::compile_ast;
use jazzc::compile::Context;
use jazzc::compile::Global;
use jazzc::emit_file;
use jazzc::parser::Parser;
use jazzc::reader::Reader;
use jazzvm::builtins::register_builtins;
use jazzvm::module::Module;
use jazzvm::value::*;
use jazzvm::vm::VM;
use jazzvm::P;
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
    #[structopt(name = "FILE", parse(from_os_str))]
    file: Option<PathBuf>,
    #[structopt(short = "d", long = "disassemble")]
    /// Print bytecode to stdout
    dump_op: bool,
    #[structopt(short = "v", long = "verbose")]
    /// Show more information e.g current opcode, field lists etc
    verbose: bool,
    #[structopt(long = "run")]
    /// Instead of emitting bytecode file directly run code
    run: bool,
}

fn main() {
    let ops = Options::from_args();
    let string = ops.file.unwrap().to_str().unwrap().to_owned();
    let reader = Reader::from_file(&string).unwrap();
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

    if ops.run {
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
    } else {
        m.borrow_mut().code = ctx.finish();
        let code = emit_file::compile(&mut m).expect("Error");
        use std::io::Write;
        let f = std::path::Path::new(&string);
        let f = f.file_stem().unwrap();
        let mut f = f.to_str().unwrap().to_owned();
        f.push('.');
        f.push('j');
        std::fs::File::create(&f).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(f)
            .expect("Error");

        file.write(&code).unwrap();
    }
}
