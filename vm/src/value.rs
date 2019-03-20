use gc::{Gc, GcCell};
use gc::{GcCellRef, GcCellRefMut};
#[derive(Trace, Finalize, Clone, PartialEq, Debug)]
pub struct GcValue(Gc<GcCell<Value>>);

impl Hash for GcValue
{
    fn hash<H: Hasher>(&self, h: &mut H)
    {
        self.0.borrow().hash(h)
    }
}

impl Hash for Value
{
    fn hash<H: Hasher>(&self, h: &mut H)
    {
        match self
        {
            Value::Int(i) => h.write_i64(*i),
            Value::Float(f) => h.write_u64(f.to_bits()),
            Value::Str(s) => h.write(s.as_bytes()),
            Value::Null => h.write_i8(0),
            Value::Array(arr) => arr.hash(h),
            Value::Object(obj) => obj.hash(h),
            Value::Bool(b) => b.hash(h),
            Value::Func(f) => f.hash(h),
        }
    }
}

#[derive(Trace, Finalize, Clone, Debug, PartialEq)]
pub enum Value
{
    Null,
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Array(Vec<GcValue>),
    Object(Object),
    Func(Function),
}

#[derive(Clone, Debug, Trace, Finalize, PartialEq, Hash)]
pub struct ObjEntry
{
    pub(crate) key: GcValue,
    pub(crate) val: GcValue,
}

#[derive(Trace, Finalize, Clone, Debug, PartialEq, Hash)]
pub struct Object
{
    pub(crate) entries: Vec<ObjEntry>,
}

impl Object
{
    pub fn new(name: &str) -> Object
    {
        let mut entries = vec![];
        entries.push(ObjEntry { key: GcValue::new(Value::Str(format!("__name__"))),
                                val: GcValue::new(Value::Str(format!("{}", name))) });
        Object { entries }
    }

    pub fn find(&self, key: &GcValue) -> &GcValue
    {
        if self.entries.len() == 1
        {
            if &self.entries[0].key == key
            {
                return &self.entries[0].val;
            }
        }
        else
        {
            for entry in self.entries.iter()
            {
                if &entry.key == key
                {
                    return &entry.val;
                }
            }
        }
        panic!("Not found {:?}", key);
    }

    pub fn contains(&self, key: &GcValue) -> bool
    {
        for entry in self.entries.iter()
        {
            if &entry.key == key
            {
                return true;
            }
        }
        false
    }

    pub fn insert(&mut self, key: GcValue, val: GcValue)
    {
        if self.contains(&key)
        {
            for entry in self.entries.iter_mut()
            {
                if &entry.key == &key
                {
                    entry.val = val;
                    return;
                }
            }
        }
        else
        {
            self.entries.push(ObjEntry { key, val });
        }
    }
}

use std::hash::{Hash, Hasher};

impl GcValue
{
    pub fn new(val: Value) -> GcValue
    {
        GcValue(Gc::new(GcCell::new(val)))
    }
    #[inline]
    pub fn get_mut<'a>(&'a self) -> GcCellRefMut<'_, Value>
    {
        self.0.borrow_mut()
    }

    #[inline]
    pub fn get<'a>(&'a self) -> GcCellRef<'_, Value>
    {
        self.0.borrow()
    }

    pub fn map<T>(&self, f: &mut FnMut(&Value) -> T) -> T
    {
        let val = self.get();
        f(&val)
    }
    pub fn map_mut<T>(&self, f: &mut FnMut(&mut Value) -> T) -> T
    {
        let val = &mut self.get_mut();
        f(val)
    }
}

use crate::instruction::Instruction;

#[derive(Clone, Debug, Trace, Finalize, Hash, PartialEq)]
pub enum FuncVar
{
    Code(Vec<Instruction>, u16 /* max locals */),
    Native(i64),
}
#[derive(Clone, Debug, Trace, Finalize, Hash, PartialEq)]
pub struct Function
{
    pub var: FuncVar,
    pub name: String,
}
