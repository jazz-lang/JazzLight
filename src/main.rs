extern crate jazzlight;

use jazzlight::compiler::*;
use jazzlight::parser::Parser;
use jazzlight::reader::Reader;
use jazzlight::vm::{Frame, Machine};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "jazzlight", version = "0.0.1")]
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
    #[structopt(long = "run", help = "run bytecode file")]
    run: bool,
    #[structopt(short = "c", long = "compile", help = "emit bytecode file")]
    compile: bool,
    #[structopt(short = "o", parse(from_os_str))]
    output: Option<PathBuf>,
}
use cgc::generational::*;
use jazzlight::vm::runtime::register_builtins;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

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
    if ops.run {
        let string = ops.file.unwrap().to_str().unwrap().to_owned();
        let mut file = File::open(&string).unwrap();
        let mut m = Machine::new();
        let mut code = vec![];

        file.read_to_end(&mut code).unwrap();
        let c = code.len();
        let mut reader = jazzlight::decoder::BytecodeReader {
            machine: &mut m,
            bytecode: std::io::Cursor::new(code),
            pc: 0,
            count: c
        };

        let code = reader.read();
        let mut frame = Frame::new(&mut m);
        frame.code = jazzlight::vm::value::new_ref(code);
        jazzlight::vm::runtime::register_builtins(frame.env.clone());

        frame.execute();
        return;
    }
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
    c.compile_ast(&ast, true);
    if ops.dump_op {
        for (i, op) in c.frame.code.borrow().iter().enumerate() {
            println!("{:04}: {:?}", i, op);
        }
    }
    let mut writer = jazzlight::writer::Writer {
        machine: c.frame.m,
        code: c.frame.code.borrow().clone(),
        bytecode: vec![],
        names: jazzlight::map::LinkedHashMap::new(),
    };
    writer.emit();

    let path = match ops.output {
        Some(path) => path.to_str().unwrap().to_owned(),
        None => {
            let p = std::path::Path::new(&string);
            match p.file_name() {
                Some(file_name) => {
                    let p = std::path::Path::new(&file_name);
                    match p.file_stem() {
                        Some(name) => name.to_str().unwrap().to_owned(),
                        _ => file_name.to_str().unwrap().to_owned(),
                    }
                }
                None => {
                    eprintln!("Cannot get file name");
                    std::process::exit(1);
                }
            }
        }
    };
    if !std::path::Path::new(&path).exists() {
        File::create(&path).unwrap();
    }
    let mut file = OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len(0).unwrap();
    file.write_all(&writer.bytecode).unwrap();

    //*STOP.lock() = true;
    //thread.join().unwrap();
    //gc_collect_not_par();
}

fn repl() {
    use rustyline::{error::ReadlineError, Editor};
    let mut rl = Editor::<()>::new();
    //let mut code = String::new();
    let mut m = Machine::new();
    let mut f = Frame::new(&mut m);
    register_builtins(f.env.clone());
    let mut compiler = Compiler::new(&mut f);
    let mut last_loc = 0;
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

                let reader = Reader::from_string(s);
                let mut ast = vec![];
                let mut p = Parser::new(reader, &mut ast);
                match p.parse() {
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                }
                compiler.compile_ast(&ast, false);
                if last_loc != 0 {
                    compiler
                        .frame
                        .code
                        .borrow_mut()
                        .insert(0, jazzlight::vm::opcodes::Opcode::Jump(last_loc + 1));
                }
                compiler.frame.execute();
                last_loc = compiler.frame.code.borrow().len() as u32;
            }

            Err(ReadlineError::Interrupted) => {
                println!("Use Ctrl + D or type 'exit' to leave repl ");
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(e) => {
                eprintln!("{}", e);
                break;
            }
        }
    }
}
