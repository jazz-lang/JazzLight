use crate::*;

#[derive(Clone,Debug)]
pub enum Op {
    LoadNull,
    LoadTrue,
    LoadFalse,
    LoadInt(i64),
    LoadGlobal(u32),
    LoadEnv(u16),
    LoadLocal(u16),
    LoadBuiltin(String),
    LoadThis,
    Load,
    Store,
    StoreEnv(u16),
    StoreLocal(u16),
    StoreThis,
    Pop(u16),
    Call(u16),
    ObjCall(u16),
    TailCall(u16),
    Jump(u32),
    JumpIf(u32),
    JumpIfNot(u32),
    /// Push catch block address
    CatchPush(u32),
    Throw,
    Ret,
    MakeEnv(u16),
    MakeArray(u16),
    IsNull,
    IsNotNull,
    Add,
    Sub,
    Div,
    Mul,
    Mod,
    Shl,
    Shr,
    UShr,
    Or,
    And,
    Xor,
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Not,
    Neg,
    Hash,
    New,
    Nop,

    Last,
}
