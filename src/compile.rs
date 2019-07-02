use nabox::hash::hash_bytes;
use nabox::opcode::Opcode;
use nabox::Cell;

#[derive(Clone, Debug)]
pub enum UOP {
    Goto(String),
    GotoF(String),
    GotoT(String),
    Label(String),
    Op(Opcode),
}

#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub enum Global {
    Var(String),
    Func(i32, i32),
    Str(String),
    Float(String),
}
#[derive(Clone, Debug, PartialEq)]
pub enum Access {
    Env(i32),
    Stack(i32),
    Global(i32),
    Field(String),
    Index(i32),
    Array,
    This,
}



#[derive(Clone, Debug)]
pub struct Globals {
    pub globals: LinkedHashMap<Global, i32>,
    pub objects: LinkedHashMap<String, Vec<i32>>,
    pub functions: Vec<(Vec<UOP>, Vec<(i32, i32)>, i32, i32)>,
    pub table: Vec<Global>,
}

use linked_hash_map::LinkedHashMap;

pub struct Context {
    pub g: Cell<Globals>,
    pub ops: Vec<UOP>,
    pub locals: LinkedHashMap<String, i32>,
    pub env: LinkedHashMap<String, i32>,
    pub stack: i32,
    pub limit: i32,
    pub nenv: i32,
    pub breaks: Vec<String>,
    pub continues: Vec<String>,
    pub pos: Vec<(i32, i32)>,
    pub cur_pos: (i32, i32),
    pub cur_file: String,
    pub builtins: LinkedHashMap<String, i32>,
    pub labels: LinkedHashMap<String, Option<usize>>,
    pub fields: Cell<LinkedHashMap<u64, String>>,
    pub used_upvars: LinkedHashMap<String, i32>,
    pub optimize: bool,
}

use crate::ast::*;
use crate::token::Position;
use crate::P;

impl Context {
    pub fn check_stack(&self, stack: i32, p: &str) {
        if self.stack != stack {
            panic!("Stack alignment failure {}", p);
        }
    }
    pub fn pos(&self) -> usize {
        self.ops.len()
    }

    pub fn write(&mut self, op: Opcode) {
        self.pos.push(self.cur_pos);
        self.ops.push(UOP::Op(op));
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
        self.write(Opcode::Jump(p));
    }
    pub fn global(&mut self, g: &Global) -> i32 {
        return match self.g.globals.get(g).cloned() {
            Some(g) => g.clone(),
            None => {
                let gid = self.g.table.len() as i32;
                self.g.globals.insert(g.clone(), gid);
                self.g.table.push(g.clone());

                gid
            }
        };
    }

    /*pub fn scan_labels(&mut self, supported: bool, in_block: bool, expr: &P<Expr>) {
        match &expr.decl {
            ExprDecl::Function(args, body) => {
                let nargs = args.len();
                self.stack += nargs as i32;
                self.scan_labels(supported, false, body);
                self.stack -= nargs as i32;
            }
            ExprDecl::Block(exprs) => {
                let old = self.stack;
                for expr in exprs.iter() {
                    self.scan_labels(supported, true, expr);
                }
                self.stack = old;
            }
            ExprDecl::Var(_, _, init) => {
                if !in_block {
                    panic!("Variable declaration must be done in block")
                };
                match init {
                    Some(e) => self.scan_labels(supported, false, e),
                    _ => (),
                };
                self.stack += 1;
            }

            ExprDecl::Label(l) => {
                if !supported {
                    panic!("Label not supported in this part")
                }
            }
            ExprDecl::Assign(e1, e2) => {
                fn is_extended(e: &ExprDecl) -> bool {
                    match e {
                        ExprDecl::Paren(p) => is_extended(&p.decl),
                        ExprDecl::Array(_, _) | ExprDecl::Field(_, _) => true,
                        _ => false,
                    }
                }
                let ext = is_extended(&e1.decl);
                if ext {
                    self.stack += 1;
                }
                self.scan_labels(supported, false, e2);
                self.stack += 1;
                self.scan_labels(supported, false, e1);
                self.stack -= if ext { 2 } else { 1 };
            }
            ExprDecl::Call(e, el) => {
                for ex in el.iter() {
                    self.scan_labels(supported, false, ex);
                }
                self.scan_labels(supported, false, e);
                self.stack -= el.len() as i32;
            }
            ExprDecl::Binop(_, _, _) | ExprDecl::Field(_, _) | ExprDecl::Array(_, _) => {
                expr.iter(move |x| {
                    self.scan_labels(false, false, x);
                });
            }
            _ => {
                expr.iter(move |x| {
                    self.scan_labels(supported, false, x);
                });
            }
        }
    }*/

    pub fn compile_binop(&mut self, op: &str, e1: &P<Expr>, e2: &P<Expr>) {
        match op {
            "==" => match &e2.decl {
                ExprDecl::Const(Constant::Null) => {
                    self.compile(e1);
                    self.write(Opcode::IsNull);
                }
                _ => {
                    self.compile(e2);
                    self.compile(e1);
                    self.write(Opcode::Eq);
                }
            },
            "!=" => match &e2.decl {
                ExprDecl::Const(Constant::Null) => {
                    self.compile(e1);
                    self.write(Opcode::IsNotNull);
                }
                _ => {
                    self.compile(e2);
                    self.compile(e1);
                    self.write(Opcode::Neq);
                }
            },
            _ => {
                if self.optimize {
                    match (op, &e1.decl, &e2.decl) {
                        (
                            "+",
                            ExprDecl::Const(Constant::Int(i)),
                            ExprDecl::Const(Constant::Int(i2)),
                        ) => self.write(Opcode::LdInt(i + i2)),
                        (
                            "-",
                            ExprDecl::Const(Constant::Int(i)),
                            ExprDecl::Const(Constant::Int(i2)),
                        ) => self.write(Opcode::LdInt(i - i2)),
                        (
                            "/",
                            ExprDecl::Const(Constant::Int(i)),
                            ExprDecl::Const(Constant::Int(i2)),
                        ) => self.write(Opcode::LdInt(i / i2)),
                        (
                            "*",
                            ExprDecl::Const(Constant::Int(i)),
                            ExprDecl::Const(Constant::Int(i2)),
                        ) => self.write(Opcode::LdInt(i * i2)),
                        (
                            ">>",
                            ExprDecl::Const(Constant::Int(i)),
                            ExprDecl::Const(Constant::Int(i2)),
                        ) => self.write(Opcode::LdInt(i >> i2)),
                        (
                            "<<",
                            ExprDecl::Const(Constant::Int(i)),
                            ExprDecl::Const(Constant::Int(i2)),
                        ) => self.write(Opcode::LdInt(i << i2)),

                        (
                            "+",
                            ExprDecl::Const(Constant::Float(i)),
                            ExprDecl::Const(Constant::Float(i2)),
                        ) => self.write(Opcode::LdFloat(i + i2)),
                        (
                            "-",
                            ExprDecl::Const(Constant::Float(i)),
                            ExprDecl::Const(Constant::Float(i2)),
                        ) => self.write(Opcode::LdFloat(i - i2)),
                        (
                            "/",
                            ExprDecl::Const(Constant::Float(i)),
                            ExprDecl::Const(Constant::Float(i2)),
                        ) => self.write(Opcode::LdFloat(i / i2)),
                        (
                            "*",
                            ExprDecl::Const(Constant::Float(i)),
                            ExprDecl::Const(Constant::Float(i2)),
                        ) => self.write(Opcode::LdFloat(i * i2)),
                        (op, _, _) => {
                            self.compile(e2);
                            self.compile(e1);
                            self.write_op(op);
                        }
                    }
                } else {
                    self.compile(e2);

                    self.compile(e1);
                    self.write_op(op);
                }
            }
        }
    }
    pub fn compile_const(&mut self, c: &Constant, _p: Position) {
        match c {
            Constant::True => self.write(Opcode::LdTrue),
            Constant::False => self.write(Opcode::LdFalse),
            Constant::Null => self.write(Opcode::LdNull),
            Constant::This => self.write(Opcode::LdThis),
            Constant::Int(n) => self.write(Opcode::LdInt(n.clone())),
            Constant::Float(f) => self.write(Opcode::LdFloat(f.clone())),
            Constant::Str(s) => self.write(Opcode::LdStr(s.clone())),
            Constant::Ident(s) => {
                let s: &str = s;
                if self.locals.contains_key(s) {
                    let i = *self.locals.get(s).unwrap();
                    self.write(Opcode::LdLocal(i as u32));
                } else if self.env.contains_key(s) {
                    self.nenv += 1;
                    let pos = if !self.used_upvars.contains_key(s) {
                        let pos = self.used_upvars.len();

                        self.used_upvars.insert(s.to_owned(), pos as _);
                        pos as u32
                    } else {
                        *self.used_upvars.get(s).unwrap() as u32
                    };
                    self.write(Opcode::LdEnv(pos));
                } else {
                    let g = self.global(&Global::Var(s.to_owned()));
                    self.write(Opcode::LdGlobal(g as u32));
                }
            }
            Constant::Builtin(name) => {
                let idx = self.builtins.get(name).expect("Builtin not found").clone();
                self.write(Opcode::LdBuiltin(idx as u32));
            }
        }
    }
    pub fn write_op(&mut self, op: &str) {
        use Opcode::*;
        match op {
            "&&" => self.write(Band),
            "||" => self.write(Bor),
            "+" => self.write(Add),
            "-" => self.write(Sub),
            "/" => self.write(Div),
            "*" => self.write(Mul),
            "%" => self.write(Rem),
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
                self.compile(e);
                return Access::Field(f.to_owned());
            }
            ExprDecl::Const(Constant::This) => return Access::This,
            ExprDecl::Array(ea, ei) => {
                if let ExprDecl::Const(Constant::Int(i)) = ei.decl {
                    self.compile(ea);
                    return Access::Index(i as i32);
                }
                self.compile(ea);
                self.compile(ei);
                return Access::Array;
            }
            _ => unimplemented!(),
        }
    }

    pub fn access_get(&mut self, acc: Access) {
        match acc {
            Access::Env(i) => self.write(Opcode::LdEnv(i as _)),
            Access::Stack(i) => self.write(Opcode::LdLocal(i as _)),
            Access::Global(g) => self.write(Opcode::LdGlobal(g as _)),
            Access::Field(f) => {
                let mut h = 0xcbf29ce484222325;
                hash_bytes(&mut h, f.as_bytes());

                self.fields.insert(h, f.to_owned());
                self.write(Opcode::LdField(h));
            }
            Access::Index(i) => self.write(Opcode::LdIndex(i as _)),
            Access::This => self.write(Opcode::LdThis),
            Access::Array => {
                self.write(Opcode::LdArray);
            }
        }
    }

    pub fn access_set(&mut self, acc: Access) {
        match acc {
            Access::Env(n) => self.write(Opcode::SetEnv(n as u32)),
            Access::Stack(l) => self.write(Opcode::SetLocal(l as u32)),
            Access::Global(g) => self.write(Opcode::SetGlobal(g as u32)),
            Access::Field(f) => {
                let mut h = 0xcbf29ce484222325;
                hash_bytes(&mut h, f.as_bytes());
                nabox::vm::FIELDS.borrow_mut().insert(h, f.to_owned());
                self.fields.insert(h, f.to_owned());
                self.write(Opcode::SetField(h));
            }
            Access::Index(i) => self.write(Opcode::SetIndex(i as u32)),
            Access::This => self.write(Opcode::SetThis),
            Access::Array => self.write(Opcode::SetArray),
        }
    }

    pub fn finish(&mut self) -> Vec<Opcode> {
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
                UOP::Goto(ref lbl) => Opcode::Jump(self.labels.get(lbl).unwrap().unwrap() as u32),
                UOP::GotoF(ref lbl) => {
                    Opcode::JumpIfNot(self.labels.get(lbl).unwrap().unwrap() as u32)
                }
                UOP::GotoT(ref lbl) => {
                    Opcode::JumpIf(self.labels.get(lbl).unwrap().unwrap() as u32)
                }
                _ => Opcode::Nop,
            })
            .collect::<Vec<Opcode>>()
    }

    pub fn compile(&mut self, e: &P<Expr>) {
        match &e.decl {
            ExprDecl::Break(e) => {
                if e.is_some() {
                    let e = e.clone().unwrap();
                    self.compile(&e);
                } else {
                    self.write(Opcode::LdNull);
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
            ExprDecl::Const(c) => self.compile_const(c, e.pos.clone()),
            ExprDecl::Block(v) => {
                if v.len() == 0 {
                    self.write(Opcode::LdNull);
                } else {
                    let locals = self.locals.clone();
                    //let stack = self.stack;
                    for el in v.iter() {
                        self.compile(el);
                    }

                    /*if stack < self.stack {
                        self.write(Opcode::Pop((self.stack - stack) as u32)); // clear stack from values and locals
                    }*/
                    self.locals = locals;
                }
            }
            ExprDecl::Paren(e) => self.compile(e),
            ExprDecl::Field(e, f) => {
                self.compile(e);
                let mut h = 0xcbf29ce484222325;
                hash_bytes(&mut h, f.as_bytes());
                self.write(Opcode::LdField(h));
            }
            ExprDecl::Array(ea, ei) => {
                self.compile(ea);
                self.compile(ei);
                self.write(Opcode::LdArray);
            }
            ExprDecl::Var(_, name, init) => {
                match init {
                    Some(e) => match &e.decl {
                        ExprDecl::Function(params, body) => {
                            self.compile_function(params, body, Some(name));
                        }
                        _ => self.compile(e),
                    },
                    None => self.write(Opcode::LdNull),
                }
                let id = self.locals.len() as u32;
                self.write(Opcode::SetLocal(id));

                self.locals.insert(name.to_owned(), id as i32);
            }

            ExprDecl::Assign(e1, e2) => {
                let a = self.compile_access(e1);
                self.compile(e2);
                self.access_set(a);
            }
            ExprDecl::Binop(op, e1, e2) => {
                self.compile_binop(op, e1, e2);
            }
            ExprDecl::Function(params, e) => {
                self.compile_function(params, e, None);
            }
            ExprDecl::Return(e) => {
                match e {
                    Some(e) => self.compile(e),
                    None => self.write(Opcode::LdNull),
                }

                //let stack = self.stack;
                self.write(Opcode::Ret);
                //self.stack = stack;
            }
            ExprDecl::While(cond, body) => {
                let start = self.new_empty_label();
                let end = self.new_empty_label();
                self.breaks.push(end.clone());
                self.continues.push(start.clone());
                self.label_here(&start);
                self.compile(cond);
                self.emit_gotof(&end);
                self.compile(body);
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
                    self.compile(value);
                    self.compile(cond);
                    self.write(Opcode::Eq);
                    self.emit_gotof(&l1);
                    self.compile(&expr);
                    self.emit_goto(&end);
                    self.label_here(&l1);
                }
                if default_.is_some() {
                    self.emit_goto(&orl);
                }
                self.label_here(&orl);
                if default_.is_some() {
                    self.compile(&default_.clone().unwrap());
                    self.emit_goto(&end);
                }
                self.label_here(&end);
            }

            ExprDecl::If(e, e1, e2) => {
                //let stack = self.stack;

                let lbl_false = self.new_empty_label();
                self.compile(&e);
                self.emit_gotof(&lbl_false);
                self.compile(e1);
                self.label_here(&lbl_false);
                if e2.is_some() {
                    let e2 = e2.clone().unwrap();
                    self.compile(&e2);
                }
            }
            ExprDecl::Call(e, el) => {
                match &e.decl {
                    ExprDecl::Const(Constant::Builtin(name)) => {
                        let builtin: &str = name;
                        match builtin {
                            "new" => {
                                self.compile(&el[0]);
                                self.write(Opcode::New);
                                return;
                            }
                            "hash" => {
                                self.compile(&el[0]);
                                self.write(Opcode::Hash);
                                return;
                            }
                            "typeof" => {
                                self.compile(&el[0]);
                                self.write(Opcode::TypeOf);
                                return;
                            }
                            _ => (),
                        }
                    }
                    ExprDecl::Field(e, f) => {
                        for e in el.iter().rev() {
                            self.compile(e);
                        }
                        self.compile(e);
                        self.compile(e);
                        let mut h = 0xcbf29ce484222325;
                        hash_bytes(&mut h, f.as_bytes());
                        self.write(Opcode::LdField(h));
                        self.write(Opcode::ObjCall(el.len() as u32));
                        return;
                    }
                    _ => (),
                }
                for x in el.iter().rev() {
                    self.compile(x);
                }
                self.compile(e);
                self.write(Opcode::Call(el.len() as _));
            }
            ExprDecl::Yield(e) => {
                self.compile(e);
                self.write(Opcode::Yield);
            }
            ExprDecl::Unop(op, e) => {
                self.compile(e);
                let op: &str = op;
                match op {
                    "-" => self.write(Opcode::Neg),
                    _ => (),
                }
            }
            v => panic!("{:?}", v),
        }
    }

    pub fn compile_function(&mut self, params: &[String], e: &P<Expr>, vname: Option<&str>) {
        let mut ctx = Context {
            g: self.g.clone(), // we don't clone this globals, basically just copy ptr,
            ops: Vec::new(),
            pos: Vec::new(),
            optimize: self.optimize,
            limit: self.stack,
            stack: self.stack,
            locals: LinkedHashMap::new(),
            fields: self.fields.clone(),
            nenv: 0,
            env: self.locals.clone(),
            cur_pos: (0, 0),
            cur_file: self.cur_file.clone(),
            continues: vec![],
            breaks: vec![],
            builtins: self.builtins.clone(),
            labels: self.labels.clone(),
            used_upvars: LinkedHashMap::new(),
        };
        for (idx, p) in params.iter().enumerate() {
            ctx.stack += 1;
            ctx.locals.insert(p.to_owned(), idx as i32);
        }

        let s = ctx.stack.clone();
        let gid = ctx.g.table.len();
        if vname.is_some() {
            self.g
                .globals
                .insert(Global::Var(vname.unwrap().to_owned()), gid as i32);
        ctx.compile(e);

        ctx.write(Opcode::Ret);
        ctx.check_stack(s, "");

        ctx.g.functions.push((
            ctx.ops.clone(),
            ctx.pos.clone(),
            gid as i32,
            params.len() as i32,
        ));
        ctx.g.table.push(Global::Func(gid as i32, -1));
        
        }
        for (k, v) in ctx.labels.iter() {
            self.labels.insert(k.clone(), v.clone());
        }
        if ctx.nenv > 0 {
            /*let mut a = vec!["".to_string(); ctx.nenv as usize];
            for (v, i) in ctx.env.iter() {
                a[*i as usize] = v.clone();
            }
            for x in a.iter() {
                self.compile_const(&Constant::Ident(x.to_owned()), e.pos);
            }*/
            for (var, _) in ctx.used_upvars.iter() {
                self.compile_const(&Constant::Ident(var.to_owned()), e.pos);
            }
            self.write(Opcode::LdGlobal(gid as _));

            self.write(Opcode::MakeEnv((ctx.used_upvars.len()) as u32));
        } else {
            self.write(Opcode::LdGlobal(gid as _));
        }
    }
}

pub fn compile_ast(ast: Vec<P<Expr>>, optimize: bool) -> Context {
    let g = Globals {
        globals: LinkedHashMap::new(),
        objects: LinkedHashMap::new(),
        functions: vec![],
        table: vec![],
    };
    let mut ctx = Context {
        g: Cell::new(g),
        stack: 0,
        optimize,
        limit: -1,
        locals: LinkedHashMap::new(),
        ops: vec![],
        env: LinkedHashMap::new(),
        fields: Cell::new(LinkedHashMap::new()),
        labels: LinkedHashMap::new(),
        nenv: 0,
        pos: Vec::new(),
        cur_pos: (0, 0),
        builtins: LinkedHashMap::new(),
        breaks: vec![],
        continues: vec![],
        cur_file: String::from("_"),
        used_upvars: LinkedHashMap::new(),
    };
    ctx.builtins.insert("load".into(), 0);
    ctx.builtins.insert("string".into(), 1);
    ctx.builtins.insert("print".into(), 2);
    ctx.builtins.insert("array".into(), 3);
    ctx.builtins.insert("alen".into(), 4);
    ctx.builtins.insert("apush".into(), 5);
    ctx.builtins.insert("apop".into(), 6);
    ctx.builtins.insert("aset".into(), 7);
    ctx.builtins.insert("aget".into(), 8);
    ctx.builtins.insert("os_string".into(), 9);
    ctx.builtins.insert("thread_spawn".into(), 10);
    ctx.builtins.insert("thread_join".into(), 11);
    ctx.builtins.insert("exports".into(), 0xff);
    ctx.builtins.insert("loader".into(), 0xff + 1);
    ctx.builtins.insert("loadmodule".into(), 12);
    ctx.builtins.insert("string_bytes".into(), 13);
    ctx.builtins.insert("string_from_bytes".into(), 14);
    ctx.builtins.insert("file".into(), 15);
    ctx.builtins.insert("file_read".into(), 16);
    ctx.builtins.insert("file_write".into(), 17);
    ctx.builtins.insert("file_size".into(), 18);
    ctx.builtins.insert("int_to_bytes".into(), 19);
    ctx.builtins.insert("int_from_bytes".into(), 20);
    ctx.builtins.insert("float_to_bits".into(), 21);
    ctx.builtins.insert("float_from_bits".into(), 22);
    ctx.builtins.insert("string_len".into(), 23);
    ctx.builtins.insert("areverse".into(), 24);
    ctx.builtins.insert("args".into(), 25);
    ctx.builtins.insert("read_line".into(), 26);
    ctx.builtins.insert("read_char".into(), 27);
    ctx.builtins.insert("char_to_string".into(), 28);
    ctx.builtins.insert("sprintf".into(), 29);
    ctx.builtins.insert("string_chars".into(), 30);
    ctx.builtins.insert("char_to_int".into(), 31);

    let ast = P(Expr {
        pos: Position::new(0, 0),
        decl: ExprDecl::Block(ast.clone()),
    });

    //ctx.scan_labels(true, true, &ast);
    ctx.compile(&ast);

    if ctx.g.functions.len() != 0 || ctx.g.objects.len() != 0 {
        let ctxops = ctx.ops.clone();
        let _ctxpos = ctx.pos.clone();
        let ops = vec![];
        let pos = vec![];
        ctx.ops = ops;
        ctx.pos = pos;
        ctx.write(Opcode::Jump(0));

        for (fops, fpos, gid, nargs) in ctx.g.functions.iter().rev() {
            let g = ctx.g.borrow_mut();

            g.table[*gid as usize] = Global::Func(ctx.ops.len() as i32, *nargs);

            for op in fops.iter() {
                ctx.ops.push(op.clone());
            }
            ctx.ops[0] = UOP::Op(Opcode::Jump(ctx.ops.len() as u32));
            for op in fpos.iter() {
                ctx.pos.push(op.clone());
            }
        }
        for op in ctxops.iter() {
            ctx.ops.push(op.clone());
        }
    }

    ctx
}
