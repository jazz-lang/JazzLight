use jazzlightc::reader::Reader;

use jazzlightc::codegen::{compile, module_from_context};
use jazzlightc::parser::Parser;
use std::path::PathBuf;
use structopt::StructOpt;
use jazzlight::writer::BytecodeWriter;

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
    let ops = Options::from_args();
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
        for (i, op) in m.borrow().code.iter().enumerate() {
            println!("{:04}: {:?}", i, op)
        }
        println!();
    }
    let mut w = BytecodeWriter {
        bytecode: vec![]
    };
    w.write_module(m);
    let path = std::path::Path::new(&string);
    let stem = path.file_stem().unwrap();
    let path = format!("{}.j",stem.to_str().unwrap());
    if std::path::Path::new(&path).exists() {
        let mut f = std::fs::OpenOptions::new().write(true).open(&path);
        f.unwrap().set_len(0).unwrap();
    }
    std::fs::write(&path,&w.bytecode).unwrap();
}
