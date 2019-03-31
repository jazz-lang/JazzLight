use jazzvm::vm::VM;
use jazzvm::P;
use jazzvm::value::Value;

#[no_mangle]
pub extern "C" fn test_func(_: &mut VM,_: Vec<P<Value>>) -> P<Value> {
    println!("Called function from dynamic library!");
    return P(Value::Null);
}