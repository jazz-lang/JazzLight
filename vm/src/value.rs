use crate::*;
use hashlink::LinkedHashMap;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Ref<String>),
    Array(Ref<Vec<Value>>),
    Object(Ref<Object>),
    Function(Ref<Function>),
    Char(char),
    User(Ref<dyn UserKind>),
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum ValTag {
    Null,
    Bool,
    Int,
    Float,
    Str,
    Array,
    Object,
    Func,
    Char,
    User(&'static str),
}

impl Value {
    pub fn to_bool(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(x) => *x,
            Value::Int(x) => {
                if *x == 0 {
                    false
                } else {
                    true
                }
            }
            Value::Float(x) => {
                if *x == 0.0 {
                    false
                } else {
                    true
                }
            }
            _ => true,
        }
    }

    pub fn to_object(&self) -> Option<Ref<Object>> {
        match self {
            Value::Object(obj) => return Some(obj.clone()),
            _ => None,
        }
    }

    pub fn to_array(&self) -> Option<Ref<Vec<Value>>> {
        match self {
            Value::Array(array) => return Some(array.clone()),
            _ => None,
        }
    }
    pub fn to_int(&self) -> Option<i64> {
        match self {
            Value::Int(x) => Some(*x),
            Value::Float(x) => Some(*x as i64),
            _ => None,
        }
    }

    pub fn to_float(&self) -> Option<f64> {
        match self {
            Value::Int(x) => Some(*x as f64),
            Value::Float(x) => Some(*x),
            _ => None,
        }
    }

    pub fn tag(&self) -> ValTag {
        match self {
            Value::Int(_) => ValTag::Int,
            Value::Float(_) => ValTag::Float,
            Value::Null => ValTag::Null,
            Value::Object(_) => ValTag::Object,
            Value::Array(_) => ValTag::Array,
            Value::String(_) => ValTag::Str,
            Value::Function(_) => ValTag::Func,
            Value::Bool(_) => ValTag::Bool,
            Value::Char(_) => ValTag::Char,
            Value::User(x) => ValTag::User(x.borrow().get_kind()),
        }
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Value::Null => {
                0.hash(state);
            }
            Value::Int(x) => {
                1.hash(state);
                x.hash(state);
            }
            Value::Float(x) => {
                2.hash(state);
                x.to_bits().hash(state);
            }
            Value::String(s) => {
                3.hash(state);
                s.borrow().hash(state);
            }
            Value::Array(array) => {
                4.hash(state);
                array.borrow().hash(state);
            }
            Value::Object(object) => {
                5.hash(state);
                object.borrow().hash(state);
            }
            Value::Bool(x) => {
                6.hash(state);
                x.hash(state);
            }
            Value::Char(x) => {
                7.hash(state);
                x.hash(state);
            }
            _ => (),
        }
    }
}

use std::fmt;
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(x) => write!(f, "{}", x),
            Value::Float(x) => write!(f, "{}", x),
            Value::Array(array) => {
                let mut fmt = String::new();
                fmt.push('[');
                for (idx, value) in array.borrow().iter().enumerate() {
                    fmt.push_str(&value.to_string());

                    if idx < array.borrow().len() - 1 {
                        fmt.push(',');
                    }
                }

                fmt.push(']');
                write!(f, "{}", fmt)
            }
            Value::Char(x) => write!(f, "{}", x),
            Value::Object(object) => {
                let mut fmt = String::new();
                fmt.push_str("{\n");
                for (i, (key, val)) in object.borrow().table.iter().enumerate() {
                    let key = key.to_string();
                    let value = val.to_string();
                    fmt.push_str(&format!("  {} => {}", key, value));
                    if i < object.borrow().table.len() - 1 {
                        fmt.push(',');
                    }
                    fmt.push('\n');
                }
                fmt.push('}');
                write!(f, "{}", fmt)
            }
            Value::Function(func) => {
                if func.borrow().native {
                    write!(f, "<function {:x}>", func.borrow().address)
                } else {
                    write!(f, "<function at {:x}>", func.borrow().address)
                }
            }
            Value::User(x) => write!(f, "{}", x.borrow()),
            Value::Null => write!(f, "null"),
            Value::String(s) => write!(f, "{}", *s.borrow()),
            Value::Bool(x) => write!(f, "{}", x),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Value::Bool(x) => match other {
                Value::Bool(y) => x == y,
                _ => false,
            },
            Value::Int(x) => match other {
                Value::Int(y) => x == y,
                Value::Float(y) => *x == *y as i64,
                _ => false,
            },
            Value::Float(x) => match other {
                Value::Int(y) => *x == *y as f64,
                Value::Float(y) => x == y,
                _ => false,
            },
            Value::Char(x) => match other {
                Value::Int(y) => *x as u32 == *y as u32,
                Value::Char(y) => *x == *y,
                _ => false,
            },
            Value::String(s) => match other {
                Value::String(s2) => *s.borrow() == *s2.borrow(),
                _ => false,
            },
            Value::Array(x) => match other {
                Value::Array(y) => *x.borrow() == *y.borrow(),
                _ => false,
            },
            Value::Null => match other {
                Value::Null => true,
                _ => false,
            },
            Value::Object(x) => match other {
                Value::Object(y) => {
                    for ((key1, val1), (key2, val2)) in
                        x.borrow().table.iter().zip(y.borrow().table.iter())
                    {
                        if (key1 != key2) || (val2 != val1) {
                            return false;
                        }
                    }
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
}

impl Eq for Value {}

pub struct Object {
    pub prototype: Option<Ref<Object>>,
    pub table: LinkedHashMap<Value, Value>,
}

impl Object {
    pub fn get(&self, value: Value) -> Option<Value> {
        match self.table.get(&value) {
            Some(value) => Some(value.clone()),
            None => match &self.prototype {
                Some(proto) => proto.borrow().get(value),
                None => None,
            },
        }
    }

    pub fn set(&mut self, key: Value, value: Value) {
        self.table.insert(key, value);
    }
}

impl Hash for Object {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (key, val) in self.table.iter() {
            key.hash(state);
            val.hash(state);
        }
        self.table.len().hash(state);
        match &self.prototype {
            Some(value) => value.borrow().hash(state),
            _ => (),
        }
    }
}

#[derive(Clone)]
pub struct Function {
    pub native: bool,
    pub address: usize,
    pub env: Value,
    pub module: Option<Ref<Module>>,
    pub argc: i32,
}

pub trait UserKind: mopa::Any + fmt::Debug + fmt::Display {
    fn get_kind(&self) -> &'static str;
}
/*
use crate::gc::Trace;

impl Trace for Function {
    fn trace(&self) {
        match &self.module {
            Some(m) => m.trace(),
            _ => (),
        }
        self.env.trace();
    }
}

impl Trace for Value {
    fn trace(&self) {
        match self {
            Value::String(s) => s.trace(),
            Value::Array(a) => a.trace(),
            Value::Object(o) => o.trace(),
            Value::Function(f) => f.trace(),
            Value::User(_) => (),
            _ => (),
        }
    }
}

impl Trace for Object {
    fn trace(&self) {
        match &self.prototype {
            Some(proto) => proto.trace(),
            _ => (),
        }
        for (key, val) in self.table.iter() {
            key.trace();
            val.trace();
        }
    }
}
*/

mopafy!(UserKind);
