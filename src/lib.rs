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
pub mod gc;
pub mod map;
pub mod interner;
pub mod lexer;
pub mod msg;
pub mod ngc;
pub mod parser;
pub mod reader;
pub mod token;
pub mod vm;
use wrc::WRC as Arc;

pub type P<T> = Arc<T>;

#[allow(non_snake_case)]
pub fn P<T>(value: T) -> Arc<T> {
    Arc::new(value)
}

pub use interner::{intern, str, Name};
