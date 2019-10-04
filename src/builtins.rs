pub mod array;
pub mod common;
pub mod function;
pub mod math;
pub mod number;
pub mod object;

use crate::value::*;
use crate::*;

pub fn new_builtin_fn(f: usize, argc: i32) -> Value {
    let state = STATE.lock();
    let object = state
        .static_variables
        .get(&Value::String(Gc::new("Function".to_owned())))
        .cloned()
        .unwrap();
    let object_proto = state
        .static_variables
        .get(&Value::String(Gc::new("Object".to_owned())))
        .cloned()
        .unwrap();
    let object_proto = match object_proto {
        Value::Object(object) => object,
        _ => crate::unreachable(),
    };
    let object = match object {
        Value::Object(object) => object,
        _ => crate::unreachable(),
    };
    let fun = Gc::new(Function {
        module: None,
        addr: f,
        is_native: true,
        argc,
        env: Value::Null,
        prototype: Value::Object(object_proto),
    });
    let func = ObjectKind::Function(fun);
    let function = Value::Object(Gc::new(Object {
        proto: Some(object),
        kind: func,
        properties: Gc::new(vec![]),
    }));

    function
}

pub fn new_func(fun: Gc<Function>, argc: i32) -> Value {
    let state = STATE.lock();
    let object = state
        .static_variables
        .get(&Value::String(Gc::new("Function".to_owned())))
        .cloned()
        .unwrap();
    let object_proto = state
        .static_variables
        .get(&Value::String(Gc::new("Object".to_owned())))
        .cloned()
        .unwrap();
    let object_proto = match object_proto {
        Value::Object(object) => object,
        _ => crate::unreachable(),
    };
    let object = match object {
        Value::Object(object) => object,
        _ => crate::unreachable(),
    };

    fun.get_mut().prototype = Value::Object(object_proto);
    fun.get_mut().env = Value::Object(Gc::new(Object {
        kind: ObjectKind::Array(Gc::new(vec![])),
        properties: Gc::new(vec![]),
        proto: None,
    }));
    fun.get_mut().argc = argc;

    let func = ObjectKind::Function(fun);
    let function = Value::Object(Gc::new(Object {
        proto: Some(object),
        kind: func,
        properties: Gc::new(vec![]),
    }));

    function
}

pub extern "C" fn println(_: Value, args: &[Value]) -> Result<Value, Value> {
    for arg in args.iter() {
        print!("{}", arg);
    }
    println!();
    Ok(Value::Null)
}

pub fn builtin_fns() {
    let println = new_builtin_fn(println as _, -1);
    let mut state = STATE.lock();
    state
        .static_variables
        .insert(Value::String(Gc::new("println".to_owned())), println);
}

#[macro_export]
macro_rules! define_global {
    ($global_name: ident $static_name: expr; {
        $($t: tt)*
    }
    ) => {
        #[allow(non_snake_case)]
        pub mod $global_name {
            pub use crate::*;
            pub use value::*;
            use super::*;
                define_global!(@parse $($t)*);

            pub fn init() {
                let object = Gc::new(Object {
                    proto: None,
                    kind: ObjectKind::Ordinary,
                    properties: Gc::new(vec![])
                });

                define_global!(@set_properties object;$($t)*);

                let mut state = STATE.lock();
                state.static_variables.insert(Value::String(Gc::new($static_name.to_owned())),Value::Object(object));
            }
        }
    };

    (@set_properties $object: expr; function $name: ident ($($arg: ident),*) $b: block; $($rest: tt)* ) => {
        {
            let mut argc = 0;
            $(
                let $arg = Value::Null;
                argc += 1;
            )*


            $object.get_mut().set_property(Value::String(Gc::new(stringify!($name).to_owned())),new_builtin_fn($name as _,argc));

            define_global!(@set_properties $object; $($rest)*);
        }
    };
    (@set_properties $object: expr; function ($this: ident) $name: ident ($($arg: ident),*) $b: block; $($rest: tt)* ) => {
        {
            let mut argc = 0;
            $(
                let $arg = Value::Null;
                argc += 1;
            )*


            $object.get_mut().set_property(Value::String(Gc::new(stringify!($name).to_owned())),new_builtin_fn($name as _,argc));

            define_global!(@set_properties $object; $($rest)*);
        }
    };
    (@set_properties $object: expr; $name: ident = $val: expr; $($rest: tt)*) => {
        $object.get_mut().set_property(Value::String(Gc::new(stringify!($name).to_owned())),$val);
        define_global!(@set_properties $object; $($rest)*);
    };
    (@set_properties $object: expr;) => {};

    (@parse function $name: ident ($($arg: ident),*) $b: block; $($rest: tt)*) => {
        #[allow(unused_variables,unused_mut)]
        pub extern "C" fn $name(_: $crate::value::Value, args: &[$crate::value::Value]) -> Result<$crate::value::Value, $crate::value::Value> {
            let mut _i = 0;
            $(
                let $arg = args[_i].clone();
                _i += 1;
            )*
            $b
        }
        define_global!(@parse $($rest)*);
    };

    (@parse function ($this: ident) $name: ident ($($arg: ident),*) $b: block; $($rest: tt)*) => {
        #[allow(unused_variables,unused_mut)]
        pub extern "C" fn $name(this: $crate::value::Value, args: &[$crate::value::Value]) -> Result<$crate::value::Value, $crate::value::Value> {
            let mut i = 0;
            $(
                let $arg = args[i].clone();
                i += 1;
            )*
            let $this = this;
            $b
        }
        define_global!(@parse $($rest)*);
    };
    (@parse $name: ident = $val: expr; $($rest: tt)*) => {
        define_global!(@parse $($rest)*);
    };
    (@parse) => {}
}

define_global!(StringBuiltin "String"; {
    function empty() {
        return Ok(Value::String(Gc::new(String::new())));
    };

    function (this) pushStr(arg) {
        let string = arg.to_string();
        match this {
            Value::Object(object) => {
                match &object.get().kind {
                    ObjectKind::String(s) => {
                        s.get_mut().push_str(&string);
                    }
                    _ => crate::unreachable()
                }
            }
            _ => crate::unreachable()
        }
        Ok(Value::Null)
    };

    EMPTY = Value::String(Gc::new(String::new()));
});
