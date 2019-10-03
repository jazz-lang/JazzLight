use crate::*;

use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Number(f64),
    String(Gc<String>),
    Object(Gc<Object>),
    Bool(bool),
}

/*
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
}*/

pub fn strcpy(x: Gc<String>) -> Gc<String> {
    Gc::new(x.get().to_owned())
}

impl Value {
    pub fn to_number(&self) -> f64 {
        match self {
            Value::Null => 0.0,
            Value::Number(x) => *x,
            Value::Bool(x) => *x as i32 as f64,
            Value::String(x) => x.get().parse().unwrap(),
            Value::Object(object) => match &object.get().kind {
                ObjectKind::Number(x) => *x,
                _ => std::f64::NAN,
            },
        }
    }

    pub fn unwrap_object(&self) -> Gc<Object> {
        match self {
            Value::Object(obj) => obj.clone(),
            _ => crate::unreachable(),
        }
    }

    pub fn to_object(&self) -> Result<Value, Value> {
        match self {
            Value::Null => Err(Value::String(Gc::new(
                "cannot convert null to object".to_owned(),
            ))),
            Value::Object(object) => Ok(Value::Object(object.clone())),
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
                kind: ObjectKind::String(string.clone()),
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
            Value::Object(object) => match &object.get().kind {
                ObjectKind::String(x) => write!(f, "{}", x),
                ObjectKind::Array(array) => {
                    let mut fmt = String::new();
                    fmt.push('[');
                    for (idx, value) in array.get().iter().enumerate() {
                        fmt.push_str(&value.to_string());

                        if idx < array.get().len() - 1 {
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
                    let object = object.get();
                    let properties = object.properties.get();
                    for (i, property) in properties.iter().enumerate() {
                        if property.enumerated {
                            let key = property.key.to_string();
                            let value = property.value.to_string();
                            fmt.push_str(&format!("  {} => {}", key, value));
                            if i < properties.len() - 1 {
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
                for property in object.get().properties.get().iter() {
                    let property: &Property = property;
                    property.key.hash(state);
                    property.value.hash(state);
                    property.get.hash(state);
                    property.set.hash(state);
                    property.private.hash(state);
                    property.enumerated.hash(state);
                }
                object.get().properties.get().len().hash(state);
            }
            Value::Bool(x) => {
                4.hash(state);
                x.hash(state);
            }
        }
    }
}

#[derive(Clone, Debug)]
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
                Value::Object(y) => x.get().properties.ref_eq(&y.get().properties),
                _ => false,
            },
            Value::String(x) => match other {
                Value::String(y) => x == y,
                _ => false,
            },
        }
    }
}

impl Eq for Value {}

#[derive(Clone, Debug)]
pub enum ObjectKind {
    Array(Gc<Vec<Value>>),
    Number(f64),
    Bool(bool),
    String(Gc<String>),
    Function(Gc<Function>),
    Thread(usize /* thread id */),
    Ordinary,
}

#[derive(Clone, Debug)]
pub struct Object {
    pub kind: ObjectKind,
    pub proto: Option<Gc<Object>>,
    pub properties: Gc<Vec<Property>>,
}
/*
unsafe impl GcObject for Object {
    fn references(&self) -> Vec<Gc<dyn GcObject>> {
        let mut v: Vec<Gc<dyn GcObject>> = vec![];
        match &self.kind {
            ObjectKind::Array(array) => v.push(*array),
            ObjectKind::Function(function) => v.push(*function),
            ObjectKind::String(string) => v.push(*string),
            _ => (),
        }
        v.push(self.properties);
        match self.proto {
            Some(value) => v.push(value),
            _ => (),
        }
        v
    }
}
*/
impl Object {
    pub fn get_property(&self, key: Value) -> Option<Property> {
        match &self.kind {
            ObjectKind::String(s) => match key {
                Value::String(x) => {
                    let key_ = x.get();
                    if (&*key_).eq("length") {
                        let mut property = Property::new();
                        property.key = Value::String(x.clone());
                        property.value = Value::Number(s.get().len() as _);
                        return Some(property);
                    } else {
                        let key = Value::String(x.clone());
                        for property in self.properties.get().iter() {
                            if property.key == key {
                                return Some(property.clone());
                            }
                        }
                        match &self.proto {
                            Some(proto) => return proto.get().get_property(key),
                            None => return None,
                        }
                    }
                }
                _ => return None,
            },
            ObjectKind::Function(func) => match key {
                Value::String(x) => {
                    let key_ = x.get();
                    if (&*key_).eq("prototype") {
                        drop(key_);
                        let mut property = Property::new();
                        property.key = Value::String(x);
                        property.value = func.get().prototype.clone();
                        return Some(property);
                    } else {
                        let key = Value::String(x.clone());
                        for property in self.properties.get().iter() {
                            if property.key == key {
                                return Some(property.clone());
                            }
                        }
                        match &self.proto {
                            Some(proto) => return proto.get().get_property(key),
                            None => return None,
                        }
                    }
                }
                _ => (),
            },
            ObjectKind::Array(array) => match key {
                Value::String(x) => {
                    let key_ = x.get();
                    if (&*key_).eq("prototype") {
                        drop(key_);
                        let mut property = Property::new();
                        property.key = Value::String(x.clone());
                        property.value = Value::Number(array.get().len() as _);
                        return Some(property);
                    } else {
                        return self
                            .proto
                            .as_ref()
                            .unwrap()
                            .get()
                            .get_property(Value::String(x.clone()));
                    }
                }
                Value::Number(x) => {
                    if x >= array.get().len() as f64 {
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
        match &self.proto {
            Some(proto) => return proto.get().get_property(key),
            None => return None,
        }
    }
    pub fn set_property(&self, key: Value, value: Value) {
        match &self.kind {
            ObjectKind::Array(array) => match key {
                Value::Number(x) => {
                    if x >= array.get().len() as f64 {
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

#[derive(Clone, Debug)]
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
