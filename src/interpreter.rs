use crate::*;
use bytecode::*;
use thread::*;
use value::*;

impl JThread {
    pub fn run(&mut self, mut module: Gc<Module>) -> Value {
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
                        if self.exceptions.is_empty() {
                            eprintln!("Exception: {}", e);
                            std::process::exit(1);
                        } else {
                            if let Some(FrameData::Frame {
                                pc,
                                locals,
                                this,
                                env,
                                module: m,
                                ctor_call: _,
                            }) = self.exceptions.pop()
                            {
                                self.pc = pc;
                                match m {
                                    Some(m) => module = m,
                                    _ => (),
                                }
                                self.env = env;
                                self.this = this;
                                self.locals = locals;
                                self.push(e);
                                continue;
                            } else {
                                crate::unreachable()
                            }
                        }
                    }
                }
            };
        }
        'inner: while self.pc < module.get().code.len() {
            let op = unsafe { module.get().code.get_unchecked(self.pc).clone() };
            self.pc += 1;
            use Op::*;
            match op {
                ConstNull => self.push(Value::Null),
                ConstInt(i) => self.push(Value::Number(i as _)),
                ConstTrue => self.push(Value::Bool(true)),
                ConstFalse => self.push(Value::Bool(false)),
                LoadLocal(idx) => {
                    let val = self.locals.get().get(&idx).cloned();
                    if val.is_none() {
                        throw!(Value::String(Gc::new(
                            "Local variable not found".to_owned()
                        )));
                    } else {
                        self.push(val.unwrap());
                    }
                }
                LoadGlobal(idx) => {
                    let global = module.get().globals.get(idx as usize).cloned();
                    let new_string = Gc::new("".to_owned());
                    match global {
                        Some(val) => match val {
                            Value::String(s) => {
                                *new_string.get_mut() = s.get().clone();
                                self.push(Value::String(new_string));
                            }
                            _ => self.push(val),
                        },
                        None => self.push(Value::Null),
                    }
                }
                Nop => (),
                LoadStatic => {
                    let value = catch!(self.pop());
                    let state: parking_lot::MutexGuard<GlobalState> = STATE.lock();
                    let var = state
                        .static_variables
                        .get(&value)
                        .cloned()
                        .unwrap_or(Value::Null);
                    self.push(var);
                }
                LoadField => {
                    let value = catch!(self.pop());
                    let object: Value = catch!(value.to_object());
                    match object {
                        Value::Object(object) => {
                            let key = catch!(self.pop());

                            let property = object.get().get_property(key);
                            if let Some(property) = property {
                                self.push(property.value.clone());
                            } else {
                                self.push(Value::Null);
                            }
                        }
                        _ => crate::unreachable(),
                    }
                }
                StoreField => {
                    let object = catch!(self.pop());
                    let field = catch!(self.pop());
                    let value = catch!(self.pop());
                    let object = catch!(object.to_object());

                    match object {
                        Value::Object(obj) => {
                            obj.get_mut().set_property(field, value);
                        }
                        _ => crate::unreachable(),
                    }
                }
                LoadEnv(var) | StoreEnv(var) => {
                    let obj = self.env.unwrap_object();
                    let obj = obj.get();
                    match &obj.kind {
                        ObjectKind::Array(array) => {
                            #[cfg(debug_assertions)]
                            {
                                if var as usize >= array.get().len() {
                                    panic!("Internal error");
                                }
                            }
                            if let StoreEnv(_) = op {
                                array.get_mut()[var as usize] = self.stack.last().cloned().unwrap();
                            } else {
                                unsafe {
                                    let array = array.get();
                                    let value: &Value = array.get_unchecked(var as usize);
                                    self.push(value.clone());
                                }
                            }
                        }
                        _ => crate::unreachable(),
                    }
                }
                StoreLocal(idx) => {
                    let value = catch!(self.pop());
                    self.locals.get_mut().insert(idx as _, value);
                }
                StoreStatic => {
                    let key = catch!(self.pop());
                    let value = catch!(self.pop());
                    let mut state = STATE.lock();
                    state.static_variables.insert(key, value);
                }
                New => {
                    let state = STATE.lock();
                    let proto = state
                        .static_variables
                        .get(&Value::String(Gc::new("Object".to_owned())))
                        .unwrap()
                        .unwrap_object();
                    let object = Gc::new(Object {
                        kind: ObjectKind::Ordinary,
                        proto: Some(proto),
                        properties: Gc::new(vec![]),
                    });

                    self.push(Value::Object(object));
                }
                Ctor(argc) => {
                    let object_proto = Gc::new(Object {
                        kind: ObjectKind::Ordinary,
                        proto: None,
                        properties: Gc::new(vec![]),
                    });
                    let value = catch!(self.pop());
                    let mut args = vec![];
                    for _ in 0..argc {
                        args.push(catch!(self.pop()));
                    }
                    if let Value::Object(object) = &value {
                        if let ObjectKind::Function(func) = &object.get().kind {
                            let func = func.get();
                            if func.argc != -1 {
                                if args.len() > func.argc as usize
                                    || args.len() < func.argc as usize
                                {
                                    throw!(Value::String(Gc::new(format!(
                                        "Function takes {} positional arguments but {} were given",
                                        func.argc,
                                        args.len()
                                    ))));
                                }
                            }
                            object_proto.get_mut().proto =
                                Some(match func.prototype.to_object().unwrap() {
                                    Value::Object(obj) => obj,
                                    _ => unsafe { std::hint::unreachable_unchecked() },
                                });
                            if func.is_native {
                                let fun: extern "C" fn(Value, &[Value]) -> Result<Value, Value> =
                                    unsafe { std::mem::transmute(func.addr) };

                                let result = catch!(fun(Value::Object(object_proto), &args));
                                self.push(result);
                            } else {
                                self.push_frame(Some(module), true);
                                self.env = func.env.clone();
                                self.locals = Gc::new(HashMap::new());
                                self.this = Value::Object(object_proto);
                                self.pc = func.addr;
                                module = func.module.clone().unwrap();
                                for (i, arg) in args.iter().enumerate() {
                                    self.locals.get_mut().insert(i as _, arg.clone());
                                }
                            }
                        } else {
                            let string = Gc::new("constructor".to_owned());
                            let property = object.get().get_property(Value::String(string));
                            if let Some(ctor) = property {
                                if let Value::Object(object) = ctor.value {
                                    if let ObjectKind::Function(func) = object.get().kind.clone() {
                                        let func = func.get();
                                        object_proto.get_mut().proto = Some(object.clone());
                                        if func.is_native {
                                            let fun: extern "C" fn(
                                                Value,
                                                &[Value],
                                            )
                                                -> Result<Value, Value> =
                                                unsafe { std::mem::transmute(func.addr) };
                                            let result =
                                                catch!(fun(Value::Object(object_proto), &args));
                                            self.push(result);
                                        } else {
                                            self.push_frame(Some(module), true);
                                            self.env = func.env.clone();
                                            self.locals = Gc::new(HashMap::new());
                                            self.this = Value::Object(object_proto);
                                            self.pc = func.addr;
                                            module = func.module.clone().unwrap();
                                            for (i, arg) in args.iter().enumerate() {
                                                self.locals.get_mut().insert(i as _, arg.clone());
                                            }
                                        }
                                    }
                                }
                            } else {
                                throw!(Value::String(Gc::new(format!(
                                    "Function expected,found {}",
                                    value
                                ))));
                            }
                        }
                    } else {
                        throw!(Value::String(Gc::new("Function expected".to_owned())));
                    }
                }
                Invoke(argc) | TailRec(argc) => {
                    let mut args = vec![];
                    let value = self.pop().unwrap();
                    for _ in 0..argc {
                        args.push(self.pop().unwrap());
                    }
                    if let Value::Object(object) = value {
                        if let ObjectKind::Function(func) = &object.get().kind {
                            let func = func.get();
                            if func.argc != -1 {
                                if args.len() > func.argc as usize {
                                    throw!(Value::String(
                                        Gc::new("Too many arguments".to_owned(),)
                                    ));
                                } else if args.len() < func.argc as usize {
                                    throw!(Value::String(
                                        Gc::new("Too many arguments".to_owned(),)
                                    ));
                                }
                            }
                            if func.is_native {
                                let fun: extern "C" fn(Value, &[Value]) -> Result<Value, Value> =
                                    unsafe { std::mem::transmute(func.addr) };
                                let result = catch!(fun(Value::Null, &args));
                                self.push(result);
                            } else {
                                if let TailRec(_) = op {
                                    self.pop_frame(Some(&mut module));
                                }
                                self.push_frame(Some(module), false);
                                self.env = func.env.clone();
                                self.locals = Gc::new(HashMap::new());
                                self.this = Value::Null;
                                self.pc = func.addr;
                                module = func.module.clone().unwrap();
                                for (i, arg) in args.iter().enumerate() {
                                    self.locals.get_mut().insert(i as _, arg.clone());
                                }
                            }
                        } else {
                            throw!(Value::String(Gc::new(format!(
                                "Function expected at {:04}",
                                self.pc - 1
                            ))));
                        }
                    } else {
                        throw!(Value::String(Gc::new("Function expected".to_owned())));
                    }
                }
                MakeArray(count) => {
                    let mut values = vec![];

                    for _ in 0..count {
                        values.push(catch!(self.pop()));
                    }
                    let array = Gc::new(values);
                    let state = STATE.lock();
                    let array_proto = state
                        .static_variables
                        .get(&Value::String(Gc::new("Array".to_owned())))
                        .unwrap()
                        .unwrap_object();
                    let object = Object {
                        proto: Some(array_proto),
                        kind: ObjectKind::Array(array),
                        properties: Gc::new(vec![]),
                    };
                    self.push(Value::Object(Gc::new(object)));
                }
                InvokeVirtual(argc) => {
                    let value = catch!(self.pop());
                    let this = catch!(self.pop());
                    let mut args = vec![];
                    for _ in 0..argc {
                        args.push(catch!(self.pop()));
                    }
                    if let Value::Object(object) = value {
                        if let ObjectKind::Function(func) = &object.get().kind {
                            let func = func.get();
                            if func.argc != -1 {
                                if args.len() > func.argc as usize {
                                    throw!(Value::String(
                                        Gc::new("Too many arguments".to_owned(),)
                                    ));
                                } else if args.len() < func.argc as usize {
                                    throw!(Value::String(
                                        Gc::new("Too many arguments".to_owned(),)
                                    ));
                                }
                            }
                            if func.is_native {
                                let fun: extern "C" fn(Value, &[Value]) -> Result<Value, Value> =
                                    unsafe { std::mem::transmute(func.addr) };

                                let result = catch!(fun(this, &args));
                                self.push(result);
                            } else {
                                self.push_frame(Some(module), false);
                                self.env = func.env.clone();
                                self.this = this;
                                self.locals = Gc::new(HashMap::new());
                                self.pc = func.addr;
                                module = func.module.clone().unwrap();
                                for (i, arg) in args.iter().enumerate() {
                                    self.locals.get_mut().insert(i as _, arg.clone());
                                }
                            }
                        } else {
                            throw!(Value::String(Gc::new("Function expected".to_owned())));
                        }
                    } else {
                        throw!(Value::String(Gc::new("Function expected".to_owned())));
                    }
                }
                Return => {
                    let this = self.this.clone();
                    let (exit, ctor) = self.pop_frame(Some(&mut module));
                    if exit {
                        return match self.pop() {
                            Ok(val) => val,
                            Err(_) => Value::Null,
                        };
                    } else {
                        if ctor {
                            self.push(this);
                        } else {
                            if self.stack.is_empty() {
                                self.push(Value::Null);
                            }
                        }
                    }
                }
                CatchIp(ip) => {
                    let frame = FrameData::Frame {
                        module: Some(module.clone()),
                        pc: ip as _,
                        env: self.env.clone(),
                        locals: self.locals.clone(),
                        this: self.this.clone(),
                        ctor_call: false,
                    };
                    self.exceptions.push(frame);
                }
                Throw => {
                    let value = catch!(self.pop());
                    catch!(Err(value));
                }
                MakeEnv(count) => {
                    let function = self.stack.last().cloned().unwrap();
                    if let Value::Object(object) = &function {
                        if let ObjectKind::Function(func) = &object.get().kind {
                            self.stack.pop();
                            let values = (0..count)
                                .into_iter()
                                .map(|_| self.pop().unwrap_or(Value::Null))
                                .collect::<Vec<Value>>();
                            if let Value::Object(object) = &func.get().env {
                                if let ObjectKind::Array(array) = &object.get().kind {
                                    array.get_mut().extend(values);
                                }
                            } else {
                                crate::unreachable()
                            }
                        } else {
                            crate::unreachable()
                        }
                    } else {
                        crate::unreachable()
                    }
                    self.stack.push(function);
                }
                Add => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::String(x) => {
                            self.push(Value::String(Gc::new(format!("{}{}", x, rhs))))
                        }
                        Value::Number(x) => match rhs {
                            Value::Number(y) => self.push(Value::Number(x + y)),
                            _ => self.push(Value::Null),
                        },
                        _ => self.push(Value::Null),
                    }
                }
                Sub => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::Number(x) => match rhs {
                            Value::Number(y) => self.push(Value::Number(x - y)),
                            _ => self.push(Value::Null),
                        },
                        _ => self.push(Value::Null),
                    }
                }
                Div => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::Number(x) => match rhs {
                            Value::Number(y) => self.push(Value::Number(x / y)),
                            _ => self.push(Value::Null),
                        },
                        _ => self.push(Value::Null),
                    }
                }
                Mul => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::Number(x) => match rhs {
                            Value::Number(y) => self.push(Value::Number(x * y)),
                            _ => self.push(Value::Null),
                        },
                        _ => self.push(Value::Null),
                    }
                }
                Rem => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::Number(x) => match rhs {
                            Value::Number(y) => self.push(Value::Number(x % y)),
                            _ => self.push(Value::Null),
                        },
                        _ => self.push(Value::Null),
                    }
                }
                Shr => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::Number(x) => match rhs {
                            Value::Number(y) => {
                                self.push(Value::Number(((x as i64) >> (y as i64)) as f64))
                            }
                            _ => self.push(Value::Null),
                        },
                        _ => self.push(Value::Null),
                    }
                }
                Shl => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::Number(x) => match rhs {
                            Value::Number(y) => {
                                self.push(Value::Number(((x as i64) << (y as i64)) as f64))
                            }
                            _ => self.push(Value::Null),
                        },
                        _ => self.push(Value::Null),
                    }
                }
                BitAnd => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::Number(x) => match rhs {
                            Value::Number(y) => {
                                self.push(Value::Number(((x as i64) & (y as i64)) as f64))
                            }
                            _ => self.push(Value::Null),
                        },
                        Value::Bool(x) => match rhs {
                            Value::Bool(y) => self.push(Value::Bool(x & y)),
                            _ => self.push(Value::Null),
                        },
                        _ => self.push(Value::Null),
                    }
                }
                BitOr => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::Number(x) => match rhs {
                            Value::Number(y) => {
                                self.push(Value::Number(((x as i64) & (y as i64)) as f64))
                            }
                            _ => self.push(Value::Null),
                        },
                        Value::Bool(x) => match rhs {
                            Value::Bool(y) => self.push(Value::Bool(x | y)),
                            _ => self.push(Value::Null),
                        },
                        _ => self.push(Value::Null),
                    }
                }
                BitXor => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::Number(x) => match rhs {
                            Value::Number(y) => {
                                self.push(Value::Number(((x as i64) ^ (y as i64)) as f64))
                            }
                            _ => self.push(Value::Null),
                        },
                        _ => self.push(Value::Null),
                    }
                }
                Neg => {
                    let value = catch!(self.pop());
                    match value {
                        Value::Number(x) => self.push(Value::Number(-x)),
                        _ => self.push(Value::Null),
                    }
                }
                Not => {
                    let value = catch!(self.pop());
                    match value {
                        Value::Number(x) => self.push(Value::Number((!(x as i64)) as f64)),
                        _ => self.push(Value::Null),
                    }
                }
                CmpEq => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());

                    self.push(Value::Bool(lhs == rhs));
                }
                CmpNeq => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());

                    self.push(Value::Bool(lhs != rhs));
                }
                CmpGt => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::Number(x) => match rhs {
                            Value::Number(y) => self.push(Value::Bool(x > y)),
                            _ => self.push(Value::Bool(false)),
                        },
                        Value::Object(x) => match rhs {
                            Value::Object(y) => match &x.get().kind {
                                ObjectKind::Array(array) => match &y.get().kind {
                                    ObjectKind::Array(yarray) => self
                                        .push(Value::Bool(array.get().len() > yarray.get().len())),
                                    _ => self.push(Value::Bool(false)),
                                },
                                _ => self.push(Value::Bool(false)),
                            },
                            _ => self.push(Value::Bool(false)),
                        },
                        Value::String(x) => match rhs {
                            Value::String(y) => {
                                self.push(Value::Bool(x.get().len() > y.get().len()))
                            }
                            _ => self.push(Value::Bool(false)),
                        },
                        _ => self.push(Value::Bool(false)),
                    }
                }
                CmpGe => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::Number(x) => match rhs {
                            Value::Number(y) => self.push(Value::Bool(x >= y)),
                            _ => self.push(Value::Bool(false)),
                        },
                        Value::String(x) => match rhs {
                            Value::String(y) => self.push(Value::Bool(x >= y)),
                            _ => self.push(Value::Bool(false)),
                        },
                        _ => self.push(Value::Bool(false)),
                    }
                }
                CmpLe => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::Number(x) => match rhs {
                            Value::Number(y) => self.push(Value::Bool(x <= y)),
                            _ => self.push(Value::Bool(false)),
                        },
                        Value::String(x) => match rhs {
                            Value::String(y) => self.push(Value::Bool(x <= y)),
                            _ => self.push(Value::Bool(false)),
                        },
                        _ => self.push(Value::Bool(false)),
                    }
                }
                CmpLt => {
                    let lhs = catch!(self.pop());
                    let rhs = catch!(self.pop());
                    match lhs {
                        Value::Number(x) => match rhs {
                            Value::Number(y) => self.push(Value::Bool(x < y)),
                            _ => self.push(Value::Bool(false)),
                        },
                        Value::Object(x) => match rhs {
                            Value::Object(y) => match &x.get().kind {
                                ObjectKind::Array(array) => match &y.get().kind {
                                    ObjectKind::Array(yarray) => self
                                        .push(Value::Bool(array.get().len() < yarray.get().len())),
                                    _ => self.push(Value::Bool(false)),
                                },
                                _ => self.push(Value::Bool(false)),
                            },
                            _ => self.push(Value::Bool(false)),
                        },
                        Value::String(x) => match rhs {
                            Value::String(y) => {
                                self.push(Value::Bool(x.get().len() < y.get().len()))
                            }
                            _ => self.push(Value::Bool(false)),
                        },
                        _ => self.push(Value::Bool(false)),
                    }
                }
                Branch(to) => {
                    self.pc = to as _;
                }
                BranchIfTrue(to) => {
                    let val = catch!(self.pop());
                    match val {
                        Value::Bool(true) => self.pc = to as _,
                        Value::Null => (),
                        Value::Number(x) => {
                            if x != 0.0 && !x.is_infinite() && !x.is_nan() {
                                self.pc = to as _;
                            }
                        }
                        _ => self.pc = to as _,
                    }
                }
                BranchIfFalse(to) => {
                    let val = catch!(self.pop());
                    match val {
                        Value::Bool(false) => self.pc = to as _,
                        Value::Null => self.pc = to as _,
                        Value::Number(x) => {
                            if x == 0.0 || x.is_infinite() || x.is_nan() {
                                self.pc = to as _;
                            }
                        }
                        _ => (),
                    }
                }
                LoadThis => self.push(self.this.clone()),
                StoreThis => {
                    let last = self.stack.last().unwrap().clone();
                    self.this = last;
                    let _ = self.pop();
                }
                _ => unimplemented!(),
            }
        }

        match self.pop() {
            Ok(val) => val,
            Err(_) => Value::Null,
        }
    }
}

pub fn call_value(value: Value, this: Value, args: &[Value]) -> Result<Value, Value> {
    if let Value::Object(object) = value {
        if let ObjectKind::Function(func) = &object.get().kind {
            let fun = func.get();
            if func.get().argc != -1 {
                if args.len() > fun.argc as usize || args.len() < fun.argc as usize {
                    return Err(Value::String(Gc::new(format!(
                        "Function takes {} positional arguments but {} were given",
                        fun.argc,
                        args.len()
                    ))));
                }
            }
            if fun.is_native {
                let fun: extern "C" fn(Value, &[Value]) -> Result<Value, Value> =
                    unsafe { std::mem::transmute(fun.addr) };

                return fun(this, args);
            } else {
                let val = THREAD.with(|thread| {
                    let thread = thread.borrow();
                    let thread: &mut JThread = &mut thread.get_mut();
                    let pc = thread.pc;
                    let locals = thread.locals.clone();
                    let env = thread.env.clone();
                    let tthis = thread.this.clone();
                    thread.pc = fun.addr;
                    thread.locals = Gc::new(HashMap::new());
                    for i in 0..args.len() {
                        thread.locals.get_mut().insert(i as _, args[i].clone());
                    }

                    thread.this = this;
                    thread.env = fun.env.clone();
                    thread.exit_frame();

                    let value = thread.run(fun.module.as_ref().unwrap().clone());
                    thread.pc = pc;
                    thread.locals = locals;
                    thread.env = env;
                    thread.this = tthis;
                    return Ok(value);
                });
                return val;
            }
        } else {
            return Err(Value::String(Gc::new("Function expected".to_owned())));
        }
    } else {
        return Err(Value::String(Gc::new("Function expected".to_owned())));
    }
}
