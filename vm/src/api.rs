#[macro_export]
macro_rules! val_check {
    ($t: ident,$v: expr) => {
        $v.map(&mut |val| match val {
            Value::$t(_) => (),
            _ => panic!("Expected {} type", stringify!($t)),
        })
    };
    (Null,$v: expr) => {
        $v.map(|val| match val {
            Value::Null => (),
            _ => panic!("Expected Null type"),
        })
    };
}

use crate::value::{GcValue, Value};

pub fn val_as_int(val: &GcValue) -> i64 {
    val.map(&mut |val| match val {
        Value::Int(i) => *i,
        Value::Float(f) => *f as i64,
        _ => panic!("Can't get int"),
    })
}

pub fn val_as_float(val: &GcValue) -> f64 {
    val.map(&mut |val| match val {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f as f64,
        _ => panic!("Can't get int"),
    })
}
