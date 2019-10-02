use super::*;
use crate::*;
use compiler::generator::{compile, module_from_context};
use compiler::parser::Parser;
use compiler::reader::Reader;
use value::*;

pub extern "C" fn require(_: Value, args: &[Value]) -> Result<Value, Value> {
    let filename = args[0].to_string();
    let reader = match Reader::from_file(&filename) {
        Ok(reader) => reader,
        Err(e) => return Err(Value::String(Gc::new(e.to_string()))),
    };

    let mut ast = vec![];
    let mut parser = Parser::new(reader, &mut ast);

    match parser.parse() {
        Ok(_) => (),
        Err(e) => return Err(Value::String(Gc::new(e.to_string()))),
    }

    let mut ctx = compile(ast, true);
    let module = module_from_context(&mut ctx);
    let object = run_module(module.clone());

    return Ok(object);
}

pub fn init_common() {
    let require = new_builtin_fn(require as _, 1);
    let mut state = STATE.lock();
    state
        .static_variables
        .insert(Value::String(Gc::new("require".to_owned())), require);
}
