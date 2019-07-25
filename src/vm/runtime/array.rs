use crate::token::Position;
use crate::vm::runtime::*;
use crate::vm::{nil, Frame};
decl_fun!( 
    function array_push(_frame,this push_val) {
        let array: &ValueData = &this.borrow();
        match array {
            ValueData::Array(array) => array.borrow_mut().push(push_val),
            _ => return Err(new_error(-1, None,"Array.push: array expected"))
        };

        Ok(nil())
    }
);

decl_fun!(
    function array_pop(_frame,this) {
        let array: &ValueData = &this.borrow();
        match array {
            ValueData::Array(array) => return Ok(array.borrow_mut().pop().unwrap_or(new_ref(ValueData::Undefined))),
            _ => return Err(new_error(-1, None, "Array.pop: array expected"))
        }
    }
);

decl_fun!(
    function array_sort(_frame,_this) {
        unimplemented!()
    }
);

pub fn array_object() -> Ref<Object> {
    let array_proto = new_object();
    array_proto.borrow_mut().set("push", new_exfunc(array_push));
    array_proto.borrow_mut().set("pop", new_exfunc(array_pop));
    array_proto.borrow_mut().set("sort", new_exfunc(array_sort));
    array_proto
}

use crate::ngc::gc_add_root;

pub fn register_array(f: &mut Frame<'_>) {
    let array_proto = array_object();
    let global = new_ref(ValueData::Object(array_proto));
    //gc_add_root(global.gc());
    declare_var(
        &f.env,
        "Array",
        global
        ,
        &Position::new(0, 0),
    )
    .unwrap();
}
