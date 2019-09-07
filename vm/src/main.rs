extern crate jazzlight;

use jazzlight::interp::*;
use jazzlight::*;


use jazzlight::reader::BytecodeReader;
use std::io::Cursor;
use jazzlight::value::Value;


fn main() {
    let file = std::env::args().nth(1);
    if file.is_none() {
        eprintln!("Please select JazzLight bytecode file");
        std::process::exit(1);
    }
    let file = file.unwrap();

    let contents = std::fs::read(&file);
    match contents {
        Ok(contents) => {
            let mut reader = BytecodeReader {
                bytes: Cursor::new(&contents)
            };
            let m = reader.read_module();
            let mut vm = VM.lock();
            vm.save_state_exit();
            match vm.interp(m) {
                Value::Int(x) => std::process::exit(x as _),
                _ => ()
            }
        }
        Err(e) => {
            eprintln!("{}",e);
            std::process::exit(1);
        }
    }
}
