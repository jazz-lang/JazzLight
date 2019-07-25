use crate::*;
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum Opcode {
    LoadInt(i64),
    LoadNil,
    LoadUndef,
    LoadTrue,
    LoadFalse,
    LoadConst(u32),
    LoadVar(Name),
    /// Load value from object or array
    Load,
    /// Store value into object or array
    Store,
    NewObj,
    ConstructArray(u32),
    DeclVar(Name),
    StoreVar(Name),
    /// Push catch block address to exception stack
    PushCatch(usize),
    /// Pop exception from exception stack
    PopCatch,
    /// Throw exception, if there are exception block jump to it otherwise print error and exit program
    Throw,
    /// Jump to instruction
    Jump(u32),
    /// Jump to instruction if value from stack != 0
    JumpIf(u32),
    /// Jump to instruction if value from stack == 0
    JumpIfFalse(u32),
    /// Invoke some function
    Call(u32),
    /// Pop n items from stack
    Pop(u32),
    /// Duplicate value from stack
    Dup,
    /// Pop environment
    PopEnv,
    /// Push empty environment
    PushEnv,
    /// Initialize function environment
    InitEnv,
    /// No opcode
    Label,
    /// Yield value from stack
    Yield,
    /// Return from function
    Return,

    Add,
    Sub,
    Div,
    Mul,
    Rem,
    Shr,
    Shl,
    Gt,
    Lt,
    Ge,
    Le,
    Eq,
    Ne,
    And,
    Or,
    BitXor,
    BitOr,
    BitAnd,
    Not,
    Neg,
    BlockEnd,
    BlockStart,
}
use crate::gc::{InGcEnv, Mark};
impl Mark for Opcode {
    fn mark(&self, _: &mut InGcEnv) {}
}
