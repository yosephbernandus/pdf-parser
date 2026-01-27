use std::collections::HashMap;

/// Reference to an indirect object (e.g., "5 0 R")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjRef {
    pub obj_num: u32,
    pub gen_num: u16,
}

impl ObjRef {
    pub fn new(obj_num: u32, gen_num: u16) -> Self {
        Self { obj_num, gen_num }
    }
}

/// All possible PDF object types
#[derive(Debug, Clone, PartialEq)]
pub enum PdfObject {
    Null,
    Bool(bool),
    Int(i64),
    Real(f64),
    String(Vec<u8>),
    Name(String),
    Array(Vec<PdfObject>),
    Dict(HashMap<String, PdfObject>),
    Stream {
        dict: HashMap<String, PdfObject>,
        data: Vec<u8>,
    },
    Ref(ObjRef),
}

// Helper methods for convenient access
impl PdfObject {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PdfObject::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            PdfObject::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_real(&self) -> Option<f64> {
        match self {
            PdfObject::Real(f) => Some(*f),
            PdfObject::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&[u8]> {
        match self {
            PdfObject::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&str> {
        match self {
            PdfObject::Name(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<PdfObject>> {
        match self {
            PdfObject::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&HashMap<String, PdfObject>> {
        match self {
            PdfObject::Dict(d) => Some(d),
            PdfObject::Stream { dict, .. } => Some(dict),
            _ => None,
        }
    }

    pub fn as_stream(&self) -> Option<(&HashMap<String, PdfObject>, &[u8])> {
        match self {
            PdfObject::Stream { dict, data } => Some((dict, data)),
            _ => None,
        }
    }

    pub fn as_ref(&self) -> Option<ObjRef> {
        match self {
            PdfObject::Ref(r) => Some(*r),
            _ => None,
        }
    }
}
