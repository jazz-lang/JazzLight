use crate::*;
use jazzlight::opcode::*;
use jazzlight::value::*;
use jazzlight::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub enum UOP {
    Goto(String),
    GotoF(String),
    GotoT(String),
    Label(String),
    PAddr(String),
    Op(Op),
}

#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub enum Global {
    Var(String),
    Func(i32, i32),
    Str(String),
    Float(u64),
}
#[derive(Clone, Debug, PartialEq)]
pub enum Access {
    Env(i32),
    Stack(i32),
    Global(i32),
    Field(P<Expr>, String),
    Index(i32),
    Array(P<Expr>, P<Expr>),
    This,
}

#[derive(Clone)]
pub struct Globals {
    pub globals: LinkedHashMap<Global, i32>,
    pub objects: LinkedHashMap<String, Vec<i32>>,
    pub functions: Vec<(Vec<UOP>, Vec<(i32, i32)>, i32, i32)>,
    pub table: Vec<Global>,
}

use crate::ast::*;
use crate::token::Position;
use hashlink::*;
use std::collections::HashMap;

pub struct Context {
    pub g: Rc<RefCell<Globals>>,
    pub ops: Vec<UOP>,
    pub pos: Vec<(i32, i32)>,
    pub locals: LinkedHashMap<String, i32>,
    pub env: LinkedHashMap<String, i32>,
    pub stack: i32,
    pub limit: i32,
    pub nenv: i32,
    pub breaks: Vec<String>,
    pub continues: Vec<String>,
    pub cur_pos: Option<Position>,
    pub labels: LinkedHashMap<String, Option<usize>>,
    pub used_upvars: LinkedHashMap<String, i32>,
    pub trace_info: HashMap<u32, (usize, String)>,
    pub ret_lbl: String,
}
impl Context {
    pub fn new_named_label(&mut self) {}
    pub fn finish(&mut self) -> Vec<Op> {
        for (idx, op) in self.ops.iter().enumerate() {
            match op {
                UOP::Label(l) => {
                    let pos = idx;
                    self.labels.insert(l.to_owned(), Some(pos));
                }
                _ => (),
            }
        }
        self.ops
            .iter()
            .map(|i| match *i {
                UOP::Op(ref op) => op.clone(),
                UOP::PAddr(ref lbl) => Op::CatchPush(self.labels.get(lbl).unwrap().unwrap() as _),
                UOP::Goto(ref lbl) => Op::Jump(self.labels.get(lbl).unwrap().unwrap() as u32),
                UOP::GotoF(ref lbl) => Op::JumpIfNot(self.labels.get(lbl).unwrap().unwrap() as u32),
                UOP::GotoT(ref lbl) => Op::JumpIf(self.labels.get(lbl).unwrap().unwrap() as u32),
                _ => Op::Nop,
            })
            .collect::<Vec<Op>>()
    }

    pub fn pos(&self) -> usize {
        self.ops.len()
    }

    pub fn write(&mut self, op: Op) {
        self.ops.push(UOP::Op(op));
    }
    pub fn emit_paddr(&mut self, t: &str) {
        self.ops.push(UOP::PAddr(t.to_owned()));
    }
    pub fn emit_goto(&mut self, to: &str) {
        self.ops.push(UOP::Goto(to.to_owned()));
    }
    pub fn emit_gotof(&mut self, to: &str) {
        self.ops.push(UOP::GotoF(to.to_owned()));
    }

    pub fn emit_gotot(&mut self, to: &str) {
        self.ops.push(UOP::GotoT(to.to_owned()));
    }

    pub fn new_empty_label(&mut self) -> String {
        let lab_name = self.labels.len().to_string();
        self.labels.insert(lab_name.clone(), None);
        lab_name
    }

    pub fn label_here(&mut self, label: &str) {
        self.ops.push(UOP::Label(label.to_owned()));
        //*self.labels.get_mut(label).unwrap() = Some(self.ops.len());
    }

    pub fn goto(&mut self, p: u32) {
        self.write(Op::Jump(p));
    }
    pub fn global(&mut self, g: &Global) -> i32 {
        let g1 = self.g.borrow().globals.get(g).cloned();
        return match g1 {
            Some(g) => g.clone(),
            None => {
                let mut g_ = self.g.borrow_mut();
                let gid = g_.table.len() as i32;
                g_.globals.insert(g.clone(), gid);
                g_.table.push(g.clone());
                drop(g_);
                gid
            }
        };
    }
    pub fn compile_const(&mut self, c: &Constant) {
        match c {
            Constant::True => self.write(Op::LoadTrue),
            Constant::False => self.write(Op::LoadFalse),
            Constant::Null => self.write(Op::LoadNull),
            Constant::This => self.write(Op::LoadThis),
            Constant::Int(n) => self.write(Op::LoadInt(n.clone())),
            Constant::Float(f) => {
                let pos = self.global(&Global::Float(f.to_bits()));
                self.write(Op::LoadGlobal(pos as _));
            }
            Constant::Str(s) => {
                let pos = self.global(&Global::Str(s.to_owned()));
                self.write(Op::LoadGlobal(pos as _));
            }
            Constant::Ident(s) => {
                let s: &str = s;
                if self.locals.contains_key(s) {
                    let i = *self.locals.get(s).unwrap();
                    self.write(Op::LoadLocal(i as u16));
                } else if self.env.contains_key(s) {
                    self.nenv += 1;
                    let pos = if !self.used_upvars.contains_key(s) {
                        let pos = self.used_upvars.len();

                        self.used_upvars.insert(s.to_owned(), pos as _);
                        pos as u16
                    } else {
                        *self.used_upvars.get(s).unwrap() as u16
                    };
                    self.write(Op::LoadEnv(pos));
                } else {
                    let g = self.global(&Global::Var(s.to_owned()));
                    self.write(Op::LoadGlobal(g as u32));
                }
            }
            Constant::Builtin(name) => {
                let _ = self.global(&Global::Str(name.to_owned()));
                self.write(Op::LoadBuiltin(name.to_owned()));
            }
        }
    }
    pub fn write_op(&mut self, op: &str) {
        use Op::*;
        match op {
            "&&" => self.write(And),
            "||" => self.write(Or),
            "+" => self.write(Add),
            "-" => self.write(Sub),
            "/" => self.write(Div),
            "*" => self.write(Mul),
            "%" => self.write(Mod),
            "<<" => self.write(Shl),
            ">>" => self.write(Shr),
            "|" => self.write(Or),
            "&" => self.write(And),
            "^" => self.write(Xor),
            "==" => self.write(Eq),
            "!=" => self.write(Neq),
            ">" => self.write(Gt),
            ">=" => self.write(Gte),
            "<" => self.write(Lt),
            "<=" => self.write(Lte),
            "!" => self.write(Not),
            _ => panic!("Unknown operation {}", op),
        }
    }

    pub fn compile_access(&mut self, e: &P<Expr>) -> Access {
        match &e.decl {
            ExprDecl::Const(Constant::Ident(name)) => {
                let l = self.locals.get(name);
                let s: &str = name;
                if l.is_some() {
                    let l = *l.unwrap();
                    return Access::Stack(l);
                } else if self.env.contains_key(s) {
                    let l = self.env.get(s);
                    self.used_upvars.insert(s.to_owned(), *l.unwrap());
                    self.nenv += 1;
                    return Access::Env(*l.unwrap());
                } else {
                    let g = self.global(&Global::Var(name.to_owned()));
                    return Access::Global(g);
                }
            }
            ExprDecl::Field(e, f) => {
                //self.compile(e);
                return Access::Field(e.clone(), f.to_owned());
            }
            ExprDecl::Const(Constant::This) => return Access::This,
            ExprDecl::Array(ea, ei) => {
                /*self.compile(ea);
                self.compile(ei);*/
                return Access::Array(ea.clone(), ei.clone());
            }
            _ => unimplemented!(),
        }
    }

    pub fn access_get(&mut self, acc: Access) {
        match acc {
            Access::Env(i) => self.write(Op::LoadEnv(i as _)),
            Access::Stack(i) => self.write(Op::LoadLocal(i as _)),
            Access::Global(g) => self.write(Op::LoadGlobal(g as _)),
            Access::Field(e, f) => {
                let gid = self.global(&Global::Str(f));
                self.write(Op::LoadGlobal(gid as _));
                self.compile(&e, false);
                self.write(Op::Load)
            }
            Access::Index(_) => unimplemented!(),
            Access::This => self.write(Op::LoadThis),
            Access::Array(ea, ei) => {
                self.compile(&ei, false);
                self.compile(&ea, false);
                self.write(Op::Load);
            }
        }
    }

    pub fn access_set(&mut self, acc: Access) {
        match acc {
            Access::Env(n) => self.write(Op::StoreEnv(n as _)),
            Access::Stack(l) => self.write(Op::StoreLocal(l as _)),
            Access::Global(_) =>
            /*self.write(Op::StoreGlobal(g as u32)),*/
            {
                unimplemented!()
            }
            Access::Field(e, f) => {
                let gid = self.global(&Global::Str(f.to_owned()));
                self.write(Op::LoadGlobal(gid as _));
                self.compile(&e, false);
                self.write(Op::Store);
            }
            Access::Index(_) => unimplemented!(),
            Access::This => self.write(Op::StoreThis),
            Access::Array(ea, ei) => {
                self.compile(&ei, false);
                self.compile(&ea, false);
                self.write(Op::Store);
            }
        }
    }
    pub fn compile(&mut self, e: &P<Expr>, tail: bool) {
        match &e.decl {
            ExprDecl::Break(e) => {
                if e.is_some() {
                    let e = e.clone().unwrap();
                    self.compile(&e, false);
                } else {
                    self.write(Op::LoadNull);
                }
                let br = self.breaks.last().expect("break in wrong context").clone();
                self.emit_goto(&br);
            }
            ExprDecl::Continue => {
                let c = self
                    .continues
                    .last()
                    .expect("continue in wrong context")
                    .clone();
                self.emit_goto(&c);
            }
            ExprDecl::Const(c) => self.compile_const(c),
            ExprDecl::Block(v) => {
                if v.len() == 0 {
                    self.write(Op::LoadNull);
                } else {
                    let locals = self.locals.clone();
                    //let stack = self.stack;
                    for el in v.iter() {
                        self.compile(el, tail);
                    }

                    /*if stack < self.stack {
                        self.write(Op::Pop((self.stack - stack) as u32)); // clear stack from values and locals
                    }*/
                    self.locals = locals;
                }
            }
            ExprDecl::Paren(e) => self.compile(e, tail),
            ExprDecl::Field(e, f) => {
                /*let mut h = 0xcbf29ce484222325;
                hash_bytes(&mut h, f.as_bytes());
                self.write(Op::LoadField(h));*/
                let gid = self.global(&Global::Str(f.to_owned()));
                self.write(Op::LoadGlobal(gid as _));
                self.compile(e, false);
                self.write(Op::Load);
            }
            ExprDecl::Array(ea, ei) => {
                self.compile(ei, false);
                self.compile(ea, false);
                self.write(Op::Load);
            }
            ExprDecl::Var(_, name, init) => {
                match init {
                    Some(e) => match &e.decl {
                        ExprDecl::Function(args, body) => {
                            self.compile_function(args, body, Some(name))
                        }
                        _ => self.compile(e, false),
                    },
                    None => self.write(Op::LoadNull),
                }
                let id = self.locals.len() as u16;
                self.locals.insert(name.to_owned(), id as i32);

                self.write(Op::StoreLocal(id));
            }

            ExprDecl::Assign(e1, e2) => {
                let a = self.compile_access(e1);
                self.compile(e2, false);
                self.access_set(a);
            }
            ExprDecl::Binop(op, e1, e2) => {
                self.compile_binop(op, e1, e2, tail);
            }
            ExprDecl::Function(params, e) => {
                self.compile_function(params, e, None);
            }
            ExprDecl::Return(e) => {
                match e {
                    Some(e) => self.compile(e, false),
                    None => self.write(Op::LoadNull),
                }

                //let _ = self.ret_lbl.clone();
                self.write(Op::Ret);
                //self.stack = stack;
            }
            ExprDecl::While(cond, body) => {
                let start = self.new_empty_label();
                let end = self.new_empty_label();
                self.breaks.push(end.clone());
                self.continues.push(start.clone());
                self.label_here(&start);
                self.compile(cond, false);
                self.emit_gotof(&end);
                self.compile(body, false);
                self.emit_goto(&start);
                self.label_here(&end);
                self.breaks.pop();
                self.continues.pop();
            }
            ExprDecl::Switch(value, with, default_) => {
                let orl = self.new_empty_label();
                let end = self.new_empty_label();

                for (cond, expr) in with.iter() {
                    let l1 = self.new_empty_label();
                    self.compile(value, false);
                    self.compile(cond, false);
                    self.write(Op::Eq);
                    self.emit_gotof(&l1);
                    self.compile(&expr, tail);
                    self.emit_goto(&end);
                    self.label_here(&l1);
                }
                if default_.is_some() {
                    self.emit_goto(&orl);
                }
                self.label_here(&orl);
                if default_.is_some() {
                    self.compile(&default_.clone().unwrap(), false);
                    self.emit_goto(&end);
                }
                self.label_here(&end);
            }

            ExprDecl::If(e, e1, e2) => {
                //let stack = self.stack;

                let lbl_false = self.new_empty_label();
                self.compile(&e, false);
                self.emit_gotof(&lbl_false);
                self.compile(e1, tail);
                self.label_here(&lbl_false);
                if e2.is_some() {
                    let e2 = e2.clone().unwrap();
                    self.compile(&e2, tail);
                }
            }
            ExprDecl::Call(e, el) => {
                match &e.decl {
                    ExprDecl::Const(Constant::Builtin(name)) => {
                        let builtin: &str = name;
                        match builtin {
                            "new" => {
                                self.compile(&el[0], false);
                                self.write(Op::New);
                                return;
                            }
                            "hash" => {
                                self.compile(&el[0], false);
                                self.write(Op::Hash);
                                return;
                            }
                            /*"typeof" => {
                                self.compile(&el[0]);
                                self.write(Op::TypeOf);
                                return;
                            }*/
                            _ => (),
                        }
                    }
                    ExprDecl::Field(e, f) => {
                        for e in el.iter().rev() {
                            self.compile(e, false);
                        }
                        self.compile(e, false);
                        let gid = self.global(&Global::Str(f.to_owned()));
                        self.write(Op::LoadGlobal(gid as _));
                        self.compile(e, false);
                        self.write(Op::Load);
                        self.write(Op::ObjCall(el.len() as u16));
                        return;
                    }
                    _ => (),
                }
                for x in el.iter().rev() {
                    self.compile(x, false);
                }
                self.compile(e, false);
                if !tail {
                    self.write(Op::Call(el.len() as _));
                } else {
                    self.write(Op::TailCall(el.len() as _));
                }
            }
            ExprDecl::Label(label) => {
                self.labels.insert(label.to_owned(), Some(self.pos()));
            }
            ExprDecl::Goto(label) => {
                self.emit_goto(label);
            }
            ExprDecl::Unop(op, e) => {
                self.compile(e, tail);
                let op: &str = op;
                match op {
                    "-" => self.write(Op::Neg),
                    "!" => self.write(Op::Not),
                    _ => (),
                }
            }
            ExprDecl::Throw(expr) => {
                self.compile(expr, false);
                self.write(Op::Throw);
            }
            ExprDecl::Try(expr, name, catch) => {
                let catch_lbl = self.new_empty_label();
                let end_lbl = self.new_empty_label();
                self.emit_paddr(&catch_lbl);
                self.compile(expr, false);
                self.emit_goto(&end_lbl);
                self.label_here(&catch_lbl);
                let locals = self.locals.clone();
                let id = self.locals.len() as _;
                self.locals.insert(name.to_owned(), id);
                self.write(Op::StoreLocal(id as _));
                self.compile(catch, tail);
                self.locals = locals;
                self.label_here(&end_lbl);
            }
            v => panic!("{:?}", v),
        }
    }

    pub fn compile_binop(&mut self, op: &str, e1: &P<Expr>, e2: &P<Expr>, tail: bool) {
        match op {
            "==" => match &e2.decl {
                ExprDecl::Const(Constant::Null) => {
                    self.compile(e1, false);
                    self.write(Op::IsNull);
                }
                _ => {
                    self.compile(e2, false);
                    self.compile(e1, false);
                    self.write(Op::Eq);
                }
            },
            "!=" => match &e2.decl {
                ExprDecl::Const(Constant::Null) => {
                    self.compile(e1, false);
                    self.write(Op::IsNotNull);
                }
                _ => {
                    self.compile(e2, false);
                    self.compile(e1, false);
                    self.write(Op::Neq);
                }
            },
            "&&" => {
                let if_false = self.new_empty_label();
                self.compile(e1, false);
                self.emit_gotof(&if_false);
                self.compile(e2, tail);
                self.label_here(&if_false);
            }
            "||" => {
                let if_true = self.new_empty_label();
                self.compile(e1, false);
                self.emit_gotot(&if_true);
                self.compile(e2, tail);

                self.label_here(&if_true);
            }
            _ => {
                self.compile(e2, false);

                self.compile(e1, false);
                self.write_op(op);
            }
        }
    }

    pub fn compile_function(&mut self, params: &[String], e: &P<Expr>, vname: Option<&str>) {
        let mut ctx = Context {
            g: self.g.clone(),
            ops: Vec::new(),
            pos: Vec::new(),
            limit: self.stack,
            stack: self.stack,
            locals: LinkedHashMap::new(),
            nenv: 0,
            env: self.locals.clone(),
            cur_pos: None,
            continues: vec![],
            breaks: vec![],
            labels: self.labels.clone(),
            used_upvars: LinkedHashMap::new(),
            trace_info: HashMap::new(),
            ret_lbl: String::new(),
        };
        for (idx, p) in params.iter().enumerate() {
            ctx.stack += 1;
            ctx.locals.insert(p.to_owned(), idx as i32);
        }

        let gid = ctx.g.borrow().table.len();
        if vname.is_some() {
            ctx.g
                .borrow_mut()
                .globals
                .insert(Global::Var(vname.unwrap().to_owned()), gid as i32);
        }
        ctx.g.borrow_mut().table.push(Global::Func(gid as i32, -1));
        ctx.ret_lbl = ctx.new_empty_label();
        ctx.compile(e, true);
        let ret_lbl = ctx.ret_lbl.clone();
        ctx.label_here(&ret_lbl);
        ctx.write(Op::Ret);
        //ctx.check_stack(s, "");

        ctx.g.borrow_mut().functions.push((
            ctx.ops.clone(),
            ctx.pos.clone(),
            gid as i32,
            params.len() as i32,
        ));

        for (k, v) in ctx.labels.iter() {
            self.labels.insert(k.clone(), v.clone());
        }
        if ctx.nenv > 0 {
            for (var, _) in ctx.used_upvars.iter().rev() {
                self.compile_const(&Constant::Ident(var.to_owned()));
            }
            self.write(Op::LoadGlobal(gid as _));

            self.write(Op::MakeEnv((ctx.used_upvars.len()) as u16));
        } else {
            self.write(Op::LoadGlobal(gid as _));
        }
    }

    pub fn new() -> Context {
        let g = Globals {
            globals: LinkedHashMap::new(),
            objects: LinkedHashMap::new(),
            functions: vec![],
            table: vec![],
        };
        Context {
            g: Rc::new(RefCell::new(g)),
            ops: vec![],
            pos: vec![],
            locals: Default::default(),
            env: Default::default(),
            stack: 0,
            limit: 0,
            nenv: 0,
            breaks: vec![],
            continues: vec![],
            cur_pos: None,
            labels: Default::default(),
            used_upvars: Default::default(),
            trace_info: HashMap::new(),
            ret_lbl: String::new(),
        }
    }
}

pub fn compile(ast: Vec<P<Expr>>) -> Context {
    let mut ctx = Context::new();
    let ast = P(Expr {
        pos: Position::new(
            ast.get(0)
                .map(|x| x.pos.file.clone())
                .unwrap_or(Arc::from("<>".to_owned())),
            0,
            0,
        ),
        decl: ExprDecl::Block(ast.clone()),
    });

    ctx.ret_lbl = ctx.new_empty_label();
    ctx.compile(&ast, false);
    let ret_lbl = ctx.ret_lbl.clone();
    ctx.label_here(&ret_lbl);
    ctx.write(Op::Ret);

    if ctx.g.borrow().functions.len() != 0 || ctx.g.borrow().objects.len() != 0 {
        let ctxops = ctx.ops.clone();
        let _ctxpos = ctx.pos.clone();
        let ops = vec![];
        let pos = vec![];
        ctx.ops = ops;
        ctx.pos = pos;
        ctx.write(Op::Jump(0));
        let functions = ctx.g.borrow().functions.clone();
        for (fops, fpos, gid, nargs) in functions.iter().rev() {
            let mut g = ctx.g.borrow_mut();

            g.table[*gid as usize] = Global::Func(ctx.ops.len() as i32, *nargs);

            for op in fops.iter() {
                ctx.ops.push(op.clone());
            }
            ctx.ops[0] = UOP::Op(Op::Jump(ctx.ops.len() as u32));
            for op in fpos.iter() {
                ctx.pos.push(*op);
            }
        }
        for op in ctxops.iter() {
            ctx.ops.push(op.clone());
        }
    }

    ctx
}

/// Construct new VM Module from compilation context.
pub fn module_from_context(ctx: &mut Context) -> Ref<Module> {
    let m = Ref(Module {
        exports: Value::Object(Ref(Object {
            prototype: None,
            table: Default::default(),
        })),
        code: vec![],

        globals: vec![Value::Null; ctx.g.borrow().table.len()],
        trace_info: Default::default(),
    });

    for (i, g) in ctx.g.borrow().table.iter().enumerate() {
        match g {
            Global::Func(off, nargs) => {
                let func = Ref(Function {
                    native: false,
                    address: *off as _,
                    argc: *nargs,
                    env: Value::Array(Ref(vec![])),
                    module: Some(m.clone()),
                });

                m.borrow_mut().globals[i] = Value::Function(func);
            }
            Global::Str(s) => {
                m.borrow_mut().globals[i] = Value::String(Ref(s.to_owned()));
            }
            Global::Float(x) => {
                m.borrow_mut().globals[i] = Value::Float(f64::from_bits(*x));
            }
            _ => (),
        };
    }
    m.borrow_mut().code = ctx.finish();

    m
}
