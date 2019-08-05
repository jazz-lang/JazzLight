#![feature(const_string_new)]
#![feature(unsize)]
#![feature(coerce_unsized)]
#![feature(allocator_api)]
#![feature(unboxed_closures)]
#![feature(decl_macro)]
#![allow(dead_code)]

pub mod ast;
#[macro_use]
pub mod macros;
pub mod compiler;
pub mod interner;
pub mod lexer;
pub mod map;
pub mod msg;
pub mod ngc;
pub mod parser;
pub mod gc;
pub mod reader;
pub mod token;
pub mod vm;
use Box as Arc;

pub type P<T> = Arc<T>;

#[allow(non_snake_case)]
pub fn P<T>(value: T) -> Arc<T> {
    Arc::new(value)
}

pub use interner::{intern, str, Name};


use vm::value::*;

pub struct State {
    env: Ref<Object>,
}

impl State {
    /// Create new state
    #[inline]
    pub fn new() -> State {
        let obj = new_object();
        vm::runtime::register_builtins(obj.clone());
       /* let ref_ = obj.clone();
        obj.borrow_mut().table.insert(ValueData::String("state".to_owned()),new_ref(ValueData::Object(ref_)));*/
        State {
            env: obj,
        }
    }
    /// Declare or set variable in current state:
    /// ```rust
    /// let mut state = State::new();
    /// state.set_var("x","Hello,World!");
    /// state.eval("print(x)");
    /// ```
    #[inline]
    pub fn set_var(&mut self,var: &str,value: impl Into<ValueData>) {
        let val = value.into();
        self.env.borrow_mut().set(var,val);
    }
    /// Get variable from current state:
    /// ```rust
    /// let mut state = State::new();
    /// state.set_var("x",ValueData::Undefined);
    /// state.eval("x = 42");
    /// println!("{}",state.get_var("x").borrow());
    /// ```
    #[inline]
    pub fn get_var(&mut self,var: &str) -> Value {
        self.env.borrow().get(&ValueData::String(var.to_owned()))
    }

    pub fn register_fn(&mut self,name: &str,f: fn(&mut vm::Frame<'_>,Value,&[Value]) -> Result<Value,ValueData>) {
        self.env.borrow_mut().set(name,ValueData::Function(new_ref(Function::Native(f as usize))));
    }

    /// Evaluate JazzLight code.
    /// 
    /// ```rust
    /// let mut state = State::new();
    /// 
    /// state.eval("print(42)");
    /// ```
    pub fn eval(&mut self,s: &str) -> Result<(),msg::MsgWithPos> {
        use compiler::Compiler;
        use reader::Reader;
        use vm::*;
        use parser::Parser;
        let mut ast = vec![];
        let r = Reader::from_string(s);
        let mut p = Parser::new(r,&mut ast);
        match p.parse() {
            Ok(_) => (),
            Err(e) => return Err(e)
        }
        let mut m = Machine::new();
        let mut f = Frame::new(&mut m);
        let mut c = Compiler::new(&mut f);
        c.compile_ast(&ast,false);
        c.frame.env = self.env.clone();
        c.frame.push_env();
        c.frame.execute();
        c.frame.pop_env();
        //self.compiled.insert(intern(s),c);
        Ok(())
    }
}