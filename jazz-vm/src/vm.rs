use crate::module::*;
use crate::opcode::Opcode;
use crate::value::*;
use crate::P;

pub enum CSPVal {
    Pc(usize),
    Module(P<Module>),
    Val(P<Value>),
    Locals(fnv::FnvHashMap<u32, P<Value>>),
    Stack(Vec<P<Value>>),
}

use crate::fields::*;
use crate::Cell;
lazy_static::lazy_static! {
    pub static ref FIELDS: Cell<fnv::FnvHashMap<u64,String>> = Cell::new(fnv::FnvHashMap::default());
}

pub struct VM {
    pub pc: usize,
    pub code: Vec<Opcode>,
    pub stack: Vec<P<Value>>,
    pub csp: Vec<CSPVal>,
    pub env: P<Value>,
    pub vthis: P<Value>,
    pub builtins: Vec<P<Value>>,
    pub sp: usize,
    pub locals: fnv::FnvHashMap<u32, P<Value>>,
}

macro_rules! push_infos {
    ($vm: expr,$m: expr) => {
        let vthis = $vm.vthis.clone();
        let pc = $vm.pc;
        let env = $vm.env.clone();
        let locals = $vm.locals.clone();

        $vm.csp.push(CSPVal::Pc(pc));
        $vm.csp.push(CSPVal::Val(env));
        $vm.csp.push(CSPVal::Val(vthis));
        $vm.csp.push(CSPVal::Module($m.clone()));
        $vm.csp.push(CSPVal::Locals(locals));
        //$vm.csp.push(CSPVal::Stack(stack));
    };
}

macro_rules! pop_infos {
    ($restpc: expr,$m: expr,$vm: expr) => {
        /*if let Some(CSPVal::Stack(stack)) = $vm.csp.pop() {
            $vm.stack = stack;
        }*/
        if let Some(CSPVal::Locals(locals)) = $vm.csp.pop() {
            $vm.locals = locals;
        }
        if let Some(CSPVal::Module(module)) = $vm.csp.pop() {
            *$m = module;
        }
        if let Some(CSPVal::Val(vthis)) = $vm.csp.pop() {
            $vm.vthis = vthis;
        }
        if let Some(CSPVal::Val(env)) = $vm.csp.pop() {
            $vm.env = env;
        }
        if let Some(CSPVal::Pc(pc)) = $vm.csp.pop() {
            $vm.pc = pc;
        }
    };
}

macro_rules! pop_macro {
    ($vm: expr,$count: expr) => {
        let mut tmp = $count;
        while tmp > 0 {
            $vm.pop();
            tmp -= 1;
        }
    };
}

#[allow(non_camel_case_types)]
pub type jazz_func = extern "C" fn(&mut VM, Vec<P<Value>>) -> P<Value>;

macro_rules! do_call {
    ($acc: expr,$vm: expr,$m: expr,$this: expr,$argc: expr) => {
        if val_is_func(&$acc) {
            push_infos!($vm, $m);
            let f = val_func(&$acc);
            let fun: &Function = f.borrow();
            $vm.env = fun.env.clone();
            *$m = fun.module.clone();
            $vm.vthis = $this;

            match &fun.var {
                FuncVar::Offset(off) => {
                    let mut args = vec![];
                    for _ in 0..$argc {
                        args.push($vm.pop().expect("Stack empty"));
                    }

                    for (idx, arg) in args.iter().enumerate() {
                        $vm.locals.insert(idx as u32, arg.clone());
                    }
                    $vm.pc = *off;
                }
                FuncVar::Native(ptr) => {
                    let f: jazz_func = unsafe { std::mem::transmute(*ptr) };
                    let mut args = vec![];

                    for _ in 0..$argc {
                        args.push($vm.pop().expect("Stack empty. <native call>"));
                    }
                    let v = f($vm, args);
                    pop_infos!(true, $m, $vm);
                    $vm.push(v);
                }
            }
        } else {
            panic!("Invalid call");
        }
    };
}

macro_rules! object_op_gen {
    ($vm: expr,$obj: expr,$param: expr,$id: expr,$err: expr,$m: expr) => {{
        let o = $obj;
        let ob = val_object(&o);
        let obj = ob.borrow();
        let arg = $param;
        let f = obj.find($id).clone();

        if f.is_none() {
            $err;
        } else {
            let f = f.unwrap();
            push_infos!($vm, $m);
            $vm.push(arg.clone());
            do_call!(f, $vm, $m, o, 1);
        }
    }};
}

macro_rules! object_op {
    ($vm: expr,$obj: expr,$param: expr,$id: expr,$m: expr) => {
        object_op_gen!($vm, $obj, $param, $id, panic!("Unsupported operation"), $m);
    };
}

macro_rules! op_ {
    ($op: tt,$vm: expr,$m: expr,$id: expr) => {{

        assert!($vm.stack.len() >= 2,stringify!($op));
        let acc = $vm.pop().expect("Stack empty");
        let val = $vm.pop().expect("Stack empty");
        let acc_c = acc.clone();
        let val_c = val.clone();
        match (acc.borrow(), val.borrow()) {
            (Value::Int(i), Value::Int(i2)) => $vm.push(P(Value::Int(i $op i2))),
            (Value::Float(f), Value::Float(f2)) => $vm.push(P(Value::Float(f $op f2))),
            (Value::Int(i), Value::Float(f2)) => $vm.push(P(Value::Float(*i as f64 $op *f2))),
            (Value::Float(f1), Value::Int(i)) => $vm.push(P(Value::Float(*f1 $op *i as f64))),
            //(Value::Str(s), Value::Str(s2)) => P(Value::Str(format!("{}{}", s, s2))),
            (Value::Object(_), _) =>
            {
                let _val = object_op!($vm, acc_c, val_c, unsafe { $id }, $m);
                //$vm.push(val);
            },
            (_, Value::Object(_)) => {
                let _val = object_op!($vm, val_c, acc_c, unsafe { $id }, $m);
                //$vm.push(val);
            }
            _ => unimplemented!(),
        };
    }};
}

macro_rules! cmp {
    ($op: tt,$vm: expr,$m: expr,$id: expr) => {
        {


        let v1 = $vm.pop().expect("Stack empty");
        let v2 = $vm.pop().expect("Stack empty");
        let val = v2.clone();
        let acc_c = v1.clone();

        let v = v1.borrow();
        let acc = v2.borrow();
        match (v,acc) {
            (Value::Int(i),Value::Int(i2)) => $vm.push(P(Value::Bool(i $op i2))),
            (Value::Int32(i),Value::Int32(i2)) => $vm.push(P(Value::Bool(i $op i2))),
            (Value::Int(i),Value::Int32(i2)) => $vm.push(P(Value::Bool(*i $op *i2 as i64))),
            (Value::Int32(i),Value::Int(i2)) => $vm.push(P(Value::Bool((*i as i64) $op *i2))),
            (Value::Int(i),Value::Float(f)) => $vm.push(P(Value::Bool((*i as f64) $op *f))),
            (Value::Int32(i),Value::Float(f)) => $vm.push(P(Value::Bool((*i as f64) $op *f))),
            (Value::Float(f), Value::Int(i)) => $vm.push(P(Value::Bool(*f $op *i as f64))),
            (Value::Float(f),Value::Int32(i)) => $vm.push(P(Value::Bool(*f $op *i as f64))),
            (Value::Float(f),Value::Float(f2)) => $vm.push(P(Value::Bool(*f $op *f2))),
            (Value::Str(s1),Value::Str(s2)) => $vm.push(P(Value::Bool(*s1 $op *s2))),
            (Value::Array(a1),Value::Array(a2)) => {
                let a1 = a1.borrow();
                let a2 = a2.borrow();
                $vm.push(P(Value::Bool(a1.len() $op a2.len())))
            }
            (Value::Bool(b),Value::Bool(b1)) => $vm.push(P(Value::Bool((*b as u8) $op *b1 as u8))),
            (Value::Object(_),_) => {
                let _val = object_op!($vm,acc_c,val,unsafe {$id},$m);
                //$vm.push(val);
            }
            (_,Value::Object(_)) =>
            {
                let _val = object_op!($vm, val, acc_c, unsafe { $id }, $m);
                //$vm.push(val);
            }
            _ => unimplemented!()
        };

        }
    };
}

use parking_lot::Mutex;
lazy_static::lazy_static! {
    pub static ref VM_THREAD: Mutex<VM> = Mutex::new(VM::new());
}
#[macro_export]
macro_rules! jazz_vm {
    () => {
        &mut VM_THREAD.lock()
    };
}

pub fn callex(vthis: P<Value>, f: P<Value>, args: Vec<P<Value>>) -> P<Value> {
    let vm = jazz_vm!();
    let old_this = vm.vthis.clone();
    let old_env = vm.env.clone();
    let mut ret = P(Value::Null);

    vm.vthis = vthis;
    if val_is_int(&f) {
        panic!("Invalid call");
    }
    if val_is_func(&f) {
        let f = val_func(&f);
        let func: &mut Function = f.borrow_mut();
        vm.env = func.env.clone();
        match &func.var {
            FuncVar::Native(ptr) => {
                let nf: jazz_func = unsafe { std::mem::transmute(*ptr) };
                ret = nf(vm, args);
            }
            FuncVar::Offset(off) => {
                if args.len() as i32 == func.nargs {
                    for n in 0..args.len() {
                        vm.locals.insert(n as _, args[n].clone());
                    }
                    vm.pc = *off as usize - 1;
                    ret = vm.interp(&mut func.module);
                }
            }
        }
    } else {
        panic!("Invalid call");
    }
    vm.vthis = old_this;
    vm.env = old_env;

    return ret;
}

unsafe impl Sync for VM {}
unsafe impl Send for VM {}
impl VM {
    pub fn new() -> VM {
        VM {
            stack: vec![],
            csp: vec![],
            pc: 0,
            code: vec![],
            builtins: vec![],
            env: P(Value::Array(P(vec![]))),
            vthis: P(Value::Null),
            sp: 0,
            locals: fnv::FnvHashMap::default(),
        }
    }
    pub fn push(&mut self, val: P<Value>) {
        self.sp = self.stack.len();
        self.stack.push(val);
    }

    pub fn pop(&mut self) -> Option<P<Value>> {
        let val = self.stack.pop();
        self.sp = self.stack.len();
        val
    }

    fn next_op(&mut self) -> Opcode {
        let op = self.code[self.pc].clone();
        self.pc += 1;
        return op;
    }

    pub fn interp(&mut self, m: &mut P<Module>) -> P<Value> {
        while self.pc < self.code.len() {
            use Opcode::*;

            let op = self.next_op();

            match op {
                LdNull => self.push(P(Value::Null)),
                LdFloat(f) => self.push(P(Value::Float(f))),
                LdStr(s) => self.push(P(Value::Str(s))),
                LdInt(i) => {
                    self.push(P(Value::Int(i)));
                }
                LdTrue => self.push(P(Value::Bool(true))),
                LdFalse => self.push(P(Value::Bool(false))),
                LdThis => self.push(self.vthis.clone()),

                LdLocal(idx) => {
                    self.push(self.locals.get(&idx).unwrap_or(&P(Value::Null)).clone());
                }
                LdGlobal(idx) => {
                    self.push(m.globals[idx as usize].clone());
                }
                LdEnv(at) => {
                    let env = val_array(&self.env);
                    let env = env.borrow_mut();
                    if at >= env.len() as u32 {
                        panic!("Reading outside env");
                    }
                    self.push(env[at as usize].clone());
                }
                Neg => {
                    let v = self.pop().unwrap();
                    let val = match v.borrow() {
                        Value::Int(i) => Value::Int(-i),
                        Value::Int32(i) => Value::Int32(-i),
                        Value::Float(f) => Value::Float(-f),
                        _ => unimplemented!(),
                    };
                    self.push(P(val));
                }
                LdField(field) => {
                    let acc = self.pop().unwrap();

                    let obj_p = val_object(&acc);
                    let obj: &Object = obj_p.borrow();
                    let f = obj.find(field as i64);
                    if f.is_some() {
                        self.push(f.unwrap().clone());
                    } else {
                        self.push(P(Value::Null));
                    }
                }
                LdArray => {
                    let acc = self.pop().unwrap();
                    let arr_p = self.pop().unwrap();
                    if (val_is_int(&acc) || val_is_int32(&acc)) && val_is_array(&arr_p) {
                        let k = val_int(&acc);
                        let arr = val_array(&arr_p);
                        let arr: &Vec<P<Value>> = arr.borrow();
                        if k < 0 || k as usize > arr.len() {
                            self.push(P(Value::Null));
                        } else {
                            self.push(arr.get(k as usize).unwrap_or(&P(Value::Null)).clone());
                        }
                    }
                }
                LdIndex(idx) => {
                    let acc = self.pop().unwrap();
                    if val_is_array(&acc) {
                        let arr = val_array(&acc);
                        let arr = arr.borrow();
                        if idx as usize >= arr.len() {
                            self.push(P(Value::Null));
                        } else {
                            self.push(arr.get(idx as usize).unwrap_or(&P(Value::Null)).clone());
                        }
                    }
                }
                LdBuiltin(idx) => {
                    let builtin = self.builtins[idx as usize].clone();
                    self.push(builtin);
                }

                SetGlobal(at) => {
                    let acc = self.pop().unwrap();
                    let module = m.borrow_mut();
                    module.globals[at as usize] = acc;
                }
                SetEnv(at) => {
                    let acc = self.pop().unwrap();
                    let env = val_array(&self.env);
                    let env = env.borrow_mut();
                    if at >= env.len() as u32 {
                        panic!("Writing outside env");
                    }
                    env[at as usize] = acc;
                }
                SetLocal(idx) => {
                    let acc = self.pop().expect("SetLocal: stack empty");
                    self.locals.insert(idx, acc);
                }
                SetField(hash) => {
                    let acc = self.pop().unwrap();
                    let val = self.pop().expect("<SetField> Stack empty");
                    if val_is_obj(&val) {
                        let obj = val_object(&val);
                        let obj = obj.borrow_mut();
                        obj.insert(hash as i64, acc);
                    }
                }
                SetArray => {
                    let v1 = self.pop().expect("<SetArray> Stack empty");
                    let v2 = self.pop().expect("<SetArray> Stack empty");
                    let acc = self.pop().unwrap();
                    if val_is_array(&v1) && (val_is_int(&v2) || val_is_int32(&v2)) {
                        let array = val_array(&v1);
                        let array = array.borrow_mut();
                        let k = val_int(&v2) as usize;
                        if k < array.len() {
                            array[k] = acc;
                        }
                    }
                }
                SetIndex(i) => {
                    let acc = self.pop().unwrap();
                    let val = self.pop().expect("<SetIndex> Stack empty");
                    if val_is_array(&val) {
                        let arr = val_array(&val);
                        let arr = arr.borrow_mut();
                        arr[i as usize] = acc;
                    }
                }
                SetThis => {
                    let acc = self.pop().unwrap();
                    self.vthis = acc;
                }

                Pop(count) => {
                    pop_macro!(self, count);
                }
                MakeEnv(mut count) => {
                    let acc = self.pop().unwrap();
                    let mut tmp = vec![];
                    while count > 0 {
                        tmp.push(self.pop().expect("<Stack empty> Make env"));
                        count -= 1;
                    }

                    if !val_is_func(&acc) {
                        panic!("Invalid environment");
                    }
                    let func = val_func(&acc);
                    let func_m: &mut Function = func.borrow_mut();
                    func_m.env = P(Value::Array(P(tmp)));
                    self.push(acc);
                }
                MakeArray(mut count) => {
                    let mut tmp = vec![];
                    while count > 0 {
                        tmp.push(self.pop().expect("<Stack empty> Make env"));
                        count -= 1;
                    }
                    self.push(P(Value::Array(P(tmp))));
                }
                Call(argc) => {
                    let vthis = self.vthis.clone();
                    let acc = self.pop().unwrap();

                    do_call!(acc, self, m, vthis, argc);
                }
                ObjCall(argc) => {
                    let vtmp = self.pop().expect("Stack empty");
                    let acc = self.pop().unwrap();

                    do_call!(vtmp, self, m, acc, argc);
                }
                TailCall(_) => unimplemented!(),

                Ret => {
                    let val = self.pop().unwrap_or(P(Value::Null));
                    //self.stack.clear();
                    pop_infos!(true, m, self);

                    self.push(val);
                }
                Jump(to) => {
                    self.pc = (to) as usize;
                }
                JumpIf(to) => {
                    let acc = self.pop().unwrap();
                    if let Value::Bool(true) = acc.borrow() {
                        self.pc = (to - 1) as usize;
                    }
                }
                JumpIfNot(to) => {
                    let acc = self.pop().unwrap();
                    if let Value::Bool(false) = acc.borrow() {
                        self.pc = (to) as usize;
                    }
                }
                Add => {
                    let acc = self.pop().expect("Stack empty");
                    let val = self.pop().expect("Stack empty");
                    let acc_c = acc.clone();
                    let val_c = val.clone();
                    match (acc.borrow(), val.borrow()) {
                        (Value::Int(i), Value::Int(i2)) => self.push(P(Value::Int(i + i2))),
                        (Value::Float(f), Value::Float(f2)) => self.push(P(Value::Float(f + f2))),
                        (Value::Int(i), Value::Float(f2)) => {
                            self.push(P(Value::Float(*i as f64 + *f2)))
                        }
                        (Value::Float(f1), Value::Int(i)) => {
                            self.push(P(Value::Float(*f1 + *i as f64)))
                        }
                        (Value::Str(s), Value::Str(s2)) => {
                            self.push(P(Value::Str(format!("{}{}", s, s2))))
                        }
                        (Value::Object(_), _) => {
                            let _val = object_op!(self, acc_c, val_c, unsafe { FIELD_ADD }, m);
                            //self.push(val);
                        }
                        (_, Value::Object(_)) => {
                            let _val = object_op!(self, val_c, acc_c, unsafe { FIELD_ADD }, m);
                            //self.push(val);
                        }
                        _ => unimplemented!(),
                    };
                }
                Sub => op_!(-,self,m,FIELD_SUB),
                Mul => op_!(*,self,m,FIELD_MUL),
                Div => op_!(/,self,m,FIELD_DIV),
                Gt => cmp!(>,self,m,FIELD_GT),
                Lt => cmp!(<,self,m,FIELD_LT),
                Lte => cmp!(<=,self,m,FIELD_LTE),
                Gte => cmp!(>=,self,m,FIELD_GTE),
                Eq => cmp!(==,self,m,FIELD_EQ),
                Neq => cmp!(!=,self,m,FIELD_NEQ),
                Not => {
                    let acc = self.pop().expect("Stack empty");
                    if val_is_any_int(&acc) {
                        let i = val_int(&acc);
                        self.push(P(Value::Int(!i)));
                    } else if val_is_bool(&acc) {
                        let b = val_bool(&acc);
                        self.push(P(Value::Bool(!b)));
                    }
                }
                Xor => {
                    let v1 = self.pop().unwrap();
                    let v2 = self.pop().unwrap();
                    let v1 = val_int(&v1);
                    let v2 = val_int(&v2);
                    self.push(P(Value::Int(v1 ^ v2)));
                }
                Or => {
                    let v1 = self.pop().unwrap();
                    let v2 = self.pop().unwrap();
                    let v1 = val_int(&v1);
                    let v2 = val_int(&v2);
                    self.push(P(Value::Int(v1 | v2)));
                }
                And => {
                    let v1 = self.pop().unwrap();
                    let v2 = self.pop().unwrap();
                    let v1 = val_int(&v1);
                    let v2 = val_int(&v2);
                    self.push(P(Value::Int(v1 | v2)));
                }
                Shr => {
                    let v1 = self.pop().unwrap();
                    let v2 = self.pop().unwrap();
                    let v1 = val_int(&v1);
                    let v2 = val_int(&v2);
                    self.push(P(Value::Int(v1 >> v2)));
                }

                Shl => {
                    let v1 = self.pop().unwrap();
                    let v2 = self.pop().unwrap();
                    let v1 = val_int(&v1);
                    let v2 = val_int(&v2);
                    self.push(P(Value::Int(v1 << v2)));
                }

                New => {
                    let val = self.pop().expect("stack empty");
                    let proto = if val_is_null(&val) {
                        vec![]
                    } else if val_is_obj(&val) {
                        let obj = val_object(&val);
                        let obj: &Object = obj.borrow();
                        let mut entries = vec![];
                        for entry in obj.entries.iter() {
                            let entry = entry.borrow();
                            entries.push(P(entry.clone()));
                        }
                        entries
                    } else {
                        panic!("Object expected")
                    };
                    let obj = Object { entries: proto };
                    self.push(P(Value::Object(P(obj))));
                }
                Hash => {
                    use crate::hash::hash_val;
                    let val = self.pop();
                    if val.is_some() {
                        let mut h = 0xcbf29ce484222325;
                        hash_val(&mut h, &val.unwrap());
                        self.push(P(Value::Int(h as i64)));
                    } else {
                        self.push(P(Value::Int(0)));
                    }
                }
                IsNull => {
                    let val = self.pop().unwrap();

                    self.push(P(Value::Bool(val_is_null(&val))));
                }
                IsNotNull => {
                    let val = self.pop().unwrap();
                    self.push(P(Value::Bool(!val_is_null(&val))));
                }
                TypeOf => {
                    let val = self.pop().unwrap();
                    let ty = match val.borrow() {
                        Value::Int(_) => "Int",
                        Value::Int32(_) => "Int32",
                        Value::Float(_) => "Float",
                        Value::Array(_) => "Array",
                        Value::Null => "Null",
                        Value::Object(_) => "Object",
                        Value::Str(_) => "String",
                        Value::Func(_) => "Function",
                        _ => unimplemented!(),
                    };
                    self.push(P(Value::Str(ty.to_owned())));
                }
                _ => unimplemented!(),
            }
        }

        return self.pop().unwrap_or(P(Value::Null));
    }
}
