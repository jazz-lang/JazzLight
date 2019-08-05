extern crate jazzc;

use jazzc::compiler::*;
use jazzc::parser::Parser;
use jazzc::reader::Reader;
use jazzc::vm::{Machine, Frame};
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

use cgc::generational::*;
use jazzc::vm::value::new_object;
use jazzc::vm::runtime::register_builtins;

fn main() {
    lazy_static::lazy_static! {
        pub static ref STOP: parking_lot::Mutex<bool> = parking_lot::Mutex::new(false);
    }
    /*let thread = std::thread::Builder::new().name("gc_thread".to_owned()).spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            gc_collect();
            if *STOP.lock() {
                break;
            }
        }
    }).unwrap();
    */

    let ops: Options = Options::from_args();
    if ops.file.is_none() {
        repl();
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
    let mut frame = Frame::new(&mut m);
    let mut c = Compiler::new(&mut frame);
    c.compile_ast(&ast,true);
    if ops.dump_op {
        for (i, op) in c.frame.code.borrow().iter().enumerate() {
            println!("{:04}: {:?}", i, op);
        }
    }

    c.frame.execute();
    //*STOP.lock() = true;
    //thread.join().unwrap();
    //gc_collect_not_par();
}



fn repl() {
    use rustyline::{Editor,error::ReadlineError};
    let mut rl = Editor::<()>::new();
    let mut code = String::new();
    let mut m = Machine::new();
    let mut f = Frame::new(&mut m);
    register_builtins(f.env.clone());
    let mut compiler = Compiler::new(&mut f);
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                
                rl.add_history_entry(line.clone());
                let s: &str = &line;
                if s == "exit" {
                    gc_collect_not_par();
                    std::process::exit(0);

                }
                code.push_str(s);
                let reader = Reader::from_string(&code);
                let mut ast = vec![];
                let mut p = Parser::new(reader,&mut ast);
                match p.parse() {
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("{}",e);
                        std::process::exit(1);
                    }
                }
                //gc_rmroot(compiler.frame.env.gc());
                compiler.frame.env = new_object();
                //gc_add_root(compiler.frame.env.gc());
                compiler.compile_ast(&ast,true);
                compiler.frame.execute();
                //gc_collect_not_par();
            }

            Err(ReadlineError::Interrupted) => {
                println!("Use Ctrl + D or type 'exit' to leave repl ");
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(e) => {
                eprintln!("{}",e);
                break;
            }

        }
    }


}