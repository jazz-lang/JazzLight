
extern crate jazzvm;

use jazzvm::vm::*;
use jazzvm::module::*;
use jazzvm::builtins::*;
use jazzvm::fields::*;

use std::env::args;

fn main() {
    let args = args().collect::<Vec<String>>();

    let mut vm = VM::new();
    init_fields();
    register_builtins(&mut vm);
    
    use std::io::Read;
    let mut f = std::fs::File::open(&args[1]).unwrap();
    let mut buf = vec![];
    f.read_to_end(&mut buf).unwrap();
    let reader = Reader {  
        code: buf,
        pc: 0,
    };  
    use std::path::Path;
    let p = Path::new(&args[1]);
    let f = p.file_stem().unwrap().to_str().unwrap().to_owned();

    let mut module = read_module(reader, &f);
    
    vm.code = module.code.clone();
    *VM_THREAD.borrow_mut() = vm;
    VM_THREAD.borrow_mut().interp(&mut module);
}