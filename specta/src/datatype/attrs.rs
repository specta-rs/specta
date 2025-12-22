// TODO: Rename and document this stuff

use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeAttribute {
    pub path: String,
    pub kind: RuntimeMeta,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuntimeMeta {
    Path,
    NameValue { key: String, value: RuntimeLiteral },
    List(Vec<RuntimeNestedMeta>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuntimeNestedMeta {
    Meta(RuntimeMeta),
    Literal(RuntimeLiteral),
}

#[derive(Debug, Clone)]
pub enum RuntimeLiteral {
    Str(String),
    Int(i64),
    Bool(bool),
    Float(f64),
}

// Manual implementation of PartialEq for RuntimeLiteral to handle f64
impl PartialEq for RuntimeLiteral {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RuntimeLiteral::Str(a), RuntimeLiteral::Str(b)) => a == b,
            (RuntimeLiteral::Int(a), RuntimeLiteral::Int(b)) => a == b,
            (RuntimeLiteral::Bool(a), RuntimeLiteral::Bool(b)) => a == b,
            (RuntimeLiteral::Float(a), RuntimeLiteral::Float(b)) => a.to_bits() == b.to_bits(),
            _ => false,
        }
    }
}

// Manual implementation of Eq for RuntimeLiteral
impl Eq for RuntimeLiteral {}

// Manual implementation of Hash for RuntimeLiteral to handle f64
impl Hash for RuntimeLiteral {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            RuntimeLiteral::Str(s) => {
                0u8.hash(state);
                s.hash(state);
            }
            RuntimeLiteral::Int(i) => {
                1u8.hash(state);
                i.hash(state);
            }
            RuntimeLiteral::Bool(b) => {
                2u8.hash(state);
                b.hash(state);
            }
            RuntimeLiteral::Float(f) => {
                3u8.hash(state);
                f.to_bits().hash(state);
            }
        }
    }
}
