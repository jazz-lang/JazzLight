extern crate jazzc;

use jazzc::compiler::*;
use jazzc::parser::Parser;
use jazzc::reader::Reader;
use jazzc::vm::codegen::basicblock;
use jazzc::vm::Machine;
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
    /// Show more information e.g current opcodes, field lists etc
    verbose: bool,
    #[structopt(long = "optimize")]
    /// Try to optimize bytecode
    optimize: bool,
    #[structopt(long = "run")]
    run: bool,
}

fn main() {
    let ops: Options = Options::from_args();
    if ops.file.is_none() {
        eprintln!("Expected file path as input");
        std::process::exit(-1);
    }
    let string = ops.file.unwrap().to_str().unwrap().to_owned();
    let reader = Reader::from_file(&string).unwrap();
    let mut ast = vec![];
    let mut parser = Parser::new(reader, &mut ast);

    match parser.parse() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
    let mut m = Machine::new();
    let mut c = Compiler::new(&mut m);
    c.compile_ast(&ast);
    if ops.dump_op {
        for (i, op) in c.frame.code.borrow().iter().enumerate() {
            println!("{:04}: {:?}", i, op);
        }
    }

    let code_with_blocks = basicblock::translate_to_blocks(c.frame.code.borrow().clone());
    for (i,bb) in code_with_blocks.iter().enumerate() {
        println!("BBlock #{}",i);
        for (j,x) in bb.opcodes.iter().enumerate() {
            println!("  {:02}: {:?}",j,x);
        }
    }
    c.frame.execute();
}
