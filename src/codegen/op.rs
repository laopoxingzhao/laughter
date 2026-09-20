//! 栈机操作码。

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op {
    Const = 0,
    True,
    False,
    Pop,
    GetLocal,
    SetLocal,
    SetLocalField,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Neg,
    Not,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Jump,
    JumpIfFalse,
    JumpIfTrue,
    Loop,
    Call,
    CallMethod,
    Return,
    NewArray,
    GetIndex,
    SetIndex,
    NewStruct,
    GetField,
    SetField,
    Len,
    Print,
    Push,
    ArrayPop,
    Input,
    StrAt,
    StrSub,
    ToString,
}

impl Op {
    pub fn from_u8(b: u8) -> Option<Op> {
        // safety: contiguous enum starting at 0
        const MAX: u8 = Op::ToString as u8;
        if b <= MAX {
            Some(unsafe { std::mem::transmute(b) })
        } else {
            None
        }
    }

    /// 指令宽度（含操作数）
    pub fn width(self) -> usize {
        match self {
            Op::Const
            | Op::GetLocal
            | Op::SetLocal
            | Op::Jump
            | Op::JumpIfFalse
            | Op::JumpIfTrue
            | Op::Loop
            | Op::Call
            | Op::CallMethod
            | Op::NewArray
            | Op::GetField
            | Op::SetField => 3,
            Op::SetLocalField | Op::NewStruct => 5,
            _ => 1,
        }
    }
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
