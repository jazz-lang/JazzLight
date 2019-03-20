extern crate jazz;
extern crate structopt;

use jazz::compiler::Compiler;
use jazz::parser::Parser;
use jazz::reader::Reader;
use jazz::runtime::builtins;
use jazzvm::vm::VirtualMachine;
use std::path::PathBuf;
use structopt::StructOpt;
use time::PreciseTime;
#[derive(StructOpt, Debug)]
pub struct Options
{
    #[structopt(name = "FILE", parse(from_os_str))]
    file: Option<PathBuf>,
}

fn main()
{
    let ops = Options::from_args();
    if let Some(path) = ops.file
    {
        let path: PathBuf = path;
        let reader = Reader::from_file(path.as_os_str().to_str().unwrap()).unwrap();
        let mut ast = vec![];
        let mut parser = Parser::new(reader, &mut ast);
        parser.parse().unwrap();
        let mut vm = VirtualMachine::new();

        let mut compiler = Compiler::new(&mut vm, "__main__".into());
        builtins(&mut compiler);
        compiler.compile_ast(ast);

        let f = compiler.globals.get("main").unwrap();
        let start = PreciseTime::now();
        let result = compiler.vm.run_function(*f);
        let end = PreciseTime::now();

        println!("RESULT: {:?} in {} ms",
                 result,
                 start.to(end).num_milliseconds());
    }
    else
    {
        panic!("You should enter file path");
    }
}
