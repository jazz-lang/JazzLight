thread_local!();

use crate::*;
use parking_lot::{Condvar, Mutex};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use value::*;

pub enum FrameData {
    ExitFrame,
    Frame {
        module: Option<Gc<Module>>,
        pc: usize,
        locals: Gc<HashMap<u32, Value>>,
        this: Value,
        env: Value,
        ctor_call: bool,
    },
}

thread_local!(
    pub static THREAD: RefCell<Arc<Ptr<JThread>>> =
        { RefCell::new(Arc::new(Ptr::new(JThread::new()))) };
);

pub struct Threads {
    pub threads: Mutex<Vec<Arc<Ptr<JThread>>>>,
    pub cond_join: Condvar,
}

pub struct JThread {
    pub pc: usize,
    pub stack: Vec<Value>,
    pub locals: Gc<HashMap<u32, Value>>,
    pub env: Value,
    pub this: Value,
    pub frames: Vec<FrameData>,
    pub exceptions: Vec<FrameData>,
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

    pub fn attach_thread(&self, thread: Arc<Ptr<JThread>>) {
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
        F: FnMut(&Arc<Ptr<JThread>>),
    {
        let threads = self.threads.lock();

        for thread in threads.iter() {
            f(thread)
        }
    }
}

/*
unsafe impl GcObject for JThread {
    fn references(&self) -> Vec<Gc<dyn GcObject>> {
        let mut v: Vec<Gc<dyn GcObject>> = vec![];
        for value in self.stack.iter() {
            v.extend(value.references());
        }
        v.extend(self.this.references());
        v.extend(self.env.references());
        v.extend(self.frames.references());
        v.extend(self.exceptions.references());
        v.push(self.locals);
        for (_, val) in self.locals.iter() {
            println!("{}", val);
        }

        v
    }
}
*/

pub struct Ptr<T> {
    ptr: *mut T,
}

unsafe impl<T: Send> Send for Ptr<T> {}
unsafe impl<T: Sync> Sync for Ptr<T> {}

impl<T> Ptr<T> {
    pub fn new(val: T) -> Self {
        Self {
            ptr: Box::into_raw(Box::new(val)),
        }
    }

    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
    pub fn get(&self) -> &T {
        unsafe { &*self.ptr }
    }
}

impl JThread {
    pub fn new() -> Self {
        Self {
            pc: 0,
            stack: vec![],
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

    pub fn push_frame(&mut self, m: Option<Gc<Module>>, ctor: bool) {
        if self.frames.len() >= 999 {
            panic!("CallStack overflow");
        }
        self.frames.push(FrameData::Frame {
            pc: self.pc,
            module: m,
            env: self.env.clone(),
            this: self.this.clone(),
            locals: self.locals.clone(),
            ctor_call: ctor,
        })
    }
    pub fn pop_frame(&mut self, m: Option<&mut Gc<Module>>) -> (bool, bool) {
        match self.frames.pop().unwrap() {
            FrameData::ExitFrame => (true, false),
            FrameData::Frame {
                pc,
                env,
                this,
                module,
                locals,
                ctor_call,
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
                (false, ctor_call)
            }
        }
    }

    pub fn pop(&mut self) -> Result<Value, Value> {
        match self.stack.pop() {
            Some(val) => Ok(val),
            None => Err(Value::String(Gc::new(format!(
                "No value to pop at {:04}",
                self.pc - 1
            )))),
        }
    }
    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }
}

/*
unsafe impl GcObject for Threads {
    fn references(&self) -> Vec<Gc<dyn GcObject>> {
        let lock = self.threads.lock();
        let mut v: Vec<Gc<dyn GcObject>> = vec![];
        for thread in lock.iter() {
            v.extend(thread.references());
        }
        v
    }
}*/
