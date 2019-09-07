use crate::*;
use parking_lot::Mutex;
use value::*;

#[derive(Clone)]
pub enum Infos {
    Exit,
    Info(
        Option<Ref<Module>>,
        usize,
        Value,
        Value,
        Ref<HashMap<u16, Value>>,
    ),
}

use std::collections::HashMap;

pub struct Vm {
    pub pc: usize,
    pub stack: Vec<Value>,
    pub exception_stack: Vec<(usize, Infos)>,
    pub info_stack: Vec<Infos>,
    pub env: Value,
    pub locals: Ref<HashMap<u16, Value>>,
    pub this: Value,
}

lazy_static::lazy_static! {
    pub static ref VM: Mutex<Vm> = Mutex::new(Vm::new());
}

impl Vm {
    pub fn new() -> Vm {
        Vm {
            pc: 0,
            stack: vec![],
            exception_stack: vec![],
            info_stack: vec![],
            env: Value::Null,
            locals: Ref(HashMap::new()),
            this: Value::Null,
        }
    }
    pub fn save_state_exit(&mut self) {
        self.info_stack.push(Infos::Exit);
    }

    pub fn save_state(&mut self, m: Option<Ref<Module>>) {
        self.info_stack.push(Infos::Info(
            m,
            self.pc,
            self.env.clone(),
            self.this.clone(),
            self.locals.clone(),
        ));
    }
    pub fn pop_state(&mut self, m: Option<&mut Ref<Module>>) -> bool {
        match self.info_stack.pop().unwrap() {
            Infos::Exit => true,
            Infos::Info(module, pc, env, this, locals) => {
                match m {
                    Some(m) => match module {
                        Some(module) => *m = module,
                        _ => (),
                    },
                    _ => (),
                }
                self.locals = locals;
                self.pc = pc;
                self.env = env;
                self.this = this;
                false
            }
        }
    }

    pub fn interp(&mut self, mut m: Ref<Module>) -> Value {
        use opcode::Op;
        macro_rules! throw {
            ($val: expr) => {
                catch!(Err($val));
            };
        }
        macro_rules! catch {
            ($e: expr) => {
                match $e {
                    Ok(val) => val,
                    Err(e) => {
                        if self.exception_stack.is_empty() {
                            let info = m.borrow().trace_info.get(&(self.pc as u32)).cloned();
                            if let Some((line, file)) = info {
                                eprintln!("Error in {}:{}: {}", file, line, e);
                            } else {
                                eprintln!("Error: {}", e);
                            }
                            std::process::exit(1);
                        } else {
                            if let Some((catch, Infos::Info(module, _, env, this, locals))) =
                                self.exception_stack.pop()
                            {
                                self.pc = catch as _;
                                match module {
                                    Some(module) => m = module,
                                    _ => (),
                                }
                                self.env = env;
                                self.this = this;
                                self.locals = locals;
                                self.stack.push(e);
                                continue;
                            } else {
                                unreachable!()
                            }
                        }
                    }
                }
            };
        }

        'inner: while self.pc < m.borrow().code.len() {
            let op = m.borrow().code[self.pc].clone();
            self.pc += 1;
            match op {
                Op::LoadBuiltin(name) => {
                    if name == "exports" {
                        self.stack.push(m.borrow().exports.clone());
                        continue;
                    }
                    use crate::builtins::get_builtin;
                    let value = get_builtin(&name);
                    if let Some(value) = value {
                        self.stack.push(value);
                    } else {
                        throw!(Value::String(Ref(format!("Builtin '{}' not found", name))));
                    }
                }
                Op::LoadNull => self.stack.push(Value::Null),
                Op::LoadInt(x) => self.stack.push(Value::Int(x)),
                Op::LoadTrue => self.stack.push(Value::Bool(true)),
                Op::LoadFalse => self.stack.push(Value::Bool(false)),
                Op::LoadGlobal(idx) => {
                    let idx = idx as usize;
                    self.stack
                        .push(m.borrow().globals.get(idx).cloned().unwrap_or(Value::Null));
                }
                Op::LoadLocal(idx) => {
                    self.stack.push(
                        self.locals
                            .borrow()
                            .get(&idx)
                            .cloned()
                            .unwrap_or(Value::Null),
                    );
                }
                Op::LoadEnv(idx) => {
                    let idx = idx as usize;
                    match &self.env {
                        Value::Array(array) => {
                            if idx >= array.borrow().len() {
                                panic!("JZVM RUNTIME ERROR: Reading outside env");
                            }
                            self.stack.push(array.borrow()[idx].clone());
                        }
                        _ => unreachable!(),
                    }
                }
                Op::LoadThis => {
                    self.stack.push(self.this.clone());
                }
                Op::StoreThis => {
                    let value = self.stack.pop();
                    match value {
                        Some(val) => self.this = val,
                        _ => throw!(Value::String(Ref("StoreThis: Stack empty".to_owned()))),
                    }
                }
                Op::StoreEnv(idx) => {
                    let idx = idx as usize;
                    let value = self.stack.pop();
                    match value {
                        Some(value) => match &self.env {
                            Value::Array(array) => {
                                array.borrow_mut()[idx] = value;
                            }
                            _ => unreachable!(),
                        },
                        _ => throw!(Value::String(Ref("StoreEnv: Stack empty".to_owned()))),
                    }
                }
                Op::StoreLocal(idx) => {
                    let value = self.stack.pop();
                    match value {
                        Some(value) => {
                            self.locals.borrow_mut().insert(idx, value);
                        }
                        _ => throw!(Value::String(Ref("StoreLocal: Stack empty".to_owned()))),
                    }
                }
                Op::Ret => {
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    let exit = self.pop_state(Some(&mut m));
                    if exit {
                        return value;
                    } else {
                        self.stack.push(value);
                    }
                }
                Op::CatchPush(addr) => {
                    let info = Infos::Info(
                        Some(m.clone()),
                        self.pc,
                        self.env.clone(),
                        self.this.clone(),
                        self.locals.clone(),
                    );
                    self.exception_stack.push((addr as usize, info));
                }
                Op::Throw => {
                    let value = self.stack.pop().unwrap();
                    catch!(Err(value));
                }
                Op::TailCall(argc) | Op::Call(argc) => {
                    let function = self.stack.pop().unwrap();
                    let args = (0..argc)
                        .into_iter()
                        .map(|_| self.stack.pop().unwrap_or(Value::Null))
                        .collect::<Vec<Value>>();
                    match function {
                        Value::Function(function) => {
                            let function = function.borrow();
                            if function.argc != -1 {
                                if args.len() < function.argc as usize
                                    || args.len() > function.argc as usize
                                {
                                    throw!(Value::String(Ref(format!(
                                        "Expected {} arguments,found {}",
                                        function.argc,
                                        args.len()
                                    ))));
                                }
                            }
                            if !function.native {
                                if let Op::TailCall(_) = op {
                                    self.pop_state(Some(&mut m));
                                }
                                self.save_state(Some(m.clone()));
                                self.env = function.env.clone();
                                self.locals = Ref(HashMap::new());
                                m = function.module.as_ref().unwrap().clone();
                                let mut locals = self.locals.borrow_mut();

                                for (i, arg) in args.iter().enumerate() {
                                    locals.insert(i as u16, arg.clone());
                                }
                                self.this = Value::Null;
                                self.pc = function.address;
                            } else {
                                let fun: fn(&[Value]) -> Result<Value, Value> =
                                    unsafe { std::mem::transmute(function.address) };

                                let result = catch!(fun(&args));
                                self.stack.push(result);
                                /*match fun(&args) {
                                    Ok(val) => self.stack.push(val),
                                    Err(e) => throw!(Err(e)),
                                }*/
                            }
                        }
                        _ => throw!(Value::String(Ref(format!(
                            "Call at {:x}: Function expected",
                            self.pc - 1
                        )))),
                    }
                }
                Op::ObjCall(argc) => {
                    let function = self.stack.pop().unwrap();
                    let this = self.stack.pop().unwrap();
                    /*let args = (0..argc)
                    .into_iter()
                    .map(|_| self.stack.pop().unwrap_or(Value::Null))
                    .collect::<Vec<Value>>();*/
                    let mut args = vec![];
                    for _ in 0..argc {
                        args.push(self.stack.pop().unwrap_or(Value::Null));
                    }

                    match function {
                        Value::Function(function) => {
                            let function = function.borrow();
                            self.save_state(Some(m.clone()));
                            self.env = function.env.clone();
                            if function.argc != -1 {
                                if args.len() < function.argc as usize
                                    || args.len() > function.argc as usize
                                {
                                    throw!(Value::String(Ref(format!(
                                        "Expected {} arguments,found {}",
                                        function.argc,
                                        args.len()
                                    ))));
                                }
                            }
                            if !function.native {
                                self.locals = Ref(HashMap::new());
                                if let Some(module) = &function.module {
                                    m = module.clone();
                                }
                                let mut locals = self.locals.borrow_mut();
                                for (i, arg) in args.iter().enumerate() {
                                    locals.insert(i as u16, arg.clone());
                                }
                                self.this = this;
                                self.pc = function.address;
                            } else {
                                let fun: fn(&[Value]) -> Result<Value, Value> =
                                    unsafe { std::mem::transmute(function.address) };
                                let mut new_args = vec![this];
                                for i in args.iter() {
                                    new_args.push(i.clone());
                                }
                                let result = catch!(fun(&new_args));
                                self.stack.push(result);
                                /*match fun(&args) {
                                    Ok(val) => self.stack.push(val),
                                    Err(e) => throw!(Err(e)),
                                }*/
                            }
                        }
                        _ => throw!(Value::String(Ref("ObjCall: Function expected".to_owned()))),
                    }
                }
                Op::Nop => {}
                Op::MakeEnv(count) => {
                    let function = self.stack.pop().unwrap();
                    assert_eq!(function.tag(), ValTag::Func);
                    let values = (0..count)
                        .into_iter()
                        .map(|_| self.stack.pop().unwrap_or(Value::Null))
                        .collect::<Vec<Value>>();
                    match &function {
                        Value::Function(func) => {
                            func.borrow_mut().env = Value::Array(Ref(values));
                        }
                        _ => unreachable!(),
                    }
                    self.stack.push(function);
                }

                Op::Load => {
                    let object = self.stack.pop().unwrap();
                    let key = self.stack.pop().unwrap();
                    match object {
                        Value::Array(array) => match key {
                            Value::Int(x) => self.stack.push(
                                array
                                    .borrow()
                                    .get(x as usize)
                                    .cloned()
                                    .unwrap_or(Value::Null),
                            ),
                            Value::Float(x) => self.stack.push(
                                array
                                    .borrow()
                                    .get(x as usize)
                                    .cloned()
                                    .unwrap_or(Value::Null),
                            ),
                            _ => self.stack.push(Value::Null),
                        },
                        Value::Object(object) => {
                            self.stack
                                .push(object.borrow().get(key).unwrap_or(Value::Null));
                        }
                        _ => self.stack.push(Value::Null),
                    }
                }
                Op::Store => {
                    let object = self.stack.pop().unwrap();
                    let key = self.stack.pop().unwrap();
                    let value = self.stack.pop().unwrap();
                    match object {
                        Value::Array(array) => match key {
                            Value::Int(x) => {
                                if x as usize >= array.borrow().len() {
                                    throw!(Value::String(Ref(
                                        "Array index out of bounds".to_owned()
                                    )));
                                }
                                array.borrow_mut()[x as usize] = value;
                            }
                            Value::Float(x) => {
                                if x as usize >= array.borrow().len() {
                                    throw!(Value::String(Ref(
                                        "Array index out of bounds".to_owned()
                                    )));
                                }
                                array.borrow_mut()[x as usize] = value;
                            }
                            _ => (),
                        },
                        Value::Object(object) => {
                            object.borrow_mut().set(key, value);
                        }
                        _ => throw!(Value::String(Ref("Invalid store operation".to_string()))),
                    }
                }
                Op::MakeArray(count) => {
                    let values = (0..count)
                        .into_iter()
                        .map(|_| self.stack.pop().unwrap())
                        .collect::<Vec<Value>>();

                    self.stack.push(Value::Array(Ref(values)));
                }
                Op::Add => {
                    let lhs = self.stack.pop().unwrap();
                    let rhs = self.stack.pop().unwrap();
                    match lhs {
                        Value::String(x) => {
                            self.stack
                                .push(Value::String(Ref(format!("{}{}", *x.borrow(), rhs))))
                        }
                        Value::Int(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Int(x + y)),
                            Value::Float(y) => self.stack.push(Value::Float(x as f64 + y)),
                            _ => self.stack.push(Value::Null),
                        },
                        Value::Float(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Float(x + y as f64)),
                            Value::Float(y) => self.stack.push(Value::Float(x + y as f64)),
                            _ => self.stack.push(Value::Null),
                        },
                        _ => self.stack.push(Value::Null),
                    }
                }
                Op::Sub => {
                    let lhs = self.stack.pop().unwrap();
                    let rhs = self.stack.pop().unwrap();
                    match lhs {
                        Value::Int(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Int(x - y)),
                            Value::Float(y) => self.stack.push(Value::Float(x as f64 - y)),
                            _ => self.stack.push(Value::Null),
                        },
                        Value::Float(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Float(x - y as f64)),
                            Value::Float(y) => self.stack.push(Value::Float(x - y as f64)),
                            _ => self.stack.push(Value::Null),
                        },
                        _ => self.stack.push(Value::Null),
                    }
                }
                Op::Div => {
                    let lhs = self.stack.pop().unwrap();
                    let rhs = self.stack.pop().unwrap();
                    match lhs {
                        Value::Int(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Int(x / y)),
                            Value::Float(y) => self.stack.push(Value::Float(x as f64 / y)),
                            _ => self.stack.push(Value::Null),
                        },
                        Value::Float(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Float(x / y as f64)),
                            Value::Float(y) => self.stack.push(Value::Float(x / y as f64)),
                            _ => self.stack.push(Value::Null),
                        },
                        _ => self.stack.push(Value::Null),
                    }
                }
                Op::Mul => {
                    let lhs = self.stack.pop().unwrap();
                    let rhs = self.stack.pop().unwrap();
                    match lhs {
                        Value::Int(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Int(x * y)),
                            Value::Float(y) => self.stack.push(Value::Float(x as f64 * y)),
                            _ => self.stack.push(Value::Null),
                        },
                        Value::Float(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Float(x * y as f64)),
                            Value::Float(y) => self.stack.push(Value::Float(x * y as f64)),
                            _ => self.stack.push(Value::Null),
                        },
                        _ => self.stack.push(Value::Null),
                    }
                }
                Op::Mod => {
                    let lhs = self.stack.pop().unwrap();
                    let rhs = self.stack.pop().unwrap();
                    match lhs {
                        Value::Int(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Int(x % y)),
                            Value::Float(y) => self.stack.push(Value::Float(x as f64 % y)),
                            _ => self.stack.push(Value::Null),
                        },
                        Value::Float(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Float(x % y as f64)),
                            Value::Float(y) => self.stack.push(Value::Float(x % y as f64)),
                            _ => self.stack.push(Value::Null),
                        },
                        _ => self.stack.push(Value::Null),
                    }
                }
                Op::Shr => {
                    let lhs = self.stack.pop().unwrap();
                    let rhs = self.stack.pop().unwrap();
                    match (lhs, rhs) {
                        (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x >> y)),
                        _ => self.stack.push(Value::Null),
                    }
                }
                Op::Shl => {
                    let lhs = self.stack.pop().unwrap();
                    let rhs = self.stack.pop().unwrap();
                    match (lhs, rhs) {
                        (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x << y)),
                        (Value::Array(array), any_value) => {
                            self.stack.push(any_value.clone());
                            array.borrow_mut().push(any_value);
                        }
                        _ => self.stack.push(Value::Null),
                    }
                }

                Op::Gt => {
                    let lhs = self.stack.pop().unwrap();
                    let rhs = self.stack.pop().unwrap();
                    match lhs {
                        Value::String(x) => match rhs {
                            Value::String(y) => self
                                .stack
                                .push(Value::Bool(x.borrow().len() > y.borrow().len())),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        Value::Int(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Bool(x > y)),
                            Value::Float(y) => self.stack.push(Value::Bool((x as f64) > y)),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        Value::Float(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Bool(x > y as f64)),
                            Value::Float(y) => self.stack.push(Value::Bool(x > y as f64)),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        Value::Array(x) => match rhs {
                            Value::Array(y) => self
                                .stack
                                .push(Value::Bool(x.borrow().len() > y.borrow().len())),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        _ => self.stack.push(Value::Bool(false)),
                    }
                }
                Op::Gte => {
                    let lhs = self.stack.pop().unwrap();
                    let rhs = self.stack.pop().unwrap();
                    match lhs {
                        Value::String(x) => match rhs {
                            Value::String(y) => {
                                self.stack.push(Value::Bool(*x.borrow() >= *y.borrow()))
                            }
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        Value::Int(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Bool(x >= y)),
                            Value::Float(y) => self.stack.push(Value::Bool((x as f64) >= y)),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        Value::Float(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Bool(x >= y as f64)),
                            Value::Float(y) => self.stack.push(Value::Bool(x >= y as f64)),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        Value::Array(x) => match rhs {
                            Value::Array(y) => self.stack.push(Value::Bool(
                                (x.borrow().len() > y.borrow().len()) || *x.borrow() == *y.borrow(),
                            )),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        _ => self.stack.push(Value::Bool(false)),
                    }
                }
                Op::Lte => {
                    let lhs = self.stack.pop().unwrap();
                    let rhs = self.stack.pop().unwrap();
                    match lhs {
                        Value::String(x) => match rhs {
                            Value::String(y) => {
                                self.stack.push(Value::Bool(*x.borrow() >= *y.borrow()))
                            }
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        Value::Int(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Bool(x >= y)),
                            Value::Float(y) => self.stack.push(Value::Bool((x as f64) >= y)),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        Value::Float(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Bool(x >= y as f64)),
                            Value::Float(y) => self.stack.push(Value::Bool(x >= y as f64)),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        Value::Array(x) => match rhs {
                            Value::Array(y) => self.stack.push(Value::Bool(
                                (x.borrow().len() < y.borrow().len()) || *x.borrow() == *y.borrow(),
                            )),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        _ => self.stack.push(Value::Bool(false)),
                    }
                }
                Op::Lt => {
                    let lhs = self.stack.pop().unwrap();
                    let rhs = self.stack.pop().unwrap();
                    match lhs {
                        Value::String(x) => match rhs {
                            Value::String(y) => self
                                .stack
                                .push(Value::Bool(x.borrow().len() < y.borrow().len())),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        Value::Int(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Bool(x < y)),
                            Value::Float(y) => self.stack.push(Value::Bool((x as f64) < y)),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        Value::Float(x) => match rhs {
                            Value::Int(y) => self.stack.push(Value::Bool(x < y as f64)),
                            Value::Float(y) => self.stack.push(Value::Bool(x < y as f64)),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        Value::Array(x) => match rhs {
                            Value::Array(y) => self
                                .stack
                                .push(Value::Bool(x.borrow().len() < y.borrow().len())),
                            _ => self.stack.push(Value::Bool(false)),
                        },
                        _ => self.stack.push(Value::Bool(false)),
                    }
                }
                Op::Eq => {
                    let lhs = self.stack.pop().unwrap();
                    let rhs = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(lhs == rhs));
                }
                Op::Neq => {
                    let lhs = self.stack.pop().unwrap();
                    let rhs = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(lhs != rhs));
                }
                Op::IsNull => {
                    let val = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(val.tag() == ValTag::Null));
                }
                Op::IsNotNull => {
                    let val = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(val.tag() != ValTag::Null));
                }
                Op::Jump(to) => {
                    self.pc = to as _;
                }
                Op::JumpIf(to) => {
                    let value = self.stack.pop().unwrap().to_bool();
                    if value {
                        self.pc = to as _;
                    }
                }
                Op::JumpIfNot(to) => {
                    let value = self.stack.pop().unwrap().to_bool();
                    if !value {
                        self.pc = to as _;
                    }
                }
                Op::Not => {
                    let val = self.stack.pop().unwrap();
                    match val {
                        Value::Int(x) => self.stack.push(Value::Int(!x)),
                        _ => self.stack.push(Value::Bool(!val.to_bool())),
                    }
                }
                Op::Neg => {
                    let val = self.stack.pop().unwrap();
                    match val {
                        Value::Int(x) => self.stack.push(Value::Int(-x)),
                        Value::Float(x) => self.stack.push(Value::Float(-x)),
                        _ => self.stack.push(Value::Null),
                    }
                }
                Op::And => {
                    let (x, y) = (self.stack.pop().unwrap(), self.stack.pop().unwrap());
                    match (x.clone(), y.clone()) {
                        (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x & y)),
                        (Value::Bool(x), Value::Bool(y)) => self.stack.push(Value::Bool(x & y)),
                        _ => self.stack.push(Value::Bool(x.to_bool() & y.to_bool())),
                    }
                }
                Op::Or => {
                    let (x, y) = (self.stack.pop().unwrap(), self.stack.pop().unwrap());
                    match (x.clone(), y.clone()) {
                        (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x & y)),
                        (Value::Bool(x), Value::Bool(y)) => self.stack.push(Value::Bool(x & y)),
                        _ => self.stack.push(Value::Bool(x.to_bool() & y.to_bool())),
                    }
                }
                Op::Xor => {
                    let (x, y) = (self.stack.pop().unwrap(), self.stack.pop().unwrap());
                    match (x.clone(), y.clone()) {
                        (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x ^ y)),
                        _ => self.stack.push(Value::Null),
                    }
                }
                Op::New => {
                    let proto = self.stack.pop().unwrap();
                    let proto = match proto {
                        Value::Null => None,
                        Value::Object(obj) => Some(obj),
                        _ => throw!(Value::String(Ref(
                            "Object or null expected as prototype".to_owned()
                        ))),
                    };
                    let object = Object {
                        prototype: proto,
                        table: hashlink::LinkedHashMap::new(),
                    };
                    self.stack.push(Value::Object(Ref(object)));
                }
                Op::Last => break 'inner,
                _ => unimplemented!(),
            }
        }
        self.stack.pop().unwrap_or(Value::Null)
    }
}

pub fn val_callex(f: Value, this: Value, args: &[Value]) -> Result<Value, Value> {
    let mut vm: parking_lot::MutexGuard<Vm> = VM.lock();
    match f {
        Value::Function(f) => {
            let function = f.borrow();
            if function.native {
                let fun: fn(&[Value]) -> Result<Value, Value> =
                    unsafe { std::mem::transmute(function.address) };
                let mut new_args = vec![this];
                for i in args.iter() {
                    new_args.push(i.clone());
                }

                return fun(&new_args);
            } else {
                vm.save_state_exit();
                let env = vm.env.clone();
                let locals = vm.locals.clone();
                let pc = vm.pc.clone();
                let this_ = vm.this.clone();
                vm.pc = function.address;
                vm.this = this;
                vm.env = function.env.clone();
                vm.locals = Ref(HashMap::new());
                if args.len() > function.argc as usize {
                    return Err(Value::String(Ref("Too many arguments".to_owned())));
                } else if args.len() < function.argc as usize {
                    return Err(Value::String(Ref("Unexpected arguments count".to_owned())));
                }
                for (i, arg) in args.iter().enumerate() {
                    vm.locals.borrow_mut().insert(i as u16, arg.clone());
                }
                let value = vm.interp(function.module.as_ref().unwrap().clone());
                vm.env = env;
                vm.locals = locals;
                vm.pc = pc;
                vm.this = this_;
                return Ok(value);
            }
        }
        _ => return Err(Value::String(Ref("Function expected".to_owned()))),
    }
}
