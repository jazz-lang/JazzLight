use crate::vm::runtime::decl_fun;
use crate::vm::value::*;

decl_fun!(
    function object_create(_frame,_this) {
        let object = new_object();
        Ok(new_ref(ValueData::Object(object)))
    } 
);

decl_fun!( 
    function object_keys(_frame,_this object) {
        let object: &ValueData = &object.borrow();
        let mut keys = vec![];
        match object {
            ValueData::Object(obj) => {
                for (key,_) in obj.borrow().table.iter() {
                    keys.push(new_ref(key.clone()));
                }
            }
            _ => return Err(new_error(-1, None, "object expected"))
        }


        Ok(new_ref(ValueData::Array(new_ref(keys))))
        
    }
);
