extern crate jazz;
extern crate structopt;

use jazz::compiler::Compiler;
use jazz::parser::Parser;
use jazz::reader::Reader;

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
        let mut string = String::new();
        string.push_str(
                        "
                    
                            function pow(x,y) {
    if y == 0 {
        return 1
    } else if y % 2 == 0 {
        return pow(x,y / 2) * pow(x,y / 2)
    } else {
        return x * pow(x,y / 2) * pow(x,y / 2)
    }
}

function sqrt(x) {
    builtin_sqrt(x)
}
function sin(x) {
    builtin_sin(x)
}
",
        );
        let mut buff = String::new();
        use std::io::Read;
        let start = PreciseTime::now();
        std::fs::File::open(path).unwrap()
                                 .read_to_string(&mut buff)
                                 .unwrap();
        string.push_str(&buff);
        let reader = Reader::from_string(&string);

        let mut ast = vec![];
        let mut parser = Parser::new(reader, &mut ast);
        parser.parse().unwrap();
        let mut vm = VirtualMachine::new();

        let mut compiler = Compiler::new(&mut vm, "__main__".into());

        compiler.compile_ast(ast);

        let f = compiler.globals.get("main").unwrap();

        compiler.vm.run_function(*f);
        let end = PreciseTime::now();

        println!("Compiling and execution time {} ms",
                 start.to(end).num_milliseconds());
    }
    else
    {
        panic!("You should enter file path");
    }
}
