//! 运行时值：VM 栈上的数据。
//! 数组与结构体用 `Rc<RefCell<...>>` 句柄共享（引用语义），便于教学实现且无需 GC。

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

pub type ArrayHandle = Rc<RefCell<Vec<Value>>>;
pub type StructHandle = Rc<RefCell<StructVal>>;

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
    Struct(StructHandle),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::Array(_) => "array",
            Value::Struct(s) => {
                // 返回静态名有困难，用占位
                let _ = s;
                "struct"
            }
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
                let s = s.borrow();
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
