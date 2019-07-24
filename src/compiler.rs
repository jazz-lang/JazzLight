use crate::vm::*;
use crate::vm::value::*;
use crate::vm::opcodes::*;
use crate::gc::{Gc,gc};
use crate::intern;
use crate::token::Position;
use hashlink::LinkedHashMap;
use crate::P;
#[derive(Clone, Debug)]
pub enum UOP {
    Goto(String),
    GotoF(String),
    GotoT(String),
    Label(String),
    PAddr(String),
    Op(Opcode),
}

pub struct Compiler<'a> {
    pub frame: Frame<'a>,
    stack_size: u32,
    code: Vec<UOP>,
    pos: Position,
    id: usize,
    functions: Vec<(Vec<UOP>,u32,Vec<String>)>,
    pub labels: LinkedHashMap<String, Option<usize>>,   
    pub breaks: Vec<String>,
    pub continues: Vec<String>,

}

use crate::ast::*;

impl<'a> Compiler<'a> {
    pub fn new(m: &'a mut Machine) -> Compiler<'a> {
        Compiler {
            frame: Frame::new(m),
            stack_size: 0,
            code: vec![],
            id: 0,
            pos: Position::new(0,0),
            functions: vec![],
            labels: LinkedHashMap::new(),
            breaks: vec![],
            continues: vec![]
        }
    }

    pub fn pos(&self) -> usize {
        self.code.len()
    }

        pub fn emit_goto(&mut self, to: &str) {
        self.code.push(UOP::Goto(to.to_owned()));
    }
    pub fn emit_gotof(&mut self, to: &str) {
        self.code.push(UOP::GotoF(to.to_owned()));
    }

    pub fn emit_paddr(&mut self,t: &str) {
        self.code.push(UOP::PAddr(t.to_owned()));
    }

    pub fn emit_gotot(&mut self, to: &str) {
        self.code.push(UOP::GotoT(to.to_owned()));
    }

    pub fn write(&mut self,op: Opcode) {
        let loc = self.code.len();
        self.frame.m.line_no.insert((loc,op),self.pos);
        self.code.push(UOP::Op(op));
    }

    pub fn cjmp(&mut self,cond: bool) -> impl Fn(&mut Self) {
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

    pub fn goto(&mut self,p: u32) {
        self.write(Opcode::Jump(p - self.pos() as u32));
    } 
    pub fn new_constant(&mut self,v: impl Into<ValueData>) -> usize {
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
        self.code
            .iter()
            .map(|i| match *i {
                UOP::Op(ref op) => op.clone(),
                UOP::Goto(ref lbl) => Opcode::Jump(self.labels.get(lbl).unwrap().unwrap() as u32),
                UOP::GotoF(ref lbl) => {
                    Opcode::JumpIfFalse(self.labels.get(lbl).unwrap().unwrap() as u32)
                }
                UOP::PAddr(ref lbl) => Opcode::PushCatch(self.labels.get(lbl).unwrap().unwrap() as usize),
                UOP::GotoT(ref lbl) => {
                    Opcode::JumpIf(self.labels.get(lbl).unwrap().unwrap() as u32)
                }
                _ => Opcode::Nop,
            })
            .collect::<Vec<Opcode>>()
    }

    pub fn compile(&mut self,expr: &Expr) {
        self.pos = expr.pos;
        self.id = expr.id;  
        match &expr.decl {
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
                _ => unreachable!()
            }
            ExprDecl::Yield(expr) => {
                self.compile(expr);
                self.write(Opcode::Yield);
            }
            ExprDecl::Array(array,idx) => {
                self.compile(idx);
                self.compile(array);
                self.write(Opcode::Load);
            }
            ExprDecl::Block(exprs) => {
                self.write(Opcode::PushEnv);
                for expr in exprs.iter() {
                    self.compile(expr);
                }
                self.write(Opcode::Pop(self.stack_size));
                self.write(Opcode::PopEnv);
            }
            ExprDecl::Break(expr) => {
                match expr {
                    Some(expr) => self.compile(expr),
                    None => self.write(Opcode::LoadNil)
                };
                let b = self.breaks.last().expect("wrong break").clone();
                self.emit_goto(&b);
            },
            ExprDecl::Continue => {
                let c = self.continues.last().expect("wrong continue").clone();
                self.emit_goto(&c);
            },
            ExprDecl::Return(expr) => {
                match expr {
                    Some(expr) => self.compile(expr),
                    None => self.write(Opcode::LoadNil)
                }
                self.write(Opcode::PopEnv);
                self.write(Opcode::Return);
            } 
            ExprDecl::Throw(expr) => {
                self.compile(expr);
                self.write(Opcode::Throw);
            }
            ExprDecl::Unop(op,expr) => {
                self.compile(expr);
                let op: &str = op;
                match op {
                    "-" => self.write(Opcode::Neg),
                    "!" => self.write(Opcode::Not),
                    "+" => {},
                    _ => unreachable!()
                };
            }
            ExprDecl::Binop(op,lhs,rhs) => {
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
                    _ => unreachable!()
                };
            }
            ExprDecl::Assign(lhs,rhs) => {
                self.compile(rhs);
                self.write(Opcode::Dup);
                match &lhs.decl {
                    ExprDecl::Field(obj,field) => {
                        let loc = self.new_constant(field);
                        self.write(Opcode::LoadConst(loc as _));
                        self.compile(obj);
                        self.write(Opcode::Store);
                    }
                    ExprDecl::Array(array,idx) => {
                        self.compile(idx);
                        self.compile(array);
                        self.write(Opcode::Store);
                    }
                    ExprDecl::Const(Constant::Ident(name)) => self.write(Opcode::StoreVar(intern(name))),
                    ExprDecl::Const(Constant::This) => self.write(Opcode::StoreVar(intern("this"))),
                    _ => panic!("Can not assign")
                };
            }
            ExprDecl::Function(args,body) => {
                self.compile_function(&body,&args,None);
            } 
            ExprDecl::FunctionDecl(name,args,body) => {
                self.compile_function(&body,&args,Some(name));
            }
            ExprDecl::Call(e,el) => {
                for arg in el.iter() {
                    self.compile(arg);
                }
                match &e.decl {
                    ExprDecl::Field(obj,field) => {
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
            ExprDecl::Var(_,name,init) => {
                match init {
                    Some(val) => self.compile(val),
                    None => self.write(Opcode::LoadUndef)
                };
                self.write(Opcode::DeclVar(intern(name)));
            }
            ExprDecl::If(cond,body,otherwise) => {
                self.compile(cond);
                
                let end = self.new_empty_label();
                let if_false = if otherwise.is_some() {self.new_empty_label()} else {end.clone()};
                self.emit_gotof(&if_false);
                self.compile(body);
                
                self.emit_goto(&end);
                if otherwise.is_some() {
                    self.label_here(&if_false);
                    self.compile(otherwise.as_ref().unwrap());
                    self.emit_goto(&end);
                }
                if otherwise.is_some() {
                    self.label_here(&end);
                }
            }
            ExprDecl::Try(expr,var_name,block) => {
                let catch_lbl = self.new_empty_label();
                let end_lbl = self.new_empty_label();
                self.emit_paddr(&catch_lbl);
                self.compile(expr);
                self.emit_goto(&end_lbl);
                self.label_here(&catch_lbl);
                self.write(Opcode::PushEnv);
                self.write(Opcode::DeclVar(intern(var_name)));
                self.compile(block);
                self.write(Opcode::PopEnv);
                self.emit_goto(&end_lbl);

                self.label_here(&end_lbl);
            }
            ExprDecl::While(cond,body) => {
                let check = self.new_empty_label();
                let end = self.new_empty_label();
                self.breaks.push(end.clone());
                self.continues.push(check.clone());
                self.label_here(&check);
                self.compile(cond);
                self.emit_gotof(&end);
                self.compile(body);
                self.emit_goto(&check);

                self.label_here(&end);

            }
            ExprDecl::Field(obj,field) => {
                let c = self.new_constant(field);
                self.compile(obj);
                self.write(Opcode::LoadConst(c as _));
                self.write(Opcode::Load);
            }
            ExprDecl::Paren(expr) => self.compile(expr),
            x => panic!("{:?}",x)
        }
    }

    pub fn compile_ast(&mut self,ast: &[P<Expr>]) {
        for expr in ast.iter() {
            self.compile(expr);
        }
        let mut addresses = LinkedHashMap::new();
        if !self.functions.is_empty() {
            let ctxops = self.code.clone();
            let ops = vec![];
            self.code = ops;
            self.write(Opcode::Jump(0));
            for (fops,gid,_) in self.functions.iter().cloned().rev() {
                addresses.insert(gid,self.code.len());
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
        let gc_code = wrc::WRC::new(std::cell::RefCell::new(code));

        if !self.functions.is_empty() {
            for (_,gid,args) in self.functions.iter().rev() {
                self.frame.m.constants[*gid as usize] = ValueData::Function(
                    new_ref(
                        Function::Regular {
                            environment: new_object(),
                            addr: *addresses.get(&gid).unwrap(),
                            yield_pos: None,
                            args: args.clone(),
                            code: gc_code.clone(),
                            yield_env: new_object(),
                        }
                    )
                );
            }
        }
        let position = self.frame.m.line_no.iter().map(|(_,val)| *val).collect::<Vec<_>>();
        self.frame.m.line_no.clear();
        for ((i,op),pos) in gc_code.borrow().iter().enumerate().zip(&position) {
            self.frame.m.line_no.insert((i,*op),pos.clone());
        }
        self.frame.code = gc_code;
        self.frame.env = new_object();
        self.frame.pc = 0;
        crate::vm::runtime::register_builtins(&mut self.frame);
    }

    pub fn compile_function(&mut self,body: &Expr,args: &Vec<String>,name: Option<&str>) {
        let code = self.code.clone();
        self.code = vec![];
        let ret_lbl = self.new_empty_label();
        self.compile(body);
        self.label_here(&ret_lbl);
        self.write(Opcode::Return);

        let gid = self.frame.m.constants.len();
        
        self.functions.push(
            (
                self.code.clone(),
                gid as _,
                args.clone()
            )
        );


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