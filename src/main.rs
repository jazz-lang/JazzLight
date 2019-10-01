extern crate vmm;
use std::path::PathBuf;
use structopt::StructOpt;

use vmm::*;

use compiler::generator::*;
use compiler::parser::Parser;
use compiler::reader::Reader;
use vmm::compiler;

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
    run: bool,
}

fn main() {
    pgc::enable_gc_stats();
    init_builtins();

    let ops = Options::from_args();
    if ops.file.is_none() {
        eprintln!("Please select file");
        std::process::exit(1);
    }
    let string = ops.file.unwrap().to_str().unwrap().to_owned();
    let r = match Reader::from_file(&string) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to open file '{}': {}", string, e);
            std::process::exit(1);
        }
    };
    let mut ast = vec![];
    let mut parser = Parser::new(r, &mut ast);
    match parser.parse() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
    let mut ctx = compile(ast);
    let m = module_from_context(&mut ctx);

    if ops.dump_op || ops.verbose {
        println!("Byteocde:");
        for (i, op) in m.get().code.iter().enumerate() {
            println!("{:04}: {:?}", i, op)
        }
        println!();
    }

    println!("{}", run_module(m.inner()));
}
