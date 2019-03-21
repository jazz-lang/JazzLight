use hashbrown::HashMap;
use hashbrown::HashSet;
use jazzvm::instruction::*;
use jazzvm::value::*;
use jazzvm::vm::*;
macro_rules! gc {
    ($e:expr) => {
        GcValue::new($e)
    };
}

use crate::ast::*;

pub struct Compiler<'a>
{
    pub globals: HashMap<String, u32>,
    pub vm: &'a mut VirtualMachine,
    pub functions: HashSet<u32>,
    pub module: String,
}

impl<'a> Compiler<'a>
{
    pub fn new(vm: &'a mut VirtualMachine, module: String) -> Compiler<'a>
    {
        Compiler { globals: HashMap::new(),
                   vm: vm,
                   functions: HashSet::new(),
                   module: module }
    }

    fn get_definitions(&mut self) -> HashMap<String, u32>
    {
        self.globals.clone()
    }

    fn get_functions(&mut self) -> HashSet<u32>
    {
        self.functions.clone()
    }
    pub fn import(&mut self, fname: &str)
    {
        use crate::parser::Parser;
        use crate::reader::Reader;
        let reader = Reader::from_file(fname).expect("file not found");
        let mut ast = vec![];
        use std::path::Path;
        let mname = Path::new(fname).file_stem()
                                    .unwrap()
                                    .to_str()
                                    .unwrap()
                                    .to_owned();

        let mut parser = Parser::new(reader, &mut ast);
        parser.parse().unwrap();
        let mut compiler = Compiler::new(self.vm, mname.clone());
        compiler.compile_ast(ast);
        let defs = compiler.get_definitions();
        //let fns = compiler.get_functions();
        let mut object = Object::new(&mname);

        for (key, value) in defs.iter()
        {
            let v = self.vm.globals.get(value).unwrap();
            object.insert(gc!(Value::Str(key.to_string())), v.clone());
        }

        let idx = self.vm.new_global(GcValue::new(Value::Object(object)));
        self.globals.insert(mname, idx);
    }

    pub fn compile_ast(&mut self, ast: Vec<Box<Expr>>)
    {
        crate::runtime::builtins(self);

        for expr in ast.iter()
        {
            match &expr.expr
            {
                ExprKind::Import(fname) => self.import(fname),
                ExprKind::Include(fname) =>
                {
                    use crate::parser::Parser;
                    use crate::reader::Reader;
                    let reader = Reader::from_file(fname).unwrap();
                    let mut ast = vec![];

                    let mut parser = Parser::new(reader, &mut ast);

                    parser.parse().unwrap();
                    use std::path::Path;
                    let mname = Path::new(fname).file_stem()
                                                .unwrap()
                                                .to_str()
                                                .unwrap()
                                                .to_owned();

                    let mut compiler = Compiler::new(self.vm, mname);
                    compiler.compile_ast(ast);

                    let defs = compiler.get_definitions();
                    let fns = compiler.get_functions();

                    self.globals.extend(defs);
                    self.functions.extend(fns);
                }
                _ => (),
            }
        }
        for expr in ast.iter()
        {
            match &expr.expr
            {
                ExprKind::Import(_fname) => (),
                ExprKind::Include(_fname) => (),
                ExprKind::Function(name, args, body) =>
                {
                    let function = Function { name: name.to_owned(),
                                              var: FuncVar::Code(vec![], 0) };
                    let fid = self.vm.new_global(gc!(Value::Func(function)));
                    self.globals.insert(name.to_owned(), fid);
                    let mut fbuilder = FunctionBuilder::new(self, args.len());
                    for (idx, param) in args.iter().enumerate()
                    {
                        fbuilder.locals.insert(param.to_string(), idx as u16);
                    }
                    fbuilder.compile(body);
                    let max_locals = fbuilder.max_locals;
                    let code = fbuilder.finish();
                    if cfg!(debug_assertions)
                    {
                        println!("Disassemble of `{}` function", name);
                        println!(".max_locals {}", max_locals);
                        for (idx, op) in code.iter().enumerate()
                        {
                            println!("{:04} {:?}", idx, op);
                        }
                    }
                    let func = self.vm.globals.get(&fid).unwrap();
                    let val: &mut Value = &mut func.get_mut();
                    let function = match val
                    {
                        Value::Func(f) => f,
                        _ => unreachable!(),
                    };
                    function.var = FuncVar::Code(code, max_locals);

                    self.functions.insert(fid);
                }

                ExprKind::Class(name, block, _implements) =>
                {
                    let object = Object::new(name);
                    let oidx = if !self.globals.contains_key(name)
                    {
                        let oidx = self.vm.new_global(gc!(Value::Object(object)));
                        self.globals.insert(name.to_owned(), oidx);

                        oidx
                    }
                    else
                    {
                        let idx = *self.globals.get(name).unwrap();
                        let obj = self.vm.globals.get(&idx).unwrap();
                        let obj_ref: &mut Value = &mut obj.get_mut();
                        match obj_ref
                        {
                            Value::Object(obj) => *obj = object,
                            _ => unreachable!(),
                        }
                        idx
                    };

                    if let ExprKind::Block(exprs) = &block.expr
                    {
                        for expr in exprs.iter()
                        {
                            match &expr.expr
                            {
                                ExprKind::Function(fname, args, block) =>
                                {
                                    let mut function = Function { name: fname.to_owned(),
                                                                  var: FuncVar::Code(vec![], 0) };
                                    let mut fbuilder = FunctionBuilder::new(self, args.len());
                                    for (idx, param) in args.iter().enumerate()
                                    {
                                        fbuilder.locals.insert(param.to_string(), idx as u16);
                                    }
                                    fbuilder.compile(block);
                                    let max_locals = fbuilder.max_locals;
                                    let code = fbuilder.finish();
                                    if cfg!(debug_assertions)
                                    {
                                        println!("Disassemble of `{}::{}` function", name, fname);
                                        println!(".max_lcoals {}", max_locals);
                                        for (idx, op) in code.iter().enumerate()
                                        {
                                            println!("{:04} {:?}", idx, op);
                                        }
                                    }
                                    function.var = FuncVar::Code(code, max_locals);

                                    //let id = self.vm.new_global(gc!(Value::Func(function)));
                                    let object = self.vm.globals.get(&oidx).unwrap();

                                    let object: &mut Value = &mut object.get_mut();
                                    let object: &mut Object = match object
                                    {
                                        Value::Object(obj) => obj,
                                        _ => unreachable!(),
                                    };
                                    object.insert(gc!(Value::Str(fname.to_owned())),
                                                  gc!(Value::Func(function)));
                                }
                                _ => unimplemented!(),
                            }
                        }
                    }
                }
                _ => unimplemented!(),
            }
        }
    }
}

impl<'a> Compiler<'a> {}

#[derive(Clone, Debug)]
pub enum UOP
{
    Goto(String),
    GotoF(String),
    GotoT(String),
    Op(Instruction),
}

pub struct FunctionBuilder<'a, 'b: 'a>
{
    compiler: &'a mut Compiler<'b>,
    locals: HashMap<String, u16>,
    /// This field is required for `break` in `while` (Check `check_labels` documentation)
    ///
    end_labels: Vec<String>,
    /// This field is required for `continue` in `while`
    /// # Example
    /// ```
    /// while i != 100 { // push to check_labels location of this check
    ///     while i < 50 { // push to check_labels location of this check
    ///         i = i + 2        
    ///         continue // use last label from check_labels
    ///     } // pop check_labels
    ///     i = i + 1
    /// }
    /// ```
    check_labels: Vec<String>,
    ins: Vec<UOP>,
    labels: HashMap<String, Option<usize>>,
    max_locals: u16,
}

impl<'a, 'b: 'a> FunctionBuilder<'a, 'b>
{
    pub fn new(cmpl: &'a mut Compiler<'b>, argc: usize) -> FunctionBuilder<'a, 'b>
    {
        let mut max_locals = 0;
        for _ in 0..argc as u16
        {
            max_locals += 1;
        }
        FunctionBuilder { compiler: cmpl,
                          max_locals,
                          locals: HashMap::new(),
                          end_labels: vec![],
                          check_labels: vec![],
                          ins: vec![],
                          labels: HashMap::new() }
    }
    #[inline]
    fn new_varid(&mut self) -> u16
    {
        self.max_locals += 1;
        self.locals.len() as u16
    }
    fn emit(&mut self, op: Instruction)
    {
        self.ins.push(UOP::Op(op));
    }
    pub fn finish(&mut self) -> Vec<Instruction>
    {
        let ins =
            self.ins
                .clone()
                .iter()
                .map(|i| match &i
                {
                    &UOP::Goto(s) => Instruction::Br(self.labels.get(s).unwrap().unwrap() as u16),
                    &UOP::GotoF(s) => Instruction::Brz(self.labels.get(s).unwrap().unwrap() as u16),
                    &UOP::GotoT(s) =>
                    {
                        Instruction::Brnz(self.labels.get(s).unwrap().unwrap() as u16)
                    }
                    &UOP::Op(op) => op.clone(),
                })
                .collect::<Vec<Instruction>>();
        ins
    }
    pub fn new_empty_label(&mut self) -> String
    {
        let lab_name = self.labels.len().to_string();
        self.labels.insert(lab_name.clone(), None);
        lab_name
    }

    pub fn label_here(&mut self, label: &str)
    {
        *self.labels.get_mut(label).expect("lbl not exists") = Some(self.ins.len());
    }

    pub fn compile(&mut self, expr: &Box<Expr>)
    {
        match &expr.expr
        {
            ExprKind::ConstInt(i) => self.emit(Instruction::LdInt(*i)),
            ExprKind::ConstFloat(f) => self.emit(Instruction::LdFloat(f.to_bits())),
            ExprKind::ConstStr(s) => self.emit(Instruction::LdString(s.clone())),
            ExprKind::ConstBool(b) => self.emit(Instruction::LdBool(*b)),
            ExprKind::Var(_, name, init) =>
            {
                if !self.locals.contains_key(name)
                {
                    let id = self.new_varid();
                    if init.is_some()
                    {
                        let val = init.clone().unwrap();
                        self.compile(&val);
                        self.emit(Instruction::StLoc(id));
                    }
                    else
                    {
                        /* Do nothing */
                    }
                    self.locals.insert(name.to_string(), id);
                }
                else
                {
                    let id = *self.locals.get(name).unwrap();
                    if init.is_some()
                    {
                        let val = init.clone().unwrap();
                        self.compile(&val);
                        self.emit(Instruction::StLoc(id));
                    }
                    else
                    {
                        /* Do nothing */
                    }
                }

                
            }
            ExprKind::Array(values) =>
            {
                for val in values.iter().rev()
                {
                    self.compile(val);
                }
                self.emit(Instruction::MakeArray(values.len() as u16));
            }

            ExprKind::Ident(name) =>
            {
                if self.compiler.globals.contains_key(name)
                {
                    let id = self.compiler.globals.get(name).unwrap();
                    self.emit(Instruction::LdGlob(*id));
                }
                else
                {
                    if name == "_ARGS_"
                    {
                        self.emit(Instruction::LdArgs);
                        return;
                    }
                    let id = self.locals.get(name);
                    if id.is_none()
                    {
                        panic!("error at {}: Variable `{}` not defined", expr.pos, name);
                    };
                    let id = id.unwrap();
                    self.emit(Instruction::LdLoc(*id));
                }
            }
            ExprKind::This => self.emit(Instruction::LdThis),
            ExprKind::Access(val, field) =>
            {
                self.compile(&val);
                self.emit(Instruction::LdString(field.to_string()));
                self.emit(Instruction::LdFld);
            }
            
            ExprKind::Assign(var, to) =>
            {
                self.compile(&to);
                match &var.expr
                {
                    ExprKind::Ident(name) =>
                    {
                        if self.locals.contains_key(name)
                        {
                            let id = self.locals.get(name).unwrap();
                            self.emit(Instruction::StLoc(*id));
                        }
                        else
                        {
                            let id = self.compiler.globals.get(name).unwrap();
                            self.emit(Instruction::StGlob(*id));
                        }
                    }
                    ExprKind::Access(obj, field) =>
                    {
                        self.compile(&to);
                        self.emit(Instruction::LdString(field.to_string()));
                        self.compile(&obj);
                        self.emit(Instruction::StFld);
                    }
                    _ => unimplemented!(),
                }
            }
            ExprKind::Block(exprs) =>
            {
                for expr in exprs.iter()
                {
                    self.compile(expr);
                }
            }
            ExprKind::BinOp(lhs, op, rhs) =>
            {
                let op: &str = op;
                self.compile(&rhs);
                self.compile(&lhs);
                let ins = match op
                {
                    "+" => Instruction::Add,
                    "-" => Instruction::Sub,
                    "*" => Instruction::Mul,
                    "%" => Instruction::Rem,
                    "/" => Instruction::Div,
                    ">" => Instruction::Gt,
                    ">=" => Instruction::Gte,
                    "<" => Instruction::Lt,
                    "<=" => Instruction::Lte,
                    "==" => Instruction::Eq,
                    "!=" => Instruction::Neq,
                    _ => unimplemented!(),
                };
                self.emit(ins);
            }
            ExprKind::Unop(op, val) =>
            {
                self.compile(&val);
                let op: &str = op;
                match op
                {
                    "-" => self.emit(Instruction::Neg),
                    _ => unimplemented!(),
                }
            }
            ExprKind::If(cond, then, or) =>
            {
                let lbl_false = self.new_empty_label();

                self.compile(&cond);
                self.ins.push(UOP::GotoF(lbl_false.clone()));
                self.compile(&then);
                self.label_here(&lbl_false);
                if or.is_some()
                {
                    let or = or.clone().unwrap();
                    self.compile(&or);
                }
            }

            ExprKind::For(decl,cond,then,block) => {
                let compare = self.new_empty_label();
                let end = self.new_empty_label();
                self.end_labels.push(end.clone());
                
                self.check_labels.push(compare.clone());
                self.compile(&decl);
                self.label_here(&compare);
                self.compile(&cond);
                self.ins.push(UOP::GotoF(end.clone()));
                self.compile(&block);
                self.compile(&then);
                self.ins.push(UOP::Goto(compare));
                self.label_here(&end);

            }
            ExprKind::While(cond, repeat) =>
            {
                let compare = self.new_empty_label();
                let end = self.new_empty_label();
                self.label_here(&compare);
                self.end_labels.push(end.clone());
                self.check_labels.push(compare.clone());
                self.compile(&cond);
                self.ins.push(UOP::GotoF(end.clone()));
                self.compile(&repeat);
                self.ins.push(UOP::Goto(compare));
                self.label_here(&end);
            }
            ExprKind::Call(val, args) =>
            {
                for arg in args.iter().rev()
                {
                    self.compile(arg);
                }

                if let ExprKind::Access(obj, field) = &val.expr
                {
                    self.compile(&obj);
                    self.emit(Instruction::Dup);
                    self.emit(Instruction::LdString(field.to_string()));
                    self.emit(Instruction::LdFld);
                }
                else
                {
                    self.emit(Instruction::LdNull);
                    self.compile(val);
                }
                self.emit(Instruction::Invoke(args.len() as u16));
            }
            ExprKind::Return(exp) =>
            {
                if exp.is_some()
                {
                    let expr = exp.clone().unwrap();
                    self.compile(&expr);
                    self.emit(Instruction::Ret);
                }
                else
                {
                    self.emit(Instruction::LdNull);
                    self.emit(Instruction::Ret);
                }
            }
            ExprKind::New(init_class) =>
            {
                if let ExprKind::Call(to_call, args) = &init_class.expr
                {
                    for arg in args.iter().rev()
                    {
                        self.compile(arg);
                    }
                    self.compile(to_call);

                    self.emit(Instruction::New(args.len() as u16));
                }
                else
                {
                    panic!("Call expected");
                }
            }
            ExprKind::Nil => self.emit(Instruction::LdNull),
            _ => unimplemented!(),
        }
    }
}
