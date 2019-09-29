thread_local!();

use crate::*;
use bytecode::*;
use parking_lot::{Condvar, Mutex};
use pgc::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use value::*;

#[derive(GcObject)]
pub enum FrameData {
    ExitFrame,
    Frame {
        module: Option<Gc<Module>>,
        pc: usize,
        locals: Gc<HashMap<u32, Value>>,
        this: Value,
        env: Value,
    },
}

pub struct Threads {
    pub threads: Mutex<Vec<Arc<Gc<JThread>>>>,
    pub cond_join: Condvar,
}

#[derive(GcObject)]
pub struct JThread {
    pub pc: usize,
    pub stack: Gc<Vec<Value>>,
    pub locals: Gc<HashMap<u32, Value>>,
    pub env: Value,
    pub this: Value,
    pub frames: Vec<FrameData>,
    pub exceptions: Vec<FrameData>,
}

thread_local!(
    pub static THREAD: RefCell<Arc<Gc<JThread>>> =
        { RefCell::new(Arc::new(Gc::new(JThread::new()))) };
);

impl JThread {
    pub fn new() -> Self {
        Self {
            pc: 0,
            stack: Gc::new(vec![]),
            locals: Gc::new(HashMap::new()),
            env: Value::Null,
            this: Value::Null,
            frames: vec![],
            exceptions: vec![],
        }
    }

    pub fn exit_frame(&mut self) {
        self.frames.push(FrameData::ExitFrame);
    }

    pub fn push_frame(&mut self, m: Option<Gc<Module>>) {
        if self.frames.len() >= 999 {
            panic!("CallStack overflow");
        }
        self.frames.push(FrameData::Frame {
            pc: self.pc,
            module: m,
            env: self.env.clone(),
            this: self.this.clone(),
            locals: self.locals,
        })
    }
    pub fn pop_frame(&mut self, m: Option<&mut Gc<Module>>) -> bool {
        match self.frames.pop().unwrap() {
            FrameData::ExitFrame => true,
            FrameData::Frame {
                pc,
                env,
                this,
                module,
                locals,
            } => {
                self.pc = pc;
                self.env = env;
                self.this = this;
                if let Some(module) = module {
                    if let Some(m) = m {
                        *m = module;
                    }
                }
                self.locals = locals;
                false
            }
        }
    }

    pub fn pop(&mut self) -> Result<Value, Value> {
        match self.stack.get_mut().pop() {
            Some(val) => Ok(val),
            None => Err(Value::String(Gc::new("No value to pop".to_owned()))),
        }
    }
    pub fn push(&mut self, value: Value) {
        self.stack.get_mut().push(value);
    }
}

unsafe impl GcObject for Threads {
    fn references(&self) -> Vec<Gc<dyn GcObject>> {
        let lock = self.threads.lock();
        let mut v: Vec<Gc<dyn GcObject>> = vec![];
        for thread in lock.iter() {
            v.extend(thread.references());
        }
        v
    }
}

impl Threads {
    pub fn new() -> Threads {
        Threads {
            threads: Mutex::new(Vec::new()),
            cond_join: Condvar::new(),
        }
    }

    pub fn attach_current_thread(&self) {
        THREAD.with(|thread| {
            let mut threads = self.threads.lock();
            threads.push(thread.borrow().clone());
        });
    }

    pub fn attach_thread(&self, thread: Arc<Gc<JThread>>) {
        let mut threads = self.threads.lock();
        threads.push(thread);
    }

    pub fn detach_current_thread(&self) {
        THREAD.with(|thread| {
            let mut threads = self.threads.lock();
            threads.retain(|elem| !Arc::ptr_eq(elem, &*thread.borrow()));
            self.cond_join.notify_all();
        });
    }

    pub fn join_all(&self) {
        let mut threads = self.threads.lock();

        while threads.len() > 0 {
            self.cond_join.wait(&mut threads);
        }
    }

    pub fn each<F>(&self, mut f: F)
    where
        F: FnMut(&Arc<Gc<JThread>>),
    {
        let threads = self.threads.lock();

        for thread in threads.iter() {
            f(thread)
        }
    }
}
