use crate::intern;
use crate::map::LinkedHashMap;
use crate::token::Position;
use crate::vm::opcodes::*;
use crate::vm::value::*;
use crate::vm::*;
use crate::P;
#[derive(Clone, Debug)]
pub enum UOP {
    Goto(String),
    GotoF(String),
    GotoT(String),
    Label(String),
    PAddr(String),
    BEnd,
    BStart,
    Op(Opcode),
}

pub struct Compiler<'a> {
    pub frame: &'a mut Frame<'a>,
    stack_size: u32,
    code: Vec<UOP>,
    pos: Position,
    id: usize,
    functions: Vec<(Vec<UOP>, u32, Vec<String>)>,
    pub labels: LinkedHashMap<String, Option<usize>>,
    pub breaks: Vec<String>,
    pub continues: Vec<String>,
    ret: String,
}

use crate::ast::*;
use crate::parser::Parser;
use crate::reader::Reader;

impl<'a> Compiler<'a> {
    pub fn new(m: &'a mut Frame<'a>) -> Compiler<'a> {
        Compiler {
            frame: m,
            stack_size: 0,
            code: vec![],
            ret: String::new(),
            id: 0,
            pos: Position::new(0, 0),
            functions: vec![],
            labels: LinkedHashMap::new(),
            breaks: vec![],
            continues: vec![],
        }
    }

    pub fn pos(&self) -> usize {
        self.code.len()
    }

    fn start(&mut self) {
        self.code.push(UOP::BStart);
    }
    fn end(&mut self) {
        self.code.push(UOP::BEnd);
    }

    pub fn emit_goto(&mut self, to: &str) {
        self.code.push(UOP::Goto(to.to_owned()));
    }
    pub fn emit_gotof(&mut self, to: &str) {
        self.code.push(UOP::GotoF(to.to_owned()));
    }

    pub fn emit_paddr(&mut self, t: &str) {
        self.code.push(UOP::PAddr(t.to_owned()));
    }

    pub fn emit_gotot(&mut self, to: &str) {
        self.code.push(UOP::GotoT(to.to_owned()));
    }

    pub fn write(&mut self, op: Opcode) {
        let loc = self.code.len();
        self.frame.m.line_no.insert((loc, op), self.pos);
        self.code.push(UOP::Op(op));
    }

    pub fn cjmp(&mut self, cond: bool) -> impl Fn(&mut Self) {
        let p: usize = self.pos();

        self.write(Opcode::Jump(0));
        move |c| {
            let p2 = c.pos() - p.clone();
            c.code[p] = if cond.clone() {
                UOP::Op(Opcode::JumpIf(p2 as u32))
            } else {
                UOP::Op(Opcode::JumpIfFalse(p2 as u32))
            };
        }
    }

    pub fn jmp(&mut self) -> impl Fn(&mut Self) {
        let p = self.pos();

        self.write(Opcode::Jump(0));

        move |c| {
            let p2 = c.pos() - p;
            c.code[p] = UOP::Op(Opcode::Jump(p2 as u32));
        }
    }

    pub fn new_empty_label(&mut self) -> String {
        let lab_name = self.labels.len().to_string();
        self.labels.insert(lab_name.clone(), None);
        lab_name
    }

    pub fn label_here(&mut self, label: &str) {
        self.code.push(UOP::Label(label.to_owned()));
        //*self.labels.get_mut(label).unwrap() = Some(self.ops.len());
    }

    pub fn goto(&mut self, p: u32) {
        self.write(Opcode::Jump(p - self.pos() as u32));
    }
    pub fn new_constant(&mut self, v: impl Into<ValueData>) -> usize {
        let loc = self.frame.m.constants.len();
        self.frame.m.constants.push(v.into());
        loc
    }

    pub fn finish(&mut self) -> Vec<Opcode> {
        for (idx, op) in self.code.iter().enumerate() {
            match op {
                UOP::Label(l) => {
                    let pos = idx;
                    self.labels.insert(l.to_owned(), Some(pos));
                }
                _ => (),
            }
        }
        let code = self
            .code
            .iter()
            .map(|i| match *i {
                UOP::Op(ref op) => op.clone(),
                UOP::Goto(ref lbl) => Opcode::Jump(self.labels.get(lbl).unwrap().unwrap() as u32),
                UOP::GotoF(ref lbl) => {
                    Opcode::JumpIfFalse(self.labels.get(lbl).unwrap().unwrap() as u32)
                }
                UOP::PAddr(ref lbl) => {
                    Opcode::PushCatch(self.labels.get(lbl).unwrap().unwrap() as usize)
                }
                UOP::GotoT(ref lbl) => {
                    Opcode::JumpIf(self.labels.get(lbl).unwrap().unwrap() as u32)
                }
                UOP::BEnd => Opcode::BlockEnd,
                UOP::BStart => Opcode::BlockStart,
                _ => Opcode::Label,
            })
            .collect::<Vec<Opcode>>();
        code
    }

    pub fn include(&mut self, name: &str) {
        let name = name.to_owned();
        let cur_dir = std::env::current_dir()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let cur_path = format!("{}/{}", cur_dir, name);
        let path = if std::path::Path::new(&cur_path).exists() {
            cur_path
        } else {
            let home_dir = option_env!("JAZZ_HOME");
            if let Some(home_dir) = home_dir {
                format!("{}/{}", home_dir, name)
            } else {
                name
            }
        };

        let mut ast = vec![];

        let r = Reader::from_file(&path).unwrap();
        let mut p = Parser::new(r, &mut ast);
        p.parse().unwrap();
        for expr in ast.iter() {
            self.compile(expr);
        }
    }

    pub fn compile(&mut self, expr: &Expr) {
        self.pos = expr.pos;
        self.id = expr.id;
        match &expr.decl {
            ExprDecl::Import(filename) => {
                self.compile(&Expr {
                    id: expr.id,
                    pos: expr.pos,
                    decl: ExprDecl::Call(
                        crate::P(Expr {
                            id: expr.id,
                            pos: expr.pos,
                            decl: ExprDecl::Const(Constant::Str("require".to_owned())),
                        }),
                        vec![crate::P(Expr {
                            id: expr.id,
                            pos: expr.pos,
                            decl: ExprDecl::Const(Constant::Str(filename.to_owned())),
                        })],
                    ),
                });
                self.write(Opcode::DeclVar(intern(filename)));
            }
            ExprDecl::FromImpot(filename, decls) => {
                let c = self.new_constant(filename);
                self.write(Opcode::LoadConst(c as _));
                self.write(Opcode::NewObj);
                self.write(Opcode::LoadVar(intern("require")));
                self.write(Opcode::Call(1));

                let name = format!("@import_{}", expr.id);

                self.write(Opcode::DeclVar(intern(&name)));
                for decl in decls.iter() {
                    self.write(Opcode::LoadVar(intern(&name)));
                    let s = self.new_constant(decl);
                    self.write(Opcode::LoadConst(s as _));
                    self.write(Opcode::Load);
                    self.write(Opcode::DeclVar(intern(decl)));
                }
            }
            ExprDecl::Include(name) => self.include(name),
            ExprDecl::Const(constant) => match constant {
                Constant::Ident(name) => self.write(Opcode::LoadVar(intern(name))),
                Constant::Int(val) => self.write(Opcode::LoadInt(*val)),
                Constant::Float(num) => {
                    let loc = self.new_constant(*num);
                    self.write(Opcode::LoadConst(loc as _));
                }
                Constant::This => self.write(Opcode::LoadVar(intern("this"))),
                Constant::Null => self.write(Opcode::LoadNil),
                Constant::Undefined => self.write(Opcode::LoadUndef),
                Constant::Array(values) => {
                    for value in values.iter().rev() {
                        self.compile(value);
                    }
                    self.write(Opcode::ConstructArray(values.len() as u32));
                }
                Constant::True => self.write(Opcode::LoadTrue),
                Constant::False => self.write(Opcode::LoadFalse),
                Constant::Str(s) => {
                    let loc = self.new_constant(s);
                    self.write(Opcode::LoadConst(loc as _));
                }
                _ => unreachable!(),
            },
            ExprDecl::Yield(expr) => {
                self.end();
                self.compile(expr);
                self.write(Opcode::Yield);
                self.start();
            }
            ExprDecl::Array(array, idx) => {
                self.compile(array);
                self.compile(idx);
                self.write(Opcode::Load);
            }
            ExprDecl::Block(exprs) => {
                self.start();
                self.write(Opcode::PushEnv);
                for expr in exprs.iter() {
                    self.compile(expr);
                }
                //self.write(Opcode::Pop(self.stack_size));
                self.write(Opcode::PopEnv);
                self.end();
            }
            ExprDecl::Break(expr) => {
                match expr {
                    Some(expr) => self.compile(expr),
                    None => self.write(Opcode::LoadNil),
                };
                let b = self.breaks.last().expect("wrong break").clone();
                self.emit_goto(&b);
            }
            ExprDecl::Continue => {
                let c = self.continues.last().expect("wrong continue").clone();
                self.emit_goto(&c);
            }
            ExprDecl::Return(expr) => {
                match expr {
                    Some(expr) => self.compile(expr),
                    None => self.write(Opcode::LoadNil),
                }
                self.write(Opcode::PopEnv);
                let r = self.ret.clone();
                self.emit_goto(&r);
            }
            ExprDecl::Throw(expr) => {
                self.compile(expr);
                self.write(Opcode::Throw);
            }
            ExprDecl::Unop(op, expr) => {
                self.compile(expr);
                let op: &str = op;
                match op {
                    "-" => self.write(Opcode::Neg),
                    "!" => self.write(Opcode::Not),
                    "+" => {}
                    _ => unreachable!(),
                };
            }
            ExprDecl::Binop(op, lhs, rhs) => {
                self.compile(rhs);
                self.compile(lhs);
                let op: &str = op;
                match op {
                    "+" => self.write(Opcode::Add),
                    "-" => self.write(Opcode::Sub),
                    "/" => self.write(Opcode::Div),
                    "*" => self.write(Opcode::Mul),
                    "%" => self.write(Opcode::Rem),
                    "&&" => self.write(Opcode::And),
                    "||" => self.write(Opcode::Or),
                    ">" => self.write(Opcode::Gt),
                    "<" => self.write(Opcode::Lt),
                    ">=" => self.write(Opcode::Ge),
                    "<=" => self.write(Opcode::Le),
                    "==" => self.write(Opcode::Eq),
                    "!=" => self.write(Opcode::Ne),
                    ">>" => self.write(Opcode::Shr),
                    "<<" => self.write(Opcode::Shl),
                    "|" => self.write(Opcode::BitOr),
                    "&" => self.write(Opcode::BitAnd),
                    "^" => self.write(Opcode::BitXor),
                    _ => unreachable!(),
                };
            }
            ExprDecl::Assign(lhs, rhs) => {
                self.write(Opcode::Dup);
                match &lhs.decl {
                    ExprDecl::Field(obj, field) => {
                        self.compile(obj);
                        let loc = self.new_constant(field);
                        self.write(Opcode::LoadConst(loc as _));
                        self.compile(rhs);
                        self.write(Opcode::Store);
                    }
                    ExprDecl::Array(array, idx) => {
                        self.compile(array);
                        self.compile(idx);

                        self.compile(rhs);
                        self.write(Opcode::Store);
                    }
                    ExprDecl::Const(Constant::Ident(name)) => {
                        self.compile(rhs);
                        self.write(Opcode::StoreVar(intern(name)))
                    }
                    ExprDecl::Const(Constant::This) => {
                        self.compile(rhs);
                        self.write(Opcode::StoreVar(intern("this")));
                    }

                    _ => panic!("Can not assign"),
                };
            }
            ExprDecl::Function(args, body) => {
                self.compile_function(&body, &args, None);
            }
            ExprDecl::FunctionDecl(name, args, body) => {
                self.compile_function(&body, &args, Some(name));
            }
            ExprDecl::New(e, el) => {
                for arg in el.iter().rev() {
                    self.compile(arg);
                }
                self.write(Opcode::NewObj);
                self.compile(e);
                self.write(Opcode::Call(el.len() as _));
            }

            ExprDecl::Call(e, el) => {
                for arg in el.iter().rev() {
                    self.compile(arg);
                }
                match &e.decl {
                    ExprDecl::Field(obj, field) => {
                        self.compile(obj);
                        self.write(Opcode::Dup);
                        let pos = self.new_constant(field);
                        self.write(Opcode::LoadConst(pos as _));
                        self.write(Opcode::Load);
                    }
                    _ => {
                        self.write(Opcode::NewObj);
                        self.compile(e);
                    }
                }
                self.write(Opcode::Call(el.len() as _));
            }
            ExprDecl::Var(_, name, init) => {
                match init {
                    Some(val) => self.compile(val),
                    None => self.write(Opcode::LoadUndef),
                };
                self.write(Opcode::DeclVar(intern(name)));
            }

            ExprDecl::Try(expr, var_name, block) => {
                let catch_lbl = self.new_empty_label();
                let end_lbl = self.new_empty_label();
                self.emit_paddr(&catch_lbl);
                self.start();
                self.compile(expr);
                self.emit_goto(&end_lbl);
                self.end();
                self.label_here(&catch_lbl);
                self.write(Opcode::PushEnv);
                self.write(Opcode::DeclVar(intern(var_name)));
                self.start();
                self.compile(block);
                self.write(Opcode::PopEnv);
                self.emit_goto(&end_lbl);
                self.end();

                self.label_here(&end_lbl);
            }

            ExprDecl::If(cond, body, otherwise) => {
                self.end();
                self.start();
                self.compile(cond);

                let end = self.new_empty_label();
                let if_false = self.new_empty_label();
                //let merge = self.new_empty_label();
                self.emit_gotof(&if_false);
                self.end();
                self.start();
                self.compile(body);
                self.emit_goto(&end);
                self.end();
                self.label_here(&if_false);
                self.start();
                if otherwise.is_some() {
                    self.compile(otherwise.as_ref().unwrap());
                }
                self.emit_goto(&end);
                self.end();
                self.start();
                self.label_here(&end);
                self.end();
            }
            ExprDecl::DoWhile(cond, body) => {
                let end = self.new_empty_label();
                let start = self.new_empty_label();
                self.breaks.push(end.clone());
                self.continues.push(start.clone());
                self.end();
                self.label_here(&start);
                self.start();
                self.compile(body);
                self.end();
                self.start();
                self.compile(cond);
                self.emit_gotot(&start);
                self.end();
                self.label_here(&end);
            }

            ExprDecl::ForIn(name, in_, body) => {
                let check = self.new_empty_label();
                let end = self.new_empty_label();
                self.breaks.push(end.clone());
                self.continues.push(check.clone());
                self.end();
                self.write(Opcode::PushEnv);
                self.write(Opcode::LoadNil);
                self.write(Opcode::DeclVar(intern(name)));
                self.compile(in_);

                self.write(Opcode::NewIter);
                self.write(Opcode::DeclVar(intern("@iterator")));
                self.label_here(&check);
                self.write(Opcode::LoadVar(intern("@iterator")));
                self.write(Opcode::IterHasNext);
                self.emit_gotof(&end);
                self.write(Opcode::PushEnv);
                self.write(Opcode::LoadVar(intern("@iterator")));
                self.write(Opcode::IterNext);
                self.write(Opcode::StoreVar(intern(name)));
                self.compile(body);
                self.write(Opcode::PopEnv);
                self.emit_goto(&check);
                self.end();
                self.label_here(&end);
            }
            ExprDecl::While(cond, body) => {
                let check = self.new_empty_label();
                let end = self.new_empty_label();
                self.breaks.push(end.clone());
                self.continues.push(check.clone());
                self.end();

                self.label_here(&check);
                self.start();
                self.compile(cond);

                self.emit_gotof(&end);
                self.end();
                self.start();
                self.compile(body);
                self.emit_goto(&check);
                self.end();
                self.label_here(&end);
            }
            ExprDecl::Field(obj, field) => {
                let c = self.new_constant(field);
                self.compile(obj);
                self.write(Opcode::LoadConst(c as _));
                self.write(Opcode::Load);
            }
            ExprDecl::Paren(expr) => self.compile(expr),
            x => panic!("{:?}", x),
        }
    }

    pub fn compile_ast(&mut self, ast: &[P<Expr>], declare_builtins: bool) {
        let ret = self.new_empty_label();
        self.ret = ret.clone();
        for expr in ast.iter() {
            self.compile(expr);
        }

        self.label_here(&ret);
        self.write(Opcode::Return);
        let mut addresses = LinkedHashMap::new();
        if !self.functions.is_empty() {
            let ctxops = self.code.clone();
            let ops = vec![];
            self.code = ops;
            self.write(Opcode::Jump(0));
            for (fops, gid, _) in self.functions.iter().cloned().rev() {
                addresses.insert(gid, self.code.len());
                for op in fops.iter() {
                    self.code.push(op.clone());
                }
                self.code[0] = UOP::Op(Opcode::Jump(self.code.len() as _));
            }

            for op in ctxops.iter() {
                self.code.push(op.clone());
            }
        }
        let code = self.finish();
        let gc_code = crate::vm::value::new_ref(code);

        if !self.functions.is_empty() {
            for (_, gid, args) in self.functions.iter().rev() {
                self.frame.m.constants[*gid as usize] =
                    ValueData::Function(new_ref(Function::Regular {
                        environment: new_object(),
                        addr: *addresses.get(&gid).unwrap(),
                        yield_pos: None,
                        args: args.clone(),
                        //constants: ref_,
                        code: gc_code.clone(),
                        yield_env: new_object(),
                        set: false,
                        get: false,
                    }));
            }
        }
        let position = self
            .frame
            .m
            .line_no
            .iter()
            .map(|(_, val)| *val)
            .collect::<Vec<_>>();
        self.frame.m.line_no.clear();
        for ((i, op), pos) in gc_code.borrow().iter().enumerate().zip(&position) {
            self.frame.m.line_no.insert((i, *op), pos.clone());
        }
        self.frame.code = gc_code;
        //self.frame.push_env();
        self.frame.pc = 0;
        //crate::ngc::gc_add_root(self.frame.env.gc());
        if declare_builtins {
            crate::vm::runtime::register_builtins(self.frame.env.clone());
        }
    }

    pub fn compile_function(&mut self, body: &Expr, args: &Vec<String>, name: Option<&str>) {
        let code = self.code.clone();
        self.code = vec![];
        let ret_lbl = self.new_empty_label();
        self.ret = ret_lbl.clone();
        self.compile(body);
        self.label_here(&ret_lbl);
        self.write(Opcode::Return);

        let gid = self.frame.m.constants.len();

        self.functions
            .push((self.code.clone(), gid as _, args.clone()));

        self.new_constant(ValueData::Undefined);

        self.code = code;
        if name.is_none() {
            self.write(Opcode::LoadConst(gid as _));
            self.write(Opcode::InitEnv);
        } else {
            self.write(Opcode::LoadConst(gid as _));
            self.write(Opcode::DeclVar(intern(*name.as_ref().unwrap())));
            self.write(Opcode::LoadVar(intern(*name.as_ref().unwrap())));
            self.write(Opcode::InitEnv);
            self.write(Opcode::StoreVar(intern(*name.as_ref().unwrap())));
        }
    }
}
