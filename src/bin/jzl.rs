extern crate jazzlight;

use jazzlight::vm::{Frame, Machine};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "jazzlight", version = "0.0.1")]
pub struct Options {
    #[structopt(name = "FILE", parse(from_os_str))]
    file: Option<PathBuf>,
}
////cgc::generational::*;
use std::fs::File;
use std::io::Read;

fn main() {
    let ops: Options = Options::from_args();
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
        count: c,
    };

    let code = reader.read();
    let mut frame = Frame::new(&mut m);
    frame.code = jazzlight::vm::value::new_ref(code);
    jazzlight::vm::runtime::register_builtins(frame.env.clone());
    jazzlight::additional::ui::minifb_init(frame.env.clone());
    frame.execute();
    return;
}
