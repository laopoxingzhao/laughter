//! 字节码 Chunk 与 Module。

use crate::codegen::op::Op;
use crate::runtime::value::Value;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
    pub lines: Vec<u32>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: vec![],
            constants: vec![],
            lines: vec![],
        }
    }

    pub fn emit(&mut self, op: Op, line: u32) {
        self.code.push(op as u8);
        self.lines.push(line);
    }

    pub fn emit_u16(&mut self, n: u16, line: u32) {
        self.code.extend_from_slice(&n.to_le_bytes());
        self.lines.push(line);
        self.lines.push(line);
    }

    pub fn add_const(&mut self, v: Value) -> Result<u16, String> {
        if let Some(i) = self.constants.iter().position(|c| const_eq(c, &v)) {
            return Ok(i as u16);
        }
        if self.constants.len() >= u16::MAX as usize {
            return Err("too many constants".into());
        }
        self.constants.push(v);
        Ok((self.constants.len() - 1) as u16)
    }

    pub fn emit_const(&mut self, v: Value, line: u32) -> Result<(), String> {
        let i = self.add_const(v)?;
        self.emit(Op::Const, line);
        self.emit_u16(i, line);
        Ok(())
    }

    pub fn emit_jump(&mut self, op: Op, line: u32) -> usize {
        self.emit(op, line);
        self.emit_u16(0xFFFF, line);
        self.code.len() - 2
    }

    pub fn patch_to_end(&mut self, at: usize) -> Result<(), String> {
        let dest = self.code.len();
        self.patch_to(at, dest)
    }

    pub fn patch_to(&mut self, at: usize, dest: usize) -> Result<(), String> {
        // operand sits at `at`; forward jump = dest - (at+2)
        if dest < at + 2 {
            return Err("backward patch via Jump not supported".into());
        }
        let j = dest - (at + 2);
        if j > u16::MAX as usize {
            return Err("jump too large".into());
        }
        let b = (j as u16).to_le_bytes();
        self.code[at] = b[0];
        self.code[at + 1] = b[1];
        Ok(())
    }

    pub fn emit_loop(&mut self, start: usize, line: u32) -> Result<(), String> {
        self.emit(Op::Loop, line);
        let j = self.code.len() - start + 2;
        if j > u16::MAX as usize {
            return Err("loop too large".into());
        }
        self.emit_u16(j as u16, line);
        Ok(())
    }

    pub fn disassemble(&self, name: &str) -> String {
        use crate::codegen::op::Op as O;
        let mut out = format!("== {name} ==\n");
        let mut i = 0;
        while i < self.code.len() {
            let line = self.lines.get(i).copied().unwrap_or(0);
            let Some(op) = O::from_u8(self.code[i]) else {
                out.push_str(&format!("{i:04} L{line} ??? {}\n", self.code[i]));
                i += 1;
                continue;
            };
            out.push_str(&format!("{i:04} L{line:<4} {op}"));
            match op {
                O::Const
                | O::GetLocal
                | O::SetLocal
                | O::Call
                | O::NewArray
                | O::GetField
                | O::SetField
                | O::Jump
                | O::JumpIfFalse
                | O::JumpIfTrue
                | O::Loop
                | O::CallMethod => {
                    let a = u16::from_le_bytes([self.code[i + 1], self.code[i + 2]]);
                    if op == O::Const {
                        let c = self
                            .constants
                            .get(a as usize)
                            .map(|v| v.display())
                            .unwrap_or_else(|| "?".into());
                        out.push_str(&format!(" {a} ({c})\n"));
                    } else if matches!(op, O::Jump | O::JumpIfFalse | O::JumpIfTrue | O::Loop) {
                        let t = if op == O::Loop {
                            i + 3 - a as usize
                        } else {
                            i + 3 + a as usize
                        };
                        out.push_str(&format!(" {a} -> {t}\n"));
                    } else {
                        out.push_str(&format!(" {a}\n"));
                    }
                    i += 3;
                    continue;
                }
                O::SetLocalField | O::NewStruct => {
                    let a = u16::from_le_bytes([self.code[i + 1], self.code[i + 2]]);
                    let b = u16::from_le_bytes([self.code[i + 3], self.code[i + 4]]);
                    out.push_str(&format!(" {a} {b}\n"));
                    i += 5;
                    continue;
                }
                _ => {
                    out.push('\n');
                    i += 1;
                }
            }
        }
        if !self.constants.is_empty() {
            out.push_str("constants:\n");
            for (i, c) in self.constants.iter().enumerate() {
                out.push_str(&format!("  [{i}] {}\n", c.display()));
            }
        }
        out
    }
}

fn const_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        _ => false,
    }
}

#[derive(Debug, Clone)]
pub struct StructType {
    pub name: String,
    pub fields: Vec<String>,
}

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
    pub functions: Vec<Function>,
    pub main_index: Option<usize>,
    pub toplevel_index: usize,
    pub struct_types: Vec<StructType>,
}
