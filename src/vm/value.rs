use super::runtime::array::*;
use super::runtime::new_exfunc;
use cgc::generational::*;
use std::cell::{Ref as CRef, RefMut};

pub fn new_ref<T: 'static>(val: T) -> Ref<T> {
    Ref(Rc::new(RefCell::new(val)))
}
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct _Ref<T: Collectable + Sized>(GCValue<T>);
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Ref<T: Sized>(Rc<RefCell<T>>);



impl<T: 'static> Ref<T> {
    pub fn borrow(&self) -> CRef<'_,T> {
        self.0.borrow()
    }
    pub fn borrow_mut(&self) -> RefMut<'_,T> {
        self.0.borrow_mut()
    }
}

//unsafe impl<T: Send + Collectable> Send for Ref<T> {}
//unsafe impl<T: Sync + Collectable> Sync for Ref<T> {}
impl<T: Collectable + 'static> _Ref<T> {
    pub fn borrow(&self) -> CRef<'_, T> {
        self.0.borrow()
    }
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.0.borrow_mut()
    }

    pub fn gc(&self) -> GCValue<dyn Collectable> {
        self.0
    }
}

use crate::map::LinkedHashMap;

#[derive(Clone)]
pub struct ValueIter {
    pub values: Vec<Value>
}

impl ValueIter {
    #[inline]
    pub fn has_next(&self) -> bool {
        !self.values.is_empty()
    }

    pub fn next(&mut self) -> Value {
        self.values.remove(0)
    }
}



#[derive(Clone)]
pub enum ValueData {
    Nil,

    Undefined,
    Bool(bool),
    Number(f64),
    String(String),
    Object(Ref<Object>),
    Array(Ref<Vec<Value>>),
    Function(Ref<Function>),
    Iterator(Ref<ValueIter>),
}
/*
impl Mark for Object {
    fn mark(&self, gc: &mut InGcEnv) {
        match &self.proto {
            Some(ref proto) => proto.mark_grey(gc),
            None => (),
        }
        for (key, val) in self.table.iter() {
            key.mark(gc);
            val.mark_grey(gc);
        }
    }
}

impl Mark for Function {
    fn mark(&self, gc: &mut InGcEnv) {
        match self {
            Function::Regular { environment, .. } => {
                environment.mark_grey(gc);
            }
            _ => (),
        }
    }
}

impl Mark for ValueData {
    fn mark(&self, gc: &mut InGcEnv) {
        match self {
            ValueData::Object(object) => {
                object.mark_grey(gc);
            }
            ValueData::Array(array) => {
                array.mark_grey(gc);
            }
            ValueData::Function(f) => {
                f.mark_grey(gc);
            }
            _ => (),
        }
    }
}
*/
impl From<ValueData> for i64 {
    fn from(val: ValueData) -> i64 {
        match val {
            ValueData::Number(x) => x as i64,
            ValueData::Nil => 0,
            ValueData::Undefined => 0,
            _ => std::i64::MAX,
        }
    }
}

impl From<ValueData> for f64 {
    fn from(val: ValueData) -> f64 {
        match val {
            ValueData::Number(x) => x,
            ValueData::Nil => 0.0,
            ValueData::Undefined => std::f64::NAN,
            _ => std::f64::NAN,
        }
    }
}

impl From<ValueData> for bool {
    fn from(val: ValueData) -> bool {
        match val {
            ValueData::Number(x) => {
                if x.floor() == 0.0 {
                    false
                } else {
                    true
                }
            }
            ValueData::Bool(x) => x,
            ValueData::Nil => false,
            _ => false,
        }
    }
}

impl From<bool> for ValueData {
    fn from(val: bool) -> ValueData {
        ValueData::Bool(val)
    }
}

impl From<ValueData> for String {
    fn from(val: ValueData) -> String {
        match val {
            ValueData::String(s) => s.clone(),
            ValueData::Number(x) => x.to_string(),
            ValueData::Nil | ValueData::Undefined => String::new(),
            ValueData::Array(_) => format!("{}", val),
            ValueData::Object(_) => format!("{}", val),
            ValueData::Bool(b) => format!("{}", b),
            ValueData::Function(_) => "<function>".to_owned(),
            ValueData::Iterator(_iter) => format!("<iterator>"),
        }
    }
}

#[derive(Clone)]
pub enum Function {
    Native(usize),
    Regular {
        environment: Environment,
        code: wrc::WRC<std::cell::RefCell<Vec<super::opcodes::Opcode>>>, // code of function module,not of function itself
        addr: usize,
        yield_pos: Option<usize>,
        //constants: WRC<RefCell<Vec<ValueData>>>,
        yield_env: Ref<Object>,
        args: Vec<String>,
    },
}

pub trait SetGet {
    fn set(&mut self, _: impl Into<ValueData>, _: impl Into<Value>) {
        unimplemented!()
    }
    fn get(&self, _: &ValueData) -> Value {
        unimplemented!()
    }
}

impl SetGet for ValueData {
    fn set(&mut self, key: impl Into<ValueData>, val: impl Into<Value>) {
        let key = key.into();
        let val = val.into();
        match self {
            ValueData::Function(func) => {
                let func: &mut Function = &mut func.borrow_mut();
                match func {
                    Function::Regular { environment, .. } => {
                        environment.borrow_mut().set(key, val);
                        //gc::new_ref(*environment,val);
                    }
                    _ => (),
                }
            }
            ValueData::Object(object) => {
                object.borrow_mut().set(key, val);
            }
            ValueData::Array(array_) => {
                let mut array = array_.borrow_mut();
                let idx = i64::from(key);
                assert!(idx >= 0);
                array[idx as usize] = val;
                //gc::new_ref(*array_,val);
            }
            _ => {
            },
        }
    }

    fn get(&self, key: &ValueData) -> Value {
        match self {
            ValueData::Function(func) => {
                let func: &Function = &func.borrow();
                match func {
                    Function::Regular {
                        environment: _,
                        yield_pos,
                        ..
                    } => {
                        let val: String = String::from(key.clone());
                        let val: &str = &val;
                        match val {
                            "yields" => new_ref(ValueData::Bool(yield_pos.is_some())),
                            _ => new_ref(ValueData::Undefined),
                        }
                    }
                    _ => return new_ref(ValueData::Undefined),
                }
            }
            ValueData::Object(object) => object.borrow().get(key),
            ValueData::Array(array) => {
                let array = array.borrow();
                match key {
                    ValueData::String(s) => {
                        let s: &str = s;
                        match s {
                            "length" => return new_ref(ValueData::Number(array.len() as f64)),
                            "push" => return new_exfunc(array_push),
                            "pop" => return new_exfunc(array_pop),
                            "sort" => return new_exfunc(array_sort),
                            "indexOf" => return new_exfunc(array_indexof),
                            "remove" => return new_exfunc(array_remove),
                            _ => return new_ref(ValueData::Undefined),
                        }
                    }
                    ValueData::Number(idx) => {
                        let idx = *idx as i64;
                        assert!(idx >= 0);
                        return array[idx as usize].clone();
                    }
                    _ => return new_ref(ValueData::Undefined),
                }
            }
            _ => new_ref(ValueData::Undefined),
        }
    }
}

use std::fmt;
impl fmt::Display for ValueData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueData::Bool(x) => write!(f, "{}", x),
            ValueData::Number(x) => write!(f, "{}", x),
            ValueData::Function(_) => write!(f, "<function>"),
            ValueData::Nil => write!(f, "nil"),
            ValueData::Undefined => write!(f, "undefined"),
            ValueData::String(s) => write!(f, "{}", s),
            ValueData::Object(object) => {
                let object: &Object = &object.borrow();

                write!(f, "{{")?;
                for (i, (key, val)) in object.table.iter().enumerate() {
                    write!(f, "{}: {}", key, val.borrow())?;
                    if i != object.table.len() - 1 {
                        write!(f, ",")?;
                    }
                }
                write!(f, "}}")
            }
            ValueData::Iterator(_) => write!(f,"<iterator>"),
            ValueData::Array(array) => {
                let array = array.borrow();
                write!(f, "[")?;
                for (i, val) in array.iter().enumerate() {
                    write!(f, "{}", val.borrow())?;
                    if i != array.len() - 1 {
                        write!(f, ",")?;
                    }
                }
                write!(f, "]")
            }
        }
    }
}

impl fmt::Debug for ValueData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}
impl PartialEq for ValueData {
    fn eq(&self, other: &Self) -> bool {
        use ValueData::*;
        match (self, other) {
            (Number(x), Number(y)) => x == y,
            (Nil, Nil) => true,
            (Undefined, Undefined) => true,
            (String(x), String(y)) => x == y,
            (Object(x), Object(y)) => {
                let x_ref = x.borrow();
                let y_ref = y.borrow();
                *x_ref == *y_ref
            }
            (Array(x), Array(y)) => *x.borrow() == *y.borrow(),
            (Bool(x), Bool(y)) => x == y,

            _ => false,
        }
    }
}

use std::cmp::Ordering;

impl PartialOrd for ValueData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (ValueData::Number(x), ValueData::Number(y)) => x.partial_cmp(y),
            (ValueData::Array(x), ValueData::Array(y)) => x.borrow().partial_cmp(&y.borrow()),
            (ValueData::Object(obj), ValueData::Object(obj1)) => {
                obj.borrow().partial_cmp(&obj1.borrow())
            }
            (ValueData::String(x), ValueData::String(y)) => x.partial_cmp(y),
            (ValueData::Bool(x), ValueData::Bool(y)) => x.partial_cmp(y),
            _ => None,
        }
    }
}

impl Ord for ValueData {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Eq for ValueData {}

use std::hash::{Hash, Hasher};

impl Hash for ValueData {
    fn hash<H: Hasher>(&self, h: &mut H) {
        match self {
            ValueData::Number(x) => x.to_bits().hash(h),
            ValueData::Nil => 0.hash(h),
            ValueData::Undefined => 0.hash(h),
            ValueData::String(s) => s.hash(h),
            ValueData::Array(array) => {
                let array = array.borrow();
                for x in array.iter() {
                    x.borrow().hash(h);
                }
                array.len().hash(h);
            }
            ValueData::Bool(x) => x.hash(h),
            ValueData::Object(object) => object.borrow().hash(h),
            _ => (-1).hash(h),
        }
    }
}

pub type Value = Ref<ValueData>;

#[derive(Clone)]
pub struct Object {
    pub proto: Option<Ref<Object>>,
    pub table: LinkedHashMap<ValueData, Ref<ValueData>>,
}

pub fn set_obj_proto(obj: Ref<Object>, proto: Ref<Object>) {
    obj.borrow_mut().proto = Some(proto);
    //gc::new_ref(obj,proto);
}

use crate::token::Position;
pub fn set_variable_in_scope(
    scopes: &Ref<Object>,
    key: impl Into<ValueData>,
    val: Ref<ValueData>,
    pos: &Position,
) -> Result<(), ValueData> {
    let scope: &mut Object = &mut scopes.borrow_mut();
    let key = key.into();
    if scope.table.contains_key(&key) {
        scope.table.insert(key, val);
        //gc::new_ref(*scopes,val);
        return Ok(());
    }
    if scope.proto.is_some() {
        return set_variable_in_scope(scope.proto.as_ref().unwrap(), key, val, pos);
    }
    Err(new_error(
        pos.line as i32,
        None,
        &format!("Variable '{}' not declared", key),
    ))
}

pub fn declare_var(
    scope_: &Ref<Object>,
    key: impl Into<ValueData>,
    val: Ref<ValueData>,
    pos: &Position,
) -> Result<(), ValueData> {
    let scope: &mut Object = &mut scope_.borrow_mut();
    let key = key.into();
    if scope.table.contains_key(&key) {
        return Err(new_error(
            pos.line as _,
            None,
            &format!("Variable '{}' already declared", key),
        ));
    }
    scope.table.insert(key, val);
    //gc::new_ref(*scope_,val);
    Ok(())
}

pub fn var_declared(scope: &Ref<Object>, key: impl Into<ValueData>) -> bool {
    let scope: &Object = &scope.borrow();
    let key = key.into();
    scope.table.contains_key(&key)
}

pub fn get_variable(
    scope: &Ref<Object>,
    key: impl Into<ValueData>,
    pos: &Position,
) -> Result<Value, ValueData> {
    let scopes: &Object = &scope.borrow();
    let key = key.into();
    if scopes.table.contains_key(&key) {
        return Ok(scopes.table.get(&key).unwrap().clone());
    }
    if scopes.proto.is_some() {
        return get_variable(scopes.proto.as_ref().unwrap(), key, pos);
    }
    Err(new_error(
        pos.line as i32,
        None,
        &format!("Variable '{}' not declared", key),
    ))
}

impl SetGet for Object {
    fn set(&mut self, key: impl Into<ValueData>, val: impl Into<Value>) {
        let key = key.into();
        self.table.insert(key, val.into());
    }
    fn get(&self, key: &ValueData) -> Value {
        self.table
            .get(key)
            .unwrap_or(&new_ref(ValueData::Undefined))
            .clone()
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        self.table == other.table
            && match (&self.proto, &other.proto) {
                (Some(x), Some(y)) => *x.borrow() == *y.borrow(),
                (None, None) => true,
                _ => false,
            }
    }
}

impl PartialOrd for Object {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.table.partial_cmp(&other.table)
    }
}

impl Eq for Object {}

impl Hash for Object {
    fn hash<H: Hasher>(&self, h: &mut H) {
        for (key, val) in self.table.iter() {
            key.hash(h);
            val.borrow().hash(h);
        }
        self.table.len().hash(h);
        match &self.proto {
            Some(proto) => proto.borrow().hash(h),
            None => (),
        }
    }
}

pub type Environment = Ref<Object>;

pub fn new_object() -> Ref<Object> {
    new_ref(Object {
        proto: None,
        table: LinkedHashMap::new(),
    })
}

impl Into<ValueData> for String {
    fn into(self) -> ValueData {
        ValueData::String(self.to_owned())
    }
}

impl Into<ValueData> for &str {
    fn into(self) -> ValueData {
        ValueData::String(self.to_owned())
    }
}

impl Into<ValueData> for &String {
    fn into(self) -> ValueData {
        ValueData::String(self.to_owned())
    }
}
macro_rules! into_num {
    ($($t: ty)*) => {
        $(
        impl From<$t> for ValueData {
            fn from(x: $t) -> ValueData {
                ValueData::Number(x as f64)
            }
        }

        )*
    };
}

into_num!(
    f32 f64
    i8 i16 i32
    i64 i128
    u8 u32 u64 usize u16 u128
);

impl<T: Into<ValueData>> From<T> for Value {
    fn from(v: T) -> Value {
        new_ref(v.into())
    }
}
impl<T: Into<ValueData>> From<Option<T>> for ValueData {
    fn from(val: Option<T>) -> ValueData {
        match val {
            Some(x) => x.into(),
            None => ValueData::Nil,
        }
    }
}

pub fn new_error(line: i32, file: Option<&str>, err: &str) -> ValueData {
    let object = new_object();
    let proto = new_object();
    proto.borrow_mut().set("__name__", "JLRuntimeError");
    //object.borrow_mut().proto = Some(proto);
    set_obj_proto(object.clone(), proto);
    if line != -1 {
        object.borrow_mut().set("line", line);
    }
    if file.is_some() {
        object.borrow_mut().set("file", file);
    }
    object.borrow_mut().set("error", err);

    ValueData::Object(object)
}

pub fn instanceof(obj: &Ref<Object>, of: &Ref<Object>) -> bool {
    let of = of.borrow();
    if obj.borrow().proto.is_none() {
        return false;
    }

    *obj.borrow().proto.as_ref().unwrap().borrow() == *of
}

use std::ops::*;

impl Add for ValueData {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        match (self, other) {
            (ValueData::Number(x), ValueData::Number(y)) => ValueData::Number(x + y),
            (ValueData::Array(x), ValueData::Array(y)) => {
                let mut array = vec![];
                for x in x.borrow().iter() {
                    array.push(x.clone());
                }

                for y in y.borrow().iter() {
                    array.push(y.clone());
                }

                return ValueData::Array(new_ref(array));
            }
            (ValueData::String(x), val) => ValueData::String(format!("{}{}", x, val)),
            (val, ValueData::String(x)) => ValueData::String(format!("{}{}", val, x)),
            _ => ValueData::Undefined,
        }
    }
}

impl Sub for ValueData {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        match (self, other) {
            (ValueData::Number(x), ValueData::Number(y)) => ValueData::Number(x - y),

            _ => ValueData::Undefined,
        }
    }
}

impl Mul for ValueData {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        match (self, other) {
            (ValueData::Number(x), ValueData::Number(y)) => ValueData::Number(x * y),

            _ => ValueData::Undefined,
        }
    }
}
impl Div for ValueData {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        match (self, other) {
            (ValueData::Number(x), ValueData::Number(y)) => ValueData::Number(x / y),

            _ => ValueData::Undefined,
        }
    }
}

impl Rem for ValueData {
    type Output = Self;
    fn rem(self, other: Self) -> Self {
        match (self, other) {
            (ValueData::Number(x), ValueData::Number(y)) => ValueData::Number(x % y),

            _ => ValueData::Undefined,
        }
    }
}

impl Shr for ValueData {
    type Output = Self;
    fn shr(self, other: Self) -> Self {
        match (self, other) {
            (ValueData::Number(x), ValueData::Number(y)) => {
                ValueData::Number(((x.floor() as i64) >> y.floor() as i64) as f64)
            }

            _ => ValueData::Undefined,
        }
    }
}

impl Shl for ValueData {
    type Output = Self;
    fn shl(self, other: Self) -> Self {
        match (self, other) {
            (ValueData::Number(x), ValueData::Number(y)) => {
                ValueData::Number(((x.floor() as i64) << y.floor() as i64) as f64)
            }

            _ => ValueData::Undefined,
        }
    }
}

impl BitXor for ValueData {
    type Output = Self;
    fn bitxor(self, other: Self) -> Self {
        match (self, other) {
            (ValueData::Number(x), ValueData::Number(y)) => {
                ValueData::Number(((x.floor() as i64) ^ y.floor() as i64) as f64)
            }

            _ => ValueData::Undefined,
        }
    }
}

impl BitAnd for ValueData {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        match (self, other) {
            (ValueData::Number(x), ValueData::Number(y)) => {
                ValueData::Number(((x.floor() as i64) & y.floor() as i64) as f64)
            }

            _ => ValueData::Undefined,
        }
    }
}

impl BitOr for ValueData {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        match (self, other) {
            (ValueData::Number(x), ValueData::Number(y)) => {
                ValueData::Number(((x.floor() as i64) | y.floor() as i64) as f64)
            }

            _ => ValueData::Undefined,
        }
    }
}


/*
impl Collectable for Object {
    fn visit(&self,gc: &mut GenerationalGC)  {
        match &self.proto {
            Some(proto) => {
                gc_add_root(proto.gc());
                gc.push_grey(proto.gc());

            }
            _ => (),
        };

        for (key, val) in self.table.iter() {
            key.visit(gc);
            gc.push_grey(val.gc());
        }
    }

}
impl<T: Collectable + 'static> Collectable for Ref<T> {
    fn visit(&self,gc: &mut GenerationalGC) {
        gc.push_grey(self.0);
    }
}

impl Collectable for ValueData {
    fn visit(&self,gc: &mut GenerationalGC)  {
        match self {
            ValueData::Function(fun) => {
                gc.push_grey(fun.gc());
            }
            ValueData::Object(obj) => {
                gc.push_grey(obj.gc());
            }
            ValueData::Array(array) => {
                gc.push_grey(array.gc());
            }
            ValueData::Iterator(iter) => {
                gc.push_grey(*iter);
            }
            _ => (),
        }
    }
    
}

impl Collectable for Function {
    fn visit(&self,gc: &mut GenerationalGC) {
        match self {
            Function::Regular {
                environment,
                yield_env,
                ..
            } => {
                gc.push_grey(yield_env.gc());
                gc.push_grey(environment.gc());
            }
            _ => (),
        }
    }

}
*/

