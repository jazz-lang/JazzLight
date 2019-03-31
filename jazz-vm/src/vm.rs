use crate::module::*;
use crate::opcode::Opcode;
use crate::value::*;
use crate::P;

pub enum CSPVal {
    Pc(usize),
    Module(P<Module>),
    Val(P<Value>),
    Locals(fnv::FnvHashMap<u32, P<Value>>),
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
    };
}

macro_rules! pop_infos {
    ($restpc: expr,$m: expr,$vm: expr) => {
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
                    $vm.push(v);
                }
            }
        } else {
            panic!("Invalid call");
        }
    };
}

macro_rules! object_op_gen {
    ($acc: expr,$vm: expr,$obj: expr,$param: expr,$id: expr,$err: expr,$m: expr) => {
        let o = $obj;
        let ob = val_object(&o);
        let obj = ob.borrow();
        let arg = $param;
        let f = obj.find($id);
        if f.is_none() {
            $err;
        } else {
            push_infos!($vm, $m);
            $vm.push(callex(o, f.unwrap().clone(), vec![arg]));
            pop_infos!(false, $m, $vm);
        }
    };
}

macro_rules! object_op {
    ($acc: expr,$vm: expr,$obj: expr,$param: expr,$id: expr,$m: expr) => {
        object_op_gen!(
            $acc,
            $vm,
            $obj,
            $param,
            $id,
            panic!("Unsupported operation"),
            $m
        );
    };
}

macro_rules! op_ {
    ($op: tt,$vm: expr,$m: expr,$id: expr) => {
        {
            let acc = $vm.pop().expect("Stack empty");
        let val = $vm.pop().expect("Stack empty");
        if val_is_any_int(&acc) && val_is_any_int(&val) {
            $vm.push(P(Value::Int(val_int(&val) $op val_int(&acc))));
        } else if val_is_int(&acc) {
            if val_is_float(&val) {
                $vm.push(P(Value::Float(val_float(&val) $op val_int(&acc) as f64)));
            } else if val_is_int32(&val) {
                $vm.push(P(Value::Int32(val_int32(&val) $op val_int32(&acc))));
            } else if val_is_str(&acc) {
                unimplemented!()
            } else if val_is_obj(&acc) {
                let acc2 = acc.clone();
                object_op!(acc, $vm, acc2, val, unsafe { $id }, $m);
            } else {
                panic!("Invalid operation `{}`",stringify!($op));
            }
        } else if val_is_any_int(&val) {
            if val_is_float(&acc) {
                $vm.push(P(Value::Float(val_int(&val) as f64 $op val_float(&acc))));
            }
        } else {
            if val_is_obj(&val) {
                let v2 = val.clone();
                object_op!(acc, $vm, v2, acc, unsafe { $id }, $m);
            } else if val_is_str(&val) && val_is_str(&acc) {
                unimplemented!()
            }
        }
        }
    };
    (cmp $op: tt,$vm: expr,$m: expr,$id: expr) => {
        {
            let acc = $vm.pop().expect("Stack empty");
        let val = $vm.pop().expect("Stack empty");

        if val_is_any_int(&acc) && val_is_any_int(&val) {
            $acc = P(Value::Bool(val_int(&val) $op (val_int(&acc))));
        } else if val_is_int(&acc) {
            if val_is_float(&val) {
                $acc = P(Value::Bool(val_float(&val) $op (val_int(&acc) as f64)));
            } else if val_is_int32(&val) {
                $acc = P(Value::Bool(val_int32(&val) $op val_int32(&acc)));
            } else if val_is_str(&acc) {
                unimplemented!()
            } else if val_is_obj(&acc) {
                let acc2 = $acc.clone();
                object_op!($acc, $vm, acc2, val, unsafe { $id }, $m);
            } else {
                panic!("Invalid operation `+`");
            }
        } else if val_is_any_int(&val) {
            if val_is_float(&acc) {
                $acc = P(Value::Bool((val_int(&val) as f64) $op val_float(&acc)));
            }
        } else {
            if val_is_obj(&val) {
                let v2 = val.clone();
                object_op!($acc, $vm, v2, $acc, unsafe { $id }, $m);
            } else if val_is_str(&val) && val_is_str(&acc) {
                unimplemented!()
            }
        }
        }
    };
}

macro_rules! cmp {
    ($op: tt,$vm: expr,$m: expr,$id: expr) => {
        {


        let v2 = $vm.pop().expect("Stack empty");
        let v1 = $vm.pop().expect("Stack empty");
        let v_clon = v1.clone();
        let v = v1.borrow();
        let acc = v2.borrow();
        let val = match (v,acc) {
            (Value::Int(i),Value::Int(i2)) => Value::Bool(i $op i2),
            (Value::Int32(i),Value::Int32(i2)) => Value::Bool(i $op i2),
            (Value::Int(i),Value::Int32(i2)) => Value::Bool(*i $op *i2 as i64),
            (Value::Int32(i),Value::Int(i2)) => Value::Bool((*i as i64) $op *i2),
            (Value::Int(i),Value::Float(f)) => Value::Bool((*i as f64) $op *f),
            (Value::Int32(i),Value::Float(f)) => Value::Bool((*i as f64) $op *f),
            (Value::Float(f), Value::Int(i)) => Value::Bool(*f $op *i as f64),
            (Value::Float(f),Value::Int32(i)) => Value::Bool(*f $op *i as f64),
            (Value::Float(f),Value::Float(f2)) => Value::Bool(*f $op *f2),
            (Value::Str(s1),Value::Str(s2)) => Value::Bool(*s1 $op *s2),
            (Value::Array(a1),Value::Array(a2)) => {
                let a1 = a1.borrow();
                let a2 = a2.borrow();
                Value::Bool(a1.len() $op a2.len())
            }
            (Value::Bool(b),Value::Bool(b1)) => Value::Bool((*b as u8) $op *b1 as u8),
            (Value::Object(obj1),Value::Object(_)) => {
                let obj = obj1.borrow();
                let tmp = obj.find(unsafe {$id});
                if tmp.is_none() {
                    panic!("Invalid comparison");
                }
                callex(v1, tmp.unwrap().clone(), vec![v_clon]).borrow().clone()


            },
            _ => unimplemented!()
        };
        $vm.push(P(val));
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
                        vm.stack[n] = args[n].clone();
                    }
                    vm.pc = *off as usize;
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
            //println!("current: {:04} {:?}", self.pc - 1, op);
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
                    let module: &mut Module = m.borrow_mut();
                    self.push(module.globals[idx as usize].clone());
                }
                LdEnv(at) => {
                    let env = val_array(&self.env);
                    let env = env.borrow_mut();
                    if at >= env.len() as u32 {
                        panic!("Reading outside env");
                    }
                    self.push(env[at as usize].clone());
                }
                LdField(field) => {
                    let acc = self.pop().unwrap();
                    if val_is_obj(&acc) {
                        let obj_p = val_object(&acc);
                        let obj: &Object = obj_p.borrow();
                        let f = obj.find(field as i64);
                        if f.is_some() {
                            self.push(f.unwrap().clone());
                        } else {
                            self.push(P(Value::Null));
                        }
                    } else {
                        panic!("Invalid field access");
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
                    do_call!(acc, self, m, vtmp, argc);
                }
                TailCall(_) => unimplemented!(),

                Ret => {
                    let val = self.pop().expect("stack empty");
                    pop_infos!(true, m, self);
                    self.stack.clear();
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

                    if val_is_any_int(&acc) && val_is_any_int(&val) {
                        self.push(P(Value::Int(val_int(&val) + val_int(&acc))));
                    } else if val_is_int(&acc) {
                        if val_is_float(&val) {
                            self.push(P(Value::Float(val_float(&val) + val_int(&acc) as f64)));
                        } else if val_is_int32(&val) {
                            self.push(P(Value::Int32(val_int32(&val) + val_int32(&acc))));
                        } else if val_is_str(&acc) {
                            unimplemented!()
                        } else if val_is_obj(&acc) {
                            let acc2 = acc.clone();
                            object_op!(acc, self, acc2, val, unsafe { FIELD_ADD }, m);
                        } else {
                            panic!("Invalid operation `+`");
                        }
                    } else if val_is_any_int(&val) {
                        if val_is_float(&acc) {
                            self.push(P(Value::Float(val_int(&val) as f64 + val_float(&acc))));
                        }
                    } else {
                        if val_is_obj(&val) {
                            let v2 = val.clone();
                            object_op!(acc, self, v2, acc, unsafe { FIELD_ADD }, m);
                        } else if val_is_str(&val) && val_is_str(&acc) {
                            unimplemented!()
                        }
                    }
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
                Opcode::New => {
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
                _ => unimplemented!(),
            }
        }

        return self.pop().unwrap_or(P(Value::Null));
    }
}
