pub mod opcodes;
#[macro_use]
pub mod runtime;
pub mod codegen;
pub mod value;
use crate::str;
use opcodes::Opcode;
use std::cell::RefCell;
use value::*;
use wrc::WRC;
use crate::ngc::{gc_add_root,gc_rmroot};

pub fn nil() -> Value {
    new_ref(ValueData::Nil)
}
use crate::token::Position;
use crate::map::LinkedHashMap;

pub struct Machine {
    pub constants: Vec<ValueData>,
    pub line_no: LinkedHashMap<(usize, Opcode), Position>,
}

impl Machine {
    pub fn new() -> Machine {
        Machine {
            constants: vec![],
            line_no: LinkedHashMap::new(),
        }
    }
}

enum ExecData {
    Pc(usize),
    Env(Environment),
    Code(WRC<RefCell<Vec<Opcode>>>),
    Stack(Vec<Value>),
}

pub struct Frame<'a> {
    pub m: &'a mut Machine,
    pub code: WRC<RefCell<Vec<Opcode>>>,
    pub stack: Vec<Value>,
    pub pc: usize,
    pub env: Environment,
    pub funs: Vec<Ref<Function>>,
    exec_stack: Vec<ExecData>,
    exception_stack: Vec<usize>,
}

impl<'a> Frame<'a> {
    pub fn new(m: &'a mut Machine) -> Frame<'a> {
        let f = Frame {
            m,
            code: WRC::new(RefCell::new(vec![])),
            stack: vec![],
            pc: 0,
            env: new_object(),
            funs: vec![],
            exec_stack: vec![],
            exception_stack: vec![],
        };
        f

    }

    pub fn restore_state(
        &mut self,
        restore_pc: bool,
        restore_env: bool,
        restore_code: bool,
        restore_stack: bool,
    ) {
        if restore_pc {
            if let Some(ExecData::Pc(pc)) = self.exec_stack.pop() {
                self.pc = pc;
            }
        }
        if restore_env {
            if let Some(ExecData::Env(env)) = self.exec_stack.pop() {
                self.env = env;
            }
        }
        if restore_code {
            if let Some(ExecData::Code(code)) = self.exec_stack.pop() {
                if self.code != code {
                    self.code = code;
                }
            }
        }
        if restore_stack {
            if let Some(ExecData::Stack(stack)) = self.exec_stack.pop() {
                self.stack = stack;
            }
        }
    }

    pub fn save_state(&mut self, save_pc: bool, save_env: bool, save_code: bool, save_stack: bool) {
        if save_stack {
            self.exec_stack.push(ExecData::Stack(self.stack.clone()));
        }
        if save_code {
            self.exec_stack.push(ExecData::Code(self.code.clone()));
        }
        if save_env {
            self.exec_stack.push(ExecData::Env(self.env.clone()));
        }
        if save_pc {
            self.exec_stack.push(ExecData::Pc(self.pc));
        }
    }

    pub fn push_env(&mut self) {
        let old_env = self.env.clone();
        self.env = new_ref(Object {
            proto: Some(old_env),
            table: LinkedHashMap::new(),
        });
       // gc_add_root(self.env.gc());
        //crate::gc:://gc::new_ref(self.env,old_env);
    }

    pub fn pop_env(&mut self) {
        if self.env.borrow().proto.is_none() {
            panic!("No env to pop");
        }
        //gc_rmroot(self.env.gc());
        
        
        let proto = {
            let env = self.env.borrow();
            env.proto.as_ref().unwrap().clone()
        };
        self.env = proto.clone();
    }

    pub fn push(&mut self, val: impl Into<ValueData>) {
        self.stack.push(new_ref(val.into()));
    }

    pub fn push_ref(&mut self, val: Value) {
        self.stack.push(val);
    }
    pub fn get_pos(&self) -> Position {
        let pos = self
            .m
            .line_no
            .get(&(self.pc - 1, self.code.borrow()[self.pc - 1]))
            .unwrap()
            .clone();

        pos
    }
    pub fn pop(&mut self) -> Result<Value, ValueData> {
        match self.stack.pop() {
            Some(val) => Ok(val),
            None => Err(new_error(self.get_pos().line as _, None, "Stack empty")),
        }
    }

    pub fn execute(&mut self) {
        macro_rules! catch {
            ($result: expr) => {
                match $result {
                    Ok(val) => val,
                    Err(e) => {
                        if let Some(location) = self.exception_stack.pop() {
                            self.pc = location;
                            self.push(e);
                            continue;
                        } else {
                            eprintln!("{}", e);
                            std::process::exit(1);
                        }
                    }
                }
            };
        }

        macro_rules! throw {
            ($msg: expr) => {{
                let error = new_error(
                    self.get_pos().line as _,
                    None,
                    &format!("Runtime exception: {}", $msg),
                );
                if let Some(location) = self.exception_stack.pop() {
                    self.pc = location;
                    self.push(error);
                    continue;
                } else {
                    eprintln!("{}", error);
                    std::process::exit(-1);
                }
            }};
        }

        while self.pc < self.code.borrow().len() {
            let opcode = self.code.borrow()[self.pc];
            self.pc += 1;
            use Opcode::*;
            match opcode {
                LoadConst(index) => {
                    let constant = self.m.constants[index as usize].clone();
                    self.push(constant);
                }
                LoadInt(val) => {
                    self.push(val);
                }
                LoadFalse => {
                    self.push(false);
                }
                LoadTrue => {
                    self.push(true);
                }
                LoadNil => {
                    self.push(ValueData::Nil);
                }
                LoadUndef => {
                    self.push(ValueData::Undefined);
                }
                LoadVar(var) => {
                    //let pos = *self.m.line_no.get(&(self.pc, opcode)).unwrap();
                    let pos = Position::new(0, 0);

                    let variable = catch!(get_variable(
                        &self.env,
                        ValueData::String(str(var).to_string()),
                        &pos
                    ));
                    self.push_ref(variable);
                }
                DeclVar(name) => {
                    //println!("{} {:#?}",self.pc,self.m.line_no);
                    //let pos = *self.m.line_no.get(&(self.pc - 1, opcode)).unwrap();
                    //
                    let pos = Position::new(0, 0);
                    let val = catch!(self.pop());
                    catch!(declare_var(
                        &self.env,
                        ValueData::String(str(name).to_string()),
                        val,
                        &pos
                    ));
                }
                StoreVar(name) => {
                    //let pos = *self.m.line_no.get(&(self.pc - 1, opcode)).unwrap();
                    let pos = Position::new(0, 0);
                    let val = catch!(self.pop());

                    catch!(set_variable_in_scope(
                        &self.env,
                        ValueData::String(str(name).to_string()),
                        val,
                        &pos
                    ));
                }
                Opcode::Dup => {
                    let val = self.stack.pop().unwrap_or(new_ref(ValueData::Undefined));
                    self.push_ref(val.clone());
                    self.push_ref(val);
                }
                ConstructArray(n) => {
                    let array = new_ref(vec![]);
                    for _ in 0..n {
                        let val = catch!(self.pop());
                        array.borrow_mut().push(val);
                        //crate::gc:://gc::new_ref(array,val);
                    }
                    self.push(ValueData::Array(array));
                }
                Store => {
                    let value = catch!(self.pop());
                    let key = catch!(self.pop());
                    let object = catch!(self.pop());

                    object
                        .borrow_mut()
                        .set(key.borrow().clone(), value.borrow().clone());
                }
                NewObj => {
                    self.push(ValueData::Object(new_object()));
                }
                Load => {
                    let key = catch!(self.pop());
                    let key: &ValueData = &key.borrow();
                    let object = catch!(self.pop());

                    self.push_ref(object.borrow().get(key));
                }
                Return => {
                    let return_ = match self.stack.pop() {
                        Some(val) => val,
                        None => new_ref(ValueData::Undefined),
                    };
                    self.restore_state(true, true, true, true);
                    match self.funs.last() {
                        Some(fun) => {
                            let fun: &mut Function = &mut fun.borrow_mut();
                            match fun {
                                Function::Regular { yield_pos, .. } => {
                                    *yield_pos = None;
                                }
                                _ => (), // do nothin,external function
                            }
                        }
                        None => (), // do nothing
                    }
                    self.funs.pop();
                    self.push_ref(return_);
                }
                Yield => {
                    let return_ = catch!(self.pop());

                    match self.funs.last() {
                        Some(fun) => {
                            let fun: &mut Function = &mut fun.borrow_mut();
                            match fun {
                                Function::Regular {
                                    yield_pos,
                                    yield_env,
                                    ..
                                } => {
                                    match yield_pos {
                                        Some(ref mut pos) => *pos = self.pc,
                                        None => *yield_pos = Some(self.pc),
                                    };
                                    *yield_env = self.env.clone();
                                }
                                _ => unreachable!(),
                            }
                        }
                        None => throw!("can not find function state"),
                    }
                    self.restore_state(true, true, true, true);
                    self.push_ref(return_);
                }
                PushCatch(addr) => {
                    self.exception_stack.push(addr);
                }

                Throw => {
                    let error = match self.stack.pop() {
                        Some(val) => val,
                        None => new_ref(ValueData::Undefined), // TODO: probably be better to throw stack empty exception?
                    };
                    if let Some(location) = self.exception_stack.pop() {
                        self.pc = location;
                        self.push_ref(error);
                        continue;
                    } else {
                        eprintln!("{}", error.borrow());
                        std::process::exit(1);
                    }
                }

                Call(argc) => {
                    let mut args = vec![];
                    let function: Value = catch!(self.pop());
                    let this = catch!(self.pop());

                    for _ in 0..argc {
                        args.push(catch!(self.pop()));
                    }

                    let mauybe_function = function.borrow();
                    let maybe_function: &ValueData = &mauybe_function;
                    match maybe_function {
                        ValueData::Function(fun_) => {
                            let fun_2 = fun_.clone();
                            let fun: &Function = &fun_.borrow();
                            match fun {
                                Function::Native(addr) => {
                                    let fun: fn(
                                        &mut Self,
                                        Value,
                                        &[Value],
                                    )
                                        -> Result<Value, ValueData> =
                                        unsafe { std::mem::transmute(*addr) };

                                    let result = catch!(fun(self, this, &args));
                                    self.push_ref(result);
                                }
                                Function::Regular {
                                    environment,
                                    addr,
                                    yield_pos,
                                    code,
                                    args: args_,
                                    yield_env,
                                } => {
                                    self.funs.push(fun_2);
                                    match yield_pos {
                                        Some(ref pos) => {
                                            self.save_state(true, true, true, true);
                                            self.pc = *pos;

                                            if &self.code != code {
                                                self.code = code.clone();
                                            }
                                            self.env = yield_env.clone();
                                        }
                                        None => {
                                            self.save_state(true, true, true, true);
                                            self.pc = *addr;
                                            self.env = environment.clone();
                                        }
                                    }

                                    for (i, arg) in args_.iter().enumerate() {
                                        if var_declared(&environment, arg) {
                                            catch!(set_variable_in_scope(
                                                &environment,
                                                arg,
                                                args[i].clone(),
                                                &self.get_pos()
                                            ));
                                        } else {
                                            catch!(declare_var(
                                                &environment,
                                                arg,
                                                args[i].clone(),
                                                &self.get_pos()
                                            ))
                                        }
                                    }
                                    if var_declared(&environment, "this") {
                                        catch!(
                                            set_variable_in_scope(
                                                &environment, "this", this, &self.get_pos())
                                        );
                                    } else {
                                        catch!(
                                            declare_var(
                                                &environment, "this", this, &self.get_pos())
                                        );
                                    }
                                }
                            }
                        }
                        _ => throw!("function expected"),
                    }
                }
                PopCatch => {
                    self.exception_stack.pop();
                }
                Jump(to) => {
                    self.pc = to as usize;
                }
                JumpIf(to) => {
                    let val = catch!(self.pop());
                    let val = val.borrow().clone();
                    if bool::from(val) {
                        self.pc = to as usize;
                    }
                }
                JumpIfFalse(to) => {
                    let val = catch!(self.pop());
                    let val = val.borrow().clone();
                    if !bool::from(val) {
                        self.pc = to as usize;
                    }
                }
                InitEnv => {
                    let fun = catch!(self.pop());
                    let fun: &ValueData = &fun.borrow();

                    match fun {
                        ValueData::Function(fun) => {
                            let fun: &mut Function = &mut fun.borrow_mut();

                            match fun {
                                Function::Native(_) => {} // TODO: maybe we should throw exception there
                                Function::Regular { environment, .. } => {
                                    let env = new_object();
                                    set_obj_proto(env.clone(), self.env.clone());
                                    *environment = env;
                                }
                            }
                        }
                        _ => throw!("function expected"),
                    }
                    self.push(fun.clone());
                }
                PushEnv => self.push_env(),
                PopEnv => self.pop_env(),
                Label => (), // nothing to do,relax :D
                Add | Sub | Div | Mul | Rem | Shl | Shr | BitAnd | BitOr | BitXor | And | Or
                | Gt | Ge | Lt | Le | Eq | Ne => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    let lhs = lhs.borrow().clone();
                    let rhs = rhs.borrow().clone();
                    let result: ValueData = match opcode {
                        Add => lhs + rhs,
                        Sub => lhs - rhs,
                        Div => lhs / rhs,
                        Mul => lhs * rhs,
                        Rem => lhs % rhs,
                        Shl => lhs << rhs,
                        Shr => lhs >> rhs,
                        BitAnd => lhs & rhs,
                        BitOr => lhs | rhs,
                        BitXor => lhs ^ rhs,
                        And => ValueData::from(bool::from(lhs) && bool::from(rhs)),
                        Or => ValueData::from(bool::from(lhs) || bool::from(rhs)),
                        Gt => (lhs > rhs).into(),
                        Lt => (lhs < rhs).into(),
                        Le => (lhs <= rhs).into(),
                        Ge => (lhs >= rhs).into(),
                        Eq => (lhs == rhs).into(),
                        Ne => (lhs != rhs).into(),
                        _ => unreachable!(),
                    };
                    self.push(result);
                }
                _ => (),
            }
        }
    }
}
