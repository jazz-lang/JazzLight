use crate::*;
use value::*;

use std::fs::File;
use std::io::{Read, Write};

pub struct FileHandle(File);

use std::fmt;

impl fmt::Debug for FileHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl fmt::Display for FileHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl UserKind for FileHandle {
    fn get_kind(&self) -> &'static str {
        "File"
    }
}

pub fn file_open(args: &[Value]) -> Result<Value, Value> {
    let s = args[0].to_string();

    let file = std::fs::OpenOptions::new().write(true).read(true).open(&s);
    match file {
        Ok(file) => return Ok(Value::User(Ref(FileHandle(file)))),
        Err(e) => return Err(Value::String(Ref(e.to_string()))),
    }
}

pub fn file_contents(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::User(file) => {
            if let Some(file) = file.borrow_mut().downcast_mut::<FileHandle>() {
                let file: &mut File = &mut file.0;
                let mut buf = String::new();
                match file.read_to_string(&mut buf) {
                    Ok(_) => (),
                    Err(e) => return Err(Value::String(Ref(e.to_string()))),
                }
                return Ok(Value::String(Ref(buf)));
            } else {
                return Err(Value::String(Ref(
                    "file_contents: File expected".to_string()
                )));
            }
        }
        _ => {
            return Err(Value::String(
                Ref("file_contents: File expected".to_owned()),
            ))
        }
    }
}

pub fn file_flush(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::User(file) => {
            if let Some(handle) = file.borrow_mut().downcast_mut::<FileHandle>() {
                let file: &mut File = &mut handle.0;
                match file.flush() {
                    Ok(_) => return Ok(Value::Null),
                    Err(e) => return Err(Value::String(Ref(e.to_string()))),
                }
            } else {
                return Err(Value::String(Ref("file_flush: File expected".to_string())));
            }
        }
        _ => return Err(Value::String(Ref("file_flush: File expected".to_string()))),
    }
}

pub fn file_write_string(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::User(file) => {
            if let Some(handle) = file.borrow_mut().downcast_mut::<FileHandle>() {
                let file: &mut File = &mut handle.0;
                let s = args[1].to_string();
                match file.write(s.as_bytes()) {
                    Ok(count) => return Ok(Value::Int(count as _)),
                    Err(e) => return Err(Value::String(Ref(e.to_string()))),
                }
            } else {
                return Err(Value::String(Ref("file_flush: File expected".to_string())));
            }
        }
        _ => return Err(Value::String(Ref("file_flush: File expected".to_string()))),
    }
}

pub fn file_write(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::User(file) => {
            if let Some(handle) = file.borrow_mut().downcast_mut::<FileHandle>() {
                let file: &mut File = &mut handle.0;
                let bytes: Vec<u8> = match &args[1] {
                    Value::Int(x) => x.to_le_bytes().iter().map(|x| *x).collect::<Vec<_>>(),
                    Value::Array(array) => {
                        let mut bytes = vec![];
                        for x in array.borrow().iter() {
                            match x {
                                Value::Int(x) => bytes.push(*x as u8),
                                Value::Char(x) => bytes.extend((*x as u32).to_le_bytes().iter()),
                                _ => {
                                    return Err(Value::String(Ref(
                                        "Unexpected value to write".to_owned()
                                    )))
                                }
                            }
                        }
                        bytes
                    }
                    Value::Char(x) => (*x as u32)
                        .to_le_bytes()
                        .iter()
                        .map(|x| *x)
                        .collect::<Vec<_>>(),
                    _ => return Err(Value::String(Ref("Unexpected value to write".to_owned()))),
                };
                match file.write(&bytes) {
                    Ok(count) => return Ok(Value::Int(count as _)),
                    Err(e) => return Err(Value::String(Ref(e.to_string()))),
                }
            } else {
                return Err(Value::String(Ref("file_flush: File expected".to_string())));
            }
        }
        _ => return Err(Value::String(Ref("file_flush: File expected".to_string()))),
    }
}

pub fn file_write_byte(args: &[Value]) -> Result<Value, Value> {
    match &args[0] {
        Value::User(file) => {
            if let Some(handle) = file.borrow_mut().downcast_mut::<FileHandle>() {
                let file: &mut File = &mut handle.0;
                match &args[1] {
                    Value::Int(byte) => match file.write(&[*byte as u8]) {
                        Ok(_) => return Ok(Value::Null),
                        Err(e) => return Err(Value::String(Ref(e.to_string()))),
                    },
                    _ => {
                        return Err(Value::String(Ref(
                            "file_write_byte: Int expected".to_string()
                        )))
                    }
                }
            } else {
                return Err(Value::String(Ref(
                    "file_write_byte: File expected".to_string()
                )));
            }
        }
        _ => {
            return Err(Value::String(Ref(
                "file_write_byte: File expected".to_string()
            )))
        }
    }
}
use super::*;

pub fn file_builtins(map: &mut std::collections::HashMap<String, Value>) {
    map.insert("file_open".to_owned(), new_native_fn(file_open, 1));
    map.insert("file_contents".to_owned(), new_native_fn(file_contents, 1));
    map.insert("file_flush".to_owned(), new_native_fn(file_flush, 0));
    map.insert(
        "file_write_string".to_owned(),
        new_native_fn(file_write_string, 2),
    );
    map.insert("file_write".to_owned(), new_native_fn(file_write, 2));
    map.insert(
        "file_write_byte".to_owned(),
        new_native_fn(file_write_byte, 2),
    );
}
