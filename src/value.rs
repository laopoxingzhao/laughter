//! 运行时值：VM 栈上的数据。
//!
//! 结构体为**值语义**：`Value::Struct(StructVal)` 直接内嵌，赋值/传参按字段拷贝。
//! 若字段是 `string` / 数组，则该字段仍是指针语义（浅拷贝共享句柄）。
//! 数组本身仍是 `Rc<RefCell<Vec<Value>>>` 句柄（引用语义）。

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

pub type ArrayHandle = Rc<RefCell<Vec<Value>>>;

#[derive(Debug, Clone)]
pub struct StructVal {
    pub name: String,
    pub fields: Vec<(String, Value)>,
}

impl StructVal {
    pub fn get(&self, field: &str) -> Option<&Value> {
        self.fields.iter().find(|(n, _)| n == field).map(|(_, v)| v)
    }

    pub fn set(&mut self, field: &str, v: Value) -> bool {
        if let Some(slot) = self.fields.iter_mut().find(|(n, _)| n == field) {
            slot.1 = v;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(Rc<str>),
    Array(ArrayHandle),
    /// 值语义结构体（非句柄）
    Struct(StructVal),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::Array(_) => "array",
            Value::Struct(_) => "struct",
        }
    }

    pub fn display(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(n) => {
                if n.fract() == 0.0 && n.is_finite() {
                    format!("{n:.1}")
                } else {
                    n.to_string()
                }
            }
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => s.to_string(),
            Value::Array(a) => {
                let items: Vec<String> = a.borrow().iter().map(|v| v.display()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Struct(s) => {
                let items: Vec<String> = s
                    .fields
                    .iter()
                    .map(|(n, v)| format!("{n}: {}", v.display()))
                    .collect();
                format!("{} {{ {} }}", s.name, items.join(", "))
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}
