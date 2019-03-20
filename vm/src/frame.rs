use crate::value::*;
use crate::vm::*;

pub struct Frame<'a>
{
    vm: &'a mut VirtualMachine,
    stack: Vec<GcValue>,
    locals: Vec<GcValue>,
    ip: usize,
    code: Vec<Instruction>,
    this: &'a GcValue,
    args: GcValue,
}

use crate::instruction::*;

impl<'a> Frame<'a>
{
    pub fn new(vm: &'a mut VirtualMachine,
               code: Vec<Instruction>,
               max_locals: u16,
               this: &'a GcValue)
               -> Frame<'a>
    {
        let locals = vec![GcValue::new(Value::Null); max_locals as usize];
        Frame { locals,
                stack: Vec::with_capacity(128),
                vm,
                ip: 0,
                code,
                this,
                args: GcValue::new(Value::Null) }
    }

    pub fn next_ins(&mut self) -> Instruction
    {
        let ins = self.code[self.ip].clone();
        self.ip += 1;
        ins
    }
    #[inline]
    pub fn pop(&mut self) -> GcValue
    {
        self.stack.pop().unwrap()
    }
    #[inline]
    pub fn push(&mut self, v: GcValue)
    {
        self.stack.push(v)
    }

    pub fn invoke(&mut self, val: &GcValue, this: GcValue, argc: usize) -> GcValue
    {
        let val: &Value = &val.get();
        let mut args = vec![];
        for _ in 0..argc
        {
            args.push(self.pop());
        }
        let args_v = GcValue::new(Value::Array(args.clone()));
        match val
        {
            Value::Func(f) => match &f.var
            {
                FuncVar::Native(ptr) =>
                {
                    use std::mem::transmute;
                    let f: fn(&mut Frame, Vec<GcValue>) -> GcValue = unsafe { transmute(*ptr) };

                    f(self, args)
                }
                FuncVar::Code(code, max_locals) =>
                {
                    let mut frame = Frame::new(self.vm, code.clone(), *max_locals, &this);
                    if args.len() > *max_locals as usize
                    {
                        frame.locals.resize(args.len(), GcValue::new(Value::Null));
                    }
                    for (idx, arg) in args.iter().enumerate()
                    {
                        frame.locals[idx] = arg.clone();
                    }
                    frame.args = args_v;

                    frame.run()
                }
            },

            v => panic!("Function expected {:?}", v),
        }
    }

    pub fn run(&mut self) -> GcValue
    {
        macro_rules! bin_op {
            (with_str $op:tt,$fname: expr /* function name in object */) => {
                {
                let x = self.pop();
                let y = self.pop();

                let x_ref: &Value = &x.get();
                let y_clon = y.clone();
                let y_ref: &Value = &y.get();
                let val = match (x_ref, y_ref) {
                    (Value::Int(i),Value::Int(i2)) => GcValue::new( Value::Int((*i) $op (*i2))),
                    (Value::Float(f),Value::Float(f2)) => GcValue::new(Value::Float((*f) $op (*f2))),
                    (Value::Int(i),Value::Float(f2)) => GcValue::new( Value::Float((*i as f64) $op (*f2))),
                    (Value::Float(f),Value::Int(i2)) => GcValue::new(Value::Float((*f) $op (*i2 as f64))),
                    (Value::Str(s),Value::Str(s2)) => GcValue::new(Value::Str(format!("{}{}",s,s2))),
                    (Value::Object(obj),_v) => {
                        let obj: &Object = obj;
                        let f = obj.find(&GcValue::new(Value::Str($fname.to_string())));
                        self.stack.push(y_clon);
                        self.invoke(f, x.clone(), 1)
                    }
                    (_v,Value::Object(obj)) => {
                        let obj: &Object = obj;
                        let f = obj.find(&GcValue::new(Value::Str($fname.to_string())));
                        self.stack.push(y_clon);
                        self.invoke(f, x.clone(), 1)
                    }
                value => panic!("op: {} {:?}",stringify!($op),value),
                };
                self.push(val);
                }
            };
            ($op: tt,$fname: expr) => {
                {
                let x = self.pop();
                let y = self.pop();

                let x_ref: &Value = &x.get();
                let y_clon = y.clone();
                let y_ref: &Value = &y.get();
                let val = match (x_ref, y_ref) {
                    (Value::Int(i),Value::Int(i2)) => GcValue::new( Value::Int((*i) $op (*i2))),
                    (Value::Float(f),Value::Float(f2)) => GcValue::new(Value::Float((*f) $op (*f2))),
                    (Value::Int(i),Value::Float(f2)) =>GcValue::new( Value::Float((*i as f64) $op (*f2))),
                    (Value::Float(f),Value::Int(i2)) => GcValue::new(Value::Float((*f) $op (*i2 as f64))),
                    //(Value::Str(s),Value::Str(s2)) => Value::Str(format!("{}{}",s,s2)),
                    (Value::Object(obj),_v) => {
                        let obj: &Object = obj;
                        let f = obj.find(&GcValue::new(Value::Str($fname.to_string())));
                        self.stack.push(y_clon);
                        self.invoke(f, x.clone(), 1)
                    }
                    (_v,Value::Object(obj)) => {
                        let obj: &Object = obj;
                        let f = obj.find(&GcValue::new(Value::Str($fname.to_string())));
                        self.stack.push(y_clon);
                        self.invoke(f, x.clone(), 1)
                    }
                    v => panic!("{:?}",v),
                };
                self.push(val);
                }
            };
            (cmp $op:tt,$fname: expr) => {
                {
                let x = self.pop();
                let y = self.pop();

                let x_ref: &Value = &x.get();
                let y_ref: &Value = &y.get();
                let val = match (x_ref, y_ref) {
                    (Value::Int(i),Value::Int(i2)) => Value::Bool((*i) $op (*i2)),
                    (Value::Float(f),Value::Float(f2)) => Value::Bool((*f) $op (*f2)),
                    (Value::Int(i),Value::Float(f2)) => Value::Bool((*i as f64) $op (*f2)),
                    (Value::Float(f),Value::Int(i2)) => Value::Bool((*f) $op (*i2 as f64)),
                    (Value::Str(s),Value::Str(s2)) => Value::Str(format!("{}{}",s,s2)),
                    (Value::Null,Value::Null) => Value::Bool(false),
                    (Value::Null,v) => Value::Bool(true),
                    (v,Value::Null) => Value::Bool(true),
                    _ => unimplemented!(),
                };
                self.push(GcValue::new(val));
                }
            };

            (cmpo $op:tt,$fname: expr) => {
                {
                let x = self.pop();
                let y = self.pop();

                let x_ref: &Value = &x.get();
                let y_clon = y.clone();
                let y_ref: &Value = &y.get();
                let val = match (x_ref, y_ref) {
                    (Value::Int(i),Value::Int(i2)) => GcValue::new(Value::Bool((*i) $op (*i2))),
                    (Value::Float(f),Value::Float(f2)) => GcValue::new(Value::Bool((*f) $op (*f2))),
                    (Value::Int(i),Value::Float(f2)) => GcValue::new(Value::Bool((*i as f64) $op (*f2))),
                    (Value::Float(f),Value::Int(i2)) => GcValue::new(Value::Bool((*f) $op (*i2 as f64))),
                    (Value::Str(s),Value::Str(s2)) => GcValue::new(Value::Bool(s $op s2)),
                    (Value::Null,_v) => GcValue::new(Value::Bool(true)),
                    (_v,Value::Null) => GcValue::new(Value::Bool(true)),
                    (Value::Array(a1),Value::Array(a2)) => GcValue::new(Value::Bool(a1 == a2)),
                    (Value::Object(obj),_v) => {
                        let obj: &Object = obj;
                        let f = obj.find(&GcValue::new(Value::Str($fname.to_string())));
                        self.stack.push(y_clon);
                        self.invoke(f, x.clone(), 1)
                    }
                    (_v,Value::Object(obj)) => {
                        let obj: &Object = obj;
                        let f = obj.find(&GcValue::new(Value::Str($fname.to_string())));
                        self.stack.push(y_clon);
                        self.invoke(f, x.clone(), 1)
                    }
                    _ => unimplemented!(),
                };
                self.push(val);
                }
            }

        }
        while self.ip < self.code.len()
        {
            let ins = self.next_ins();
            use Instruction::*;
            match &ins
            {
                Nop => (),
                LdArgs => self.push(self.args.clone()),
                LdNull => self.push(GcValue::new(Value::Null)),
                LdBool(b) => self.push(GcValue::new(Value::Bool(*b))),
                LdInt(i) => self.push(GcValue::new(Value::Int(*i))),
                LdFloat(f) => self.push(GcValue::new(Value::Float(f64::from_bits(*f)))),
                LdString(s) => self.push(GcValue::new(Value::Str(s.clone()))),
                New(argc) =>
                {
                    let val = self.pop();
                    let val_ref: &Value = &val.get();
                    if let Value::Object(obj) = val_ref
                    {
                        let new_o = obj.clone();
                        let f = obj.find(&GcValue::new(Value::Str("__init__".to_owned())));
                        let new_obj = GcValue::new(Value::Object(new_o));
                        let val: &Value = &f.get();
                        let mut args = vec![];
                        for _ in 0..*argc
                        {
                            args.push(self.pop());
                        }
                        let args_v = GcValue::new(Value::Array(args.clone()));
                        match val
                        {
                            Value::Func(f) => match &f.var
                            {
                                FuncVar::Native(ptr) =>
                                {
                                    use std::mem::transmute;
                                    let f: fn(&mut Frame, Vec<GcValue>) -> GcValue =
                                        unsafe { transmute(*ptr) };

                                    f(self, args);
                                }
                                FuncVar::Code(code, max_locals) =>
                                {
                                    let mut frame =
                                        Frame::new(self.vm, code.clone(), *max_locals, &new_obj);
                                    frame.args = args_v;
                                    if args.len() > *max_locals as usize
                                    {
                                        frame.locals.resize(args.len(), GcValue::new(Value::Null));
                                    }
                                    for (idx, arg) in args.iter().enumerate()
                                    {
                                        frame.locals[idx] = arg.clone();
                                    }

                                    frame.run();

                                    self.push(new_obj);
                                }
                            },

                            v => panic!("Function expected {:?}", v),
                        };
                    }
                    else
                    {
                        panic!("Object expected");
                    }
                }
                Ret => break,
                MakeArray(count) =>
                {
                    let mut buf = vec![];
                    for _ in 0..*count
                    {
                        buf.push(self.pop());
                    }
                    let arr = Value::Array(buf);
                    self.push(GcValue::new(arr));
                }
                LdFld =>
                {
                    let key = self.pop();
                    let obj = self.pop();
                    let obj_ref: &Value = &obj.get();
                    match obj_ref
                    {
                        Value::Object(obj) =>
                        {
                            let obj: &Object = obj;
                            let val = obj.find(&key);
                            self.push(val.clone());
                        }
                        _ => panic!("Object expected"),
                    }
                }
                StFld =>
                {
                    let obj = self.pop();
                    let key = self.pop();
                    let val = self.pop();
                    let value: &mut Value = &mut obj.get_mut();
                    match value
                    {
                        Value::Object(ref mut obj) => obj.insert(key, val),
                        v => panic!("field {:?},key {:?} \n\nval {:?}", v, key, val),
                    }
                }
                Dup =>
                {
                    let last = self.stack.last().unwrap();
                    self.push(last.clone());
                }
                Br(to) =>
                {
                    self.ip = *to as usize;
                    continue;
                }
                Brz(to) =>
                {
                    let val_p = self.pop();
                    let val: &Value = &val_p.get();
                    match val
                    {
                        Value::Int(0) | Value::Bool(false) => self.ip = *to as usize,
                        _ => (),
                    };
                    continue;
                }
                Brnz(to) =>
                {
                    let val_p = self.pop();
                    let val: &Value = &val_p.get();
                    match val
                    {
                        Value::Int(i) =>
                        {
                            if *i != 0
                            {
                                self.ip = *to as usize
                            }
                        }
                        Value::Bool(true) => self.ip = *to as usize,

                        _ => (),
                    }
                    continue;
                }
                Add => bin_op!(with_str+, "_add_"),
                Sub => bin_op!(-,"_sub_"),
                Div => bin_op!(/,"_div_"),
                Mul => bin_op!(*,"_mul_"),
                Rem => bin_op!(%,"_rem_"),
                Gt => bin_op!(cmpo>,"_gt_"),
                Lt => bin_op!(cmpo<,"_lt_"),
                Lte => bin_op!(cmpo<=,"_lte_"),
                Gte => bin_op!(cmpo>=,"_gte_"),
                Eq => bin_op!(cmpo==,"_eq_"),
                Neq => bin_op!(cmpo!=,"_neq_"),
                Neg =>
                {
                    let val = self.pop();
                    let val: &Value = &val.get();
                    let res = match val
                    {
                        Value::Int(i) => Value::Int(-(*i)),
                        Value::Float(f) => Value::Float(-(*f)),
                        _ => unimplemented!(),
                    };
                    self.push(GcValue::new(res));
                }
                Invoke(argc) =>
                {
                    let f = self.pop();
                    let this = self.pop();
                    let res = self.invoke(&f, this, *argc as usize);
                    self.push(res);
                }
                LdGlob(idx) =>
                {
                    let val = self.vm.globals[idx].clone();
                    self.push(val);
                }
                StGlob(idx) =>
                {
                    let val = self.pop();
                    self.vm.globals.insert(*idx, val);
                }
                LdLoc(id) =>
                {
                    let val = self.locals[*id as usize].clone();
                    self.push(val);
                }
                StLoc(id) =>
                {
                    let val = self.pop();
                    self.locals[*id as usize] = val;
                }
                LdThis =>
                {
                    let val = self.this.clone();
                    self.push(val);
                }
                _ => unimplemented!(),
            }
        }
        self.stack.pop().unwrap_or(GcValue::new(Value::Null))
    }
}
