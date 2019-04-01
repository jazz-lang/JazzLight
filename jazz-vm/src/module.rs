use crate::{value::Value, P};

#[derive(Clone, Debug)]
pub struct Module {
    pub name: P<Value>,
    pub globals: Vec<P<Value>>,
    pub loader: P<Value>,
    pub exports: P<Value>,
    pub code: Vec<crate::opcode::Opcode>,
    pub fields: fnv::FnvHashMap<u64, String>,
}

impl Module {
    pub fn new(name: &str) -> Module {
        Module {
            name: P(Value::Str(name.to_owned())),
            globals: vec![],
            loader: P(Value::Null),
            exports: P(Value::Null),
            code: vec![],
            fields: fnv::FnvHashMap::default(),
        }
    }
}
