use crate::*;
use pgc::*;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Number(f64),
    String(Gc<String>),
    Object(Gc<Object>),
    Bool(bool),
}

unsafe impl GcObject for Value {
    fn references(&self) -> Vec<Gc<dyn GcObject>> {
        let mut v: Vec<Gc<dyn GcObject>> = vec![];
        match self {
            Value::String(s) => v.push(*s),
            Value::Object(o) => v.push(*o),
            _ => (),
        };
        v
    }
}

pub fn strcpy(x: Gc<String>) -> Gc<String> {
    Rooted::new(x.get().to_owned()).inner()
}

impl Value {
    pub fn unwrap_object(&self) -> Gc<Object> {
        match self {
            Value::Object(obj) => *obj,
            _ => crate::unreachable(),
        }
    }

    pub fn to_object(&self) -> Result<Value, Value> {
        match self {
            Value::Null => Err(Value::String(Gc::new(
                "cannot convert null to object".to_owned(),
            ))),
            Value::Object(object) => Ok(Value::Object(*object)),
            Value::Number(n) => Ok(Value::Object(Gc::new(Object {
                kind: ObjectKind::Number(*n),
                properties: Gc::new(vec![]),
                proto: Some(
                    match STATE
                        .lock()
                        .static_variables
                        .get(&Value::String(Gc::new("Number".to_owned())))
                        .cloned()
                        .unwrap()
                        .to_object()?
                    {
                        Value::Object(obj) => obj,
                        _ => crate::unreachable(),
                    },
                ),
            }))),
            Value::Bool(x) => Ok(Value::Object(Gc::new(Object {
                kind: ObjectKind::Bool(*x),
                properties: Gc::new(vec![]),
                proto: Some(
                    match STATE
                        .lock()
                        .static_variables
                        .get(&Value::String(Gc::new("Boolean".to_owned())))
                        .cloned()
                        .unwrap()
                        .to_object()?
                    {
                        Value::Object(obj) => obj,
                        _ => crate::unreachable(),
                    },
                ),
            }))),
            Value::String(string) => Ok(Value::Object(Gc::new(Object {
                kind: ObjectKind::String(*string),
                properties: Gc::new(vec![]),
                proto: Some(
                    match STATE
                        .lock()
                        .static_variables
                        .get(&Value::String(Gc::new("String".to_owned())))
                        .cloned()
                        .unwrap()
                        .to_object()?
                    {
                        Value::Object(obj) => obj,
                        _ => crate::unreachable(),
                    },
                ),
            }))),
        }
    }
}

use std::fmt;

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(x) => write!(f, "{}", x),
            Value::Number(x) => write!(f, "{}", x),
            Value::String(s) => write!(f, "{}", s),
            Value::Object(object) => match &object.kind {
                ObjectKind::String(x) => write!(f, "{}", x),
                ObjectKind::Array(array) => {
                    let mut fmt = String::new();
                    fmt.push('[');
                    for (idx, value) in array.iter().enumerate() {
                        fmt.push_str(&value.to_string());

                        if idx < array.len() - 1 {
                            fmt.push(',');
                        }
                    }

                    fmt.push(']');
                    write!(f, "{}", fmt)
                }
                ObjectKind::Thread(id) => write!(f, "<thread 0x{:x}>", id),
                ObjectKind::Number(x) => write!(f, "{}", x),
                ObjectKind::Bool(x) => write!(f, "{}", x),
                ObjectKind::Function(_) => write!(f, "<function>"),
                ObjectKind::Ordinary => {
                    let mut fmt = String::new();
                    fmt.push_str("{\n");
                    for (i, property) in object.properties.iter().enumerate() {
                        if property.enumerated {
                            let key = property.key.to_string();
                            let value = property.value.to_string();
                            fmt.push_str(&format!("  {} => {}", key, value));
                            if i < object.properties.len() - 1 {
                                fmt.push(',');
                            }
                            fmt.push('\n');
                        }
                    }
                    fmt.push('}');

                    write!(f, "{}", fmt)
                }
            },
        }
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Value::Null => 0.hash(state),
            Value::Number(x) => {
                1.hash(state);
                x.to_bits().hash(state);
            }
            Value::String(x) => {
                2.hash(state);
                x.get().hash(state);
            }
            Value::Object(object) => {
                3.hash(state);
                for property in object.properties.iter() {
                    let property: &Property = property;
                    property.key.hash(state);
                    property.value.hash(state);
                    property.get.hash(state);
                    property.set.hash(state);
                    property.private.hash(state);
                    property.enumerated.hash(state);
                }
                object.properties.len().hash(state);
            }
            Value::Bool(x) => {
                4.hash(state);
                x.hash(state);
            }
        }
    }
}

#[derive(GcObject, Clone, Debug)]
pub struct Function {
    pub module: Option<Gc<Module>>,
    pub addr: usize,
    pub is_native: bool,
    pub env: Value,
    pub prototype: Value,
    pub argc: i32,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Value::Null => match other {
                Value::Null => true,
                _ => false,
            },
            Value::Bool(x) => match other {
                Value::Bool(y) => x == y,
                _ => false,
            },
            Value::Number(x) => match other {
                Value::Number(y) => x == y,
                _ => false,
            },
            Value::Object(x) => match other {
                Value::Object(y) => x.get().properties.ref_eq(y.properties),
                _ => false,
            },
            Value::String(x) => match other {
                Value::String(y) => x.get() == y.get(),
                _ => false,
            },
        }
    }
}

impl Eq for Value {}

#[derive(GcObject, Clone, Debug)]
pub enum ObjectKind {
    Array(Gc<Vec<Value>>),
    Number(f64),
    Bool(bool),
    String(Gc<String>),
    Function(Gc<Function>),
    Thread(usize /* thread id */),
    Ordinary,
}

#[derive(GcObject, Clone, Debug)]
pub struct Object {
    pub kind: ObjectKind,
    pub proto: Option<Gc<Object>>,
    pub properties: Gc<Vec<Property>>,
}

impl Object {
    pub fn get_property(&self, key: Value) -> Option<Property> {
        match self.kind {
            ObjectKind::String(s) => match key {
                Value::String(x) => {
                    let key_: &str = x.get();
                    if key_ == "length" {
                        let mut property = Property::new();
                        property.key = key;
                        property.value = Value::Number(s.len() as _);
                        return Some(property);
                    } else {
                        for property in self.properties.get().iter() {
                            if property.key == key {
                                return Some(property.clone());
                            }
                        }
                        match self.proto {
                            Some(proto) => return proto.get_property(key),
                            None => return None,
                        }
                    }
                }
                _ => return None,
            },
            ObjectKind::Function(func) => match key {
                Value::String(x) => {
                    let key_: &str = x.get();
                    if key_ == "prototype" {
                        let mut property = Property::new();
                        property.key = key;
                        property.value = func.get().prototype.clone();
                        return Some(property);
                    } else {
                        for property in self.properties.get().iter() {
                            if property.key == key {
                                return Some(property.clone());
                            }
                        }
                        match self.proto {
                            Some(proto) => return proto.get_property(key),
                            None => return None,
                        }
                    }
                }
                _ => (),
            },
            ObjectKind::Array(array) => match key {
                Value::String(x) => {
                    let key_: &str = x.get();
                    if key_ == "length" {
                        let mut property = Property::new();
                        property.key = key;
                        property.value = Value::Number(array.len() as _);
                        return Some(property);
                    } else {
                        return self.proto.as_ref().unwrap().get_property(key);
                    }
                }
                Value::Number(x) => {
                    if x >= array.len() as f64 {
                        for _ in 0..=x as usize {
                            array.get_mut().push(Value::Null);
                        }
                    }
                    let mut property = Property::new();
                    property.key = Value::Number(x);
                    property.value = array.get()[x as usize].clone();
                    return Some(property);
                }
                _ => (),
            },
            _ => (),
        }
        for property in self.properties.get().iter() {
            if property.key == key {
                return Some(property.clone());
            }
        }
        match self.proto {
            Some(proto) => return proto.get_property(key),
            None => return None,
        }
    }
    pub fn set_property(&self, key: Value, value: Value) {
        match &self.kind {
            ObjectKind::Array(array) => match key {
                Value::Number(x) => {
                    if x >= array.len() as f64 {
                        for _ in 0..=x as usize {
                            array.get_mut().push(Value::Null);
                        }
                    }
                    array.get_mut()[x as usize] = value;
                    return;
                }
                _ => (),
            },
            _ => (),
        }
        for property in self.properties.get_mut().iter_mut() {
            if property.key == key {
                property.value = value;
                return;
            }
        }
        let mut prop = Property::new();
        prop.key = key;
        prop.value = value;
        self.properties.get_mut().push(prop);
    }
}

#[derive(GcObject, Clone, Debug)]
pub struct Property {
    pub key: Value,
    pub value: Value,
    pub enumerated: bool,
    pub private: bool,
    pub get: Option<Value>,
    pub set: Option<Value>,
}

impl Property {
    pub const fn new() -> Self {
        Self {
            key: Value::Null,
            value: Value::Null,
            enumerated: true,
            private: false,
            get: None,
            set: None,
        }
    }
}
