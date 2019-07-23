pub mod value;
pub mod runtime;

pub struct Interpreter {
    pub env: Environment
}

use hashlink::LinkedHashMap;

impl Interpreter {
    pub fn new() -> Interpreter {
        Interpreter {
            env: new_object()
        }
    }

    pub fn push_env(&mut self) {
        let old_env = self.env.clone();
        self.env = new_ref(Object {
            proto: Some(old_env),
            table: LinkedHashMap::new()
        });
    }

    pub fn pop_env(&mut self) {
        if self.env.borrow().proto.is_none() {
            panic!("No env to pop");
        }
        let env = self.env.borrow();
        let proto = env.proto.as_ref().unwrap().clone();
        drop(env);
        self.env = proto.clone();
    }
}

use crate::ast::*;
use value::*;

impl Visitor<Interpreter> for Expr {
    type Output = Result<Value,ValueData>;
    fn visit(&self,interp: &mut Interpreter) -> Self::Output {
        let pos = self.pos.clone();
        match &self.decl {
            ExprDecl::Const(constant) => match constant {
                Constant::Ident(ref name) => get_variable(&interp.env,name,&pos),
                Constant::Null => Ok(new_ref(ValueData::Nil)),
                Constant::Str(s) => Ok(new_ref(ValueData::String(s.to_owned()))),
                Constant::Int(x) => Ok(new_ref(ValueData::Number(*x as f64))),
                Constant::Float(x) => Ok(new_ref(ValueData::Number(*x))),
                Constant::True => Ok(new_ref(ValueData::Bool(true))),
                Constant::False => Ok(new_ref(ValueData::Bool(false))),
                Constant::Array(values) => {
                    let mut array = vec![];
                    for val in values.iter() {
                        let val = val.visit(interp)?;
                        array.push(val);
                    }                   
                    Ok(
                        new_ref(
                            ValueData::Array(new_ref(array))
                        )
                    ) 
                } 
                Constant::This => get_variable(&interp.env, "this", &pos),
                _ => unreachable!()
            }
            ExprDecl::Assign(to,from) => {
                let val = from.visit(interp)?;
                match &to.decl {
                    ExprDecl::Field(obj,name) => {
                        let obj = obj.visit(interp)?;
                        obj.borrow_mut().set(name,val.borrow().clone());
                    }
                    ExprDecl::Const(Constant::Ident(name)) => {
                        
                        match set_variable_in_scope(&interp.env,name, val.clone(),&pos) {
                            Ok(_) => return Ok(val),
                            Err(e) => return Err(e)
                        }
                    }
                    ExprDecl::Array(array,idx) => {
                        let array = array.visit(interp)?;
                        let idx = idx.visit(interp)?;
                        array.borrow_mut().set(idx.borrow().clone(),val.borrow().clone());
                        return Ok(val);
                    }
                    _ => {
                        let to_val = to.visit(interp)?;
                        return Err(new_error(pos.line as _,None,&format!("Can not assign '{}' to '{}'",val.borrow(),to_val.borrow())));
                    }
                }

                Ok(val)
            }
            ExprDecl::Field(val,key) => {
                let val = val.visit(interp)?;
                return Ok(val.borrow().get(&key.into()));
            }
            ExprDecl::Block(exprs) => {
                let mut result = new_ref(ValueData::Nil);
                //interp.push_env();
                for x in exprs.iter() {
                    result = x.visit(interp)?;
                }
                //interp.pop_env();
                return Ok(result)
            }
            ExprDecl::While(cond,body) => {
                let mut result = new_ref(ValueData::Nil);
                interp.push_env();
                'l : loop {
                    let cond = cond.visit(interp)?;
                    let cond_bool = bool::from(cond.borrow().clone());
                    if !cond_bool {
                        break;
                    }

                    match &body.decl {
                        ExprDecl::Block(exprs) => {
                            for expr in exprs.iter() {
                                result = expr.visit(interp)?;
                                match &expr.decl {
                                    ExprDecl::Return(_) => break 'l,
                                    ExprDecl::Break(_) => break 'l,
                                    ExprDecl::Continue => continue 'l,
                                    _ => ()
                                }
                            }
                        }
                        _ => unreachable!()
                    }
                }
                interp.pop_env();
                Ok(result)
            }
            ExprDecl::Continue => return  Ok(new_ref(ValueData::Nil)),
            ExprDecl::Break(val) => match val {
                Some(val) => return val.visit(interp),
                None => return  Ok(new_ref(ValueData::Nil))
            },
            ExprDecl::Return(expr) => {
                match expr {
                    Some(val) => return val.visit(interp),
                    None => return Ok(new_ref(ValueData::Nil))
                }
            }
            ExprDecl::Var(_,name,init) => {
                let val = match init {
                    Some(val) => val.visit(interp)?,
                    None => return Ok(new_ref(ValueData::Nil))
                };
                if !var_declared(&interp.env,name) {
                    match declare_var(&interp.env,name,val,&pos) {
                        Ok(()) => return Ok(new_ref(ValueData::Nil)),
                        Err(e) => return Err(e) 
                    }
                } else {
                    match set_variable_in_scope(&interp.env,name,val,&pos) {
                        Ok(()) => return Ok(new_ref(ValueData::Nil)),
                        Err(e) => return Err(e)
                    }
                }
            }
            ExprDecl::Try(expr,name,body) => {
                match expr.visit(interp) {
                    Ok(val) => return Ok(val),
                    Err(e) => {
                        interp.push_env();
                        declare_var(&interp.env,name,new_ref(e),&pos)?;
                        let result = body.visit(interp)?;
                        interp.pop_env();
                        return Ok(result);
                    }
                }
            }
            ExprDecl::Throw(val) => {
                return Err(val.visit(interp)?.borrow().clone());
            }
            ExprDecl::Function(args,body) => {
                let new_env = new_object();
                new_env.borrow_mut().proto = Some(interp.env.clone());
                let func = Function::Regular {
                    environment:  new_env,
                    args: args.clone(),
                    body: body.clone(),
                    args_set: std::cell::Cell::new(false)
                };
                return Ok(new_ref(
                    ValueData::Function(new_ref(func))
                ))
            }
            ExprDecl::Call(val,args) => {

                let mut args_ = vec![];
                for arg in args.iter() {
                    args_.push(arg.visit(interp)?);
                }

                let (this,fun) = match &val.decl {
                    ExprDecl::Field(obj,field) => {
                        let this = obj.visit(interp)?;
                        let fun = this.borrow().get(&ValueData::String(field.to_owned()));
                        (this,fun)
                    }
                    _ => (
                        new_ref(
                            ValueData::Object(
                                new_object()
                            )
                        ),
                        val.visit(interp)?
                    )
                };
                let fun: &ValueData = &fun.borrow();
                match fun {
                    ValueData::Function(func) => {
                        let func: &Function = &func.borrow();
                        match func {
                            Function::Native(ptr) => {
                                let fun: fn(Value,&[Value]) -> Result<Value,ValueData> = unsafe {std::mem::transmute(*ptr)};

                                return fun(this,&args_);
                            }
                            Function::Regular {
                                environment,
                                body,
                                args,
                                args_set    
                            } => {
                                let old_env = interp.env.clone();
                                interp.env = environment.clone();
                                for (i,arg_name) in args.iter().enumerate() {
                                    if !var_declared(&interp.env,arg_name) {
                                        declare_var(&interp.env,arg_name,args_.get(i).unwrap_or(&new_ref(ValueData::Undefined)).clone(),&pos)?;
                                    } else {
                                        set_variable_in_scope(&interp.env, arg_name ,args_.get(i).unwrap_or(&new_ref(ValueData::Undefined)).clone(),&pos)?;
                                    }
                                }
                                declare_var(&interp.env,"this",this,&pos); //ignore error;
                                let result = body.visit(interp)?;
                                interp.env = old_env;
                                return Ok(result);
                            }
                        }
                    }
                    _ => ()
                }

                Err(
                    new_error(
                        pos.line as _, None, "not a function")
                )

            }
            ExprDecl::Binop(op,lhs,rhs) => {
                let op: &str = op;
                match op {
                    _ => unimplemented!()
                }
            }
            _ => unimplemented!()
        }
    }
}
