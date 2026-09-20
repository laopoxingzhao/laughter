//! 字节码：操作码、`Chunk`（指令流 + 常量池 + 行号表）与编译后函数/模块。
//! 指令是栈机风格：操作数与结果都经操作数栈；跳转用相对偏移，便于 `patch_jump`。

use std::fmt;

use crate::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Const,
    GetLocal,
    SetLocal,
    GetGlobal,
    SetGlobal,
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
    /// pop condition, jump if true (for `||` short-circuit keeps true)
    JumpIfTrue,
    Loop,
    Call,
    Return,
    NewArray,
    GetIndex,
    SetIndex,
    Len,
    Print,
    Pop,
    True,
    False,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Op::Const => "CONST",
            Op::GetLocal => "GET_LOCAL",
            Op::SetLocal => "SET_LOCAL",
            Op::GetGlobal => "GET_GLOBAL",
            Op::SetGlobal => "SET_GLOBAL",
            Op::Add => "ADD",
            Op::Sub => "SUB",
            Op::Mul => "MUL",
            Op::Div => "DIV",
            Op::Rem => "REM",
            Op::Neg => "NEG",
            Op::Not => "NOT",
            Op::Eq => "EQ",
            Op::Ne => "NE",
            Op::Lt => "LT",
            Op::Le => "LE",
            Op::Gt => "GT",
            Op::Ge => "GE",
            Op::Jump => "JUMP",
            Op::JumpIfFalse => "JUMP_IF_FALSE",
            Op::JumpIfTrue => "JUMP_IF_TRUE",
            Op::Loop => "LOOP",
            Op::Call => "CALL",
            Op::Return => "RETURN",
            Op::NewArray => "NEW_ARRAY",
            Op::GetIndex => "GET_INDEX",
            Op::SetIndex => "SET_INDEX",
            Op::Len => "LEN",
            Op::Print => "PRINT",
            Op::Pop => "POP",
            Op::True => "TRUE",
            Op::False => "FALSE",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
    pub lines: Vec<u32>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn emit_op(&mut self, op: Op, line: u32) {
        self.code.push(op as u8);
        self.lines.push(line);
    }

    pub fn emit_u16(&mut self, n: u16, line: u32) {
        self.code.extend_from_slice(&n.to_le_bytes());
        self.lines.push(line);
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, v: Value) -> Result<u16, String> {
        if let Some(idx) = self.constants.iter().position(|c| value_eq_const(c, &v)) {
            return Ok(idx as u16);
        }
        if self.constants.len() >= u16::MAX as usize {
            return Err("too many constants".into());
        }
        self.constants.push(v);
        Ok((self.constants.len() - 1) as u16)
    }

    pub fn emit_const(&mut self, v: Value, line: u32) -> Result<(), String> {
        let idx = self.add_constant(v)?;
        self.emit_op(Op::Const, line);
        self.emit_u16(idx, line);
        Ok(())
    }

    /// 占位跳转偏移，返回回填位置；编译完后用 `patch_jump` 写入真实距离。
    pub fn emit_jump(&mut self, op: Op, line: u32) -> usize {
        self.emit_op(op, line);
        self.emit_u16(0xFFFF, line);
        self.code.len() - 2
    }

    pub fn patch_jump(&mut self, offset: usize) -> Result<(), String> {
        let jump = self.code.len() - offset - 2;
        if jump > u16::MAX as usize {
            return Err("jump too large".into());
        }
        let bytes = (jump as u16).to_le_bytes();
        self.code[offset] = bytes[0];
        self.code[offset + 1] = bytes[1];
        Ok(())
    }

    pub fn emit_loop(&mut self, start: usize, line: u32) -> Result<(), String> {
        self.emit_op(Op::Loop, line);
        let jump = self.code.len() - start + 2;
        if jump > u16::MAX as usize {
            return Err("loop body too large".into());
        }
        self.emit_u16(jump as u16, line);
        Ok(())
    }

    pub fn disassemble(&self, name: &str) -> String {
        let mut out = format!("== {name} ==\n");
        let mut i = 0usize;
        while i < self.code.len() {
            let line = self.lines.get(i).copied().unwrap_or(0);
            out.push_str(&format!("{i:04} L{line:<4} "));
            let op = self.code[i];
            let Some(op) = op_from_u8(op) else {
                out.push_str(&format!("UNKNOWN {op}\n"));
                i += 1;
                continue;
            };
            match op {
                Op::Const
                | Op::GetLocal
                | Op::SetLocal
                | Op::GetGlobal
                | Op::SetGlobal
                | Op::Call
                | Op::NewArray => {
                    if i + 2 < self.code.len() {
                        let arg = u16::from_le_bytes([self.code[i + 1], self.code[i + 2]]);
                        if op == Op::Const {
                            let c = self
                                .constants
                                .get(arg as usize)
                                .map(|v| v.display())
                                .unwrap_or_else(|| "?".into());
                            out.push_str(&format!("{op} {arg} ({c})\n"));
                        } else {
                            out.push_str(&format!("{op} {arg}\n"));
                        }
                        i += 3;
                        continue;
                    }
                }
                Op::Jump | Op::JumpIfFalse | Op::JumpIfTrue | Op::Loop => {
                    if i + 2 < self.code.len() {
                        let arg = u16::from_le_bytes([self.code[i + 1], self.code[i + 2]]);
                        let target = if op == Op::Loop {
                            i + 3 - arg as usize
                        } else {
                            i + 3 + arg as usize
                        };
                        out.push_str(&format!("{op} {arg} -> {target}\n"));
                        i += 3;
                        continue;
                    }
                }
                _ => {}
            }
            out.push_str(&format!("{op}\n"));
            i += 1;
        }
        out
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

pub fn value_eq_const(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        _ => false,
    }
}

pub fn op_from_u8(b: u8) -> Option<Op> {
    Some(match b {
        x if x == Op::Const as u8 => Op::Const,
        x if x == Op::GetLocal as u8 => Op::GetLocal,
        x if x == Op::SetLocal as u8 => Op::SetLocal,
        x if x == Op::GetGlobal as u8 => Op::GetGlobal,
        x if x == Op::SetGlobal as u8 => Op::SetGlobal,
        x if x == Op::Add as u8 => Op::Add,
        x if x == Op::Sub as u8 => Op::Sub,
        x if x == Op::Mul as u8 => Op::Mul,
        x if x == Op::Div as u8 => Op::Div,
        x if x == Op::Rem as u8 => Op::Rem,
        x if x == Op::Neg as u8 => Op::Neg,
        x if x == Op::Not as u8 => Op::Not,
        x if x == Op::Eq as u8 => Op::Eq,
        x if x == Op::Ne as u8 => Op::Ne,
        x if x == Op::Lt as u8 => Op::Lt,
        x if x == Op::Le as u8 => Op::Le,
        x if x == Op::Gt as u8 => Op::Gt,
        x if x == Op::Ge as u8 => Op::Ge,
        x if x == Op::Jump as u8 => Op::Jump,
        x if x == Op::JumpIfFalse as u8 => Op::JumpIfFalse,
        x if x == Op::JumpIfTrue as u8 => Op::JumpIfTrue,
        x if x == Op::Loop as u8 => Op::Loop,
        x if x == Op::Call as u8 => Op::Call,
        x if x == Op::Return as u8 => Op::Return,
        x if x == Op::NewArray as u8 => Op::NewArray,
        x if x == Op::GetIndex as u8 => Op::GetIndex,
        x if x == Op::SetIndex as u8 => Op::SetIndex,
        x if x == Op::Len as u8 => Op::Len,
        x if x == Op::Print as u8 => Op::Print,
        x if x == Op::Pop as u8 => Op::Pop,
        x if x == Op::True as u8 => Op::True,
        x if x == Op::False as u8 => Op::False,
        _ => return None,
    })
}

/// 编译后的函数对象；`is_void` 决定 VM 在 `Return` 时是否压返回值。
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub arity: u8,
    pub locals: u16,
    pub is_void: bool,
    pub chunk: Chunk,
}

#[derive(Debug, Clone)]
pub struct Module {
    /// 程序中所有函数；`$toplevel` 为顶层语句合成函数
    pub functions: Vec<Function>,
    /// `main` 在 `functions` 中的下标（若有则 run 从它进入）
    pub main_index: Option<usize>,
    pub toplevel_index: usize,
    /// MVP 未使用全局名表（顶层变量落在 `$toplevel` 局部槽）
    pub globals: Vec<String>,
}
