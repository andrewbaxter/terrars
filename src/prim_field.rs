use std::{
    fmt::Display,
    hash::Hash,
};
use serde::{
    Serialize,
};
use crate::utils::REPLACE_EXPRS;

pub trait TfPrimitiveType {
    fn extract_variable_type() -> String;
    fn to_expr_raw(&self) -> String;
    fn serialize2<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer;
}

impl TfPrimitiveType for String {
    fn extract_variable_type() -> String {
        "string".into()
    }

    fn to_expr_raw(&self) -> String {
        return format!("\"{}\"", self.replace("\\", "\\\\").replace("\"", "\\\""));
    }

    fn serialize2<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        REPLACE_EXPRS.with(|f| {
            if let Some(vs) = f.borrow().as_ref() {
                let mut out = self.replace("%{", "%%{").replace("${", "$${");
                for (k, v) in vs {
                    out = out.replace(k, v);
                }
                out.serialize(serializer)
            } else {
                self.serialize(serializer)
            }
        })
    }
}

impl TfPrimitiveType for bool {
    fn extract_variable_type() -> String {
        "bool".into()
    }

    fn to_expr_raw(&self) -> String {
        if *self {
            return "true".to_string();
        } else {
            return "false".to_string();
        }
    }

    fn serialize2<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        self.serialize(serializer)
    }
}

impl TfPrimitiveType for i64 {
    fn extract_variable_type() -> String {
        "int".into()
    }

    fn to_expr_raw(&self) -> String {
        return self.to_string();
    }

    fn serialize2<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        self.serialize(serializer)
    }
}

impl TfPrimitiveType for f64 {
    fn extract_variable_type() -> String {
        "float".into()
    }

    fn to_expr_raw(&self) -> String {
        return self.to_string();
    }

    fn serialize2<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        self.serialize(serializer)
    }
}

/// Helper trait representing core interchange values: `f64`, `i64`, `String`,
/// `bool`.
pub trait PrimType: Serialize + Clone + TfPrimitiveType + Default + PartialEq { }

impl<T: Serialize + Clone + TfPrimitiveType + Default + PartialEq> PrimType for T { }

/// In Terraform, all fields, regardless of whether a it's an int or bool or
/// whatever, can also take references like `${}`. `Primitive` represents this sort
/// of value. Base types `i64` `f64` `String` and `bool` are supported, and you
/// should be able to convert to `Primitive` with `into()`. Resource methods will
/// return typed references that can also be used here.
#[derive(Clone)]
pub enum PrimField<T: PrimType> {
    Literal(T),
    Sentinel(String),
}

impl<T: PrimType> Default for PrimField<T> {
    fn default() -> Self {
        PrimField::Literal(T::default())
    }
}

impl<T: PrimType + Hash> Hash for PrimField<T> {
    // Conditional derive somehow?
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl<T: PrimType + PartialEq> std::cmp::PartialEq for PrimField<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Literal(l0), Self::Literal(r0)) => l0 == r0,
            (Self::Sentinel(l0), Self::Sentinel(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl<T: PrimType + std::cmp::Eq + PartialEq> std::cmp::Eq for PrimField<T> { }

impl<T: PrimType> Serialize for PrimField<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        match self {
            PrimField::Literal(l) => l.serialize2(serializer),
            PrimField::Sentinel(r) => r.serialize2(serializer),
        }
    }
}

impl<T: PrimType> From<&T> for PrimField<T> {
    fn from(v: &T) -> Self {
        PrimField::Literal(v.clone())
    }
}

impl<T: PrimType> From<T> for PrimField<T> {
    fn from(v: T) -> Self {
        PrimField::Literal(v)
    }
}

impl From<&str> for PrimField<String> {
    fn from(v: &str) -> Self {
        PrimField::Literal(v.to_string())
    }
}

impl<T: PrimType + Display> Display for PrimField<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimField::Literal(v) => v.fmt(f),
            PrimField::Sentinel(v) => v.fmt(f),
        }
    }
}

#[macro_export]
macro_rules! primvec{
    [$($e: expr), *] => {
        vec![$(terrars:: PrimField:: from($e)), *]
    };
}

#[macro_export]
macro_rules! primmap{
    {
        $($k: tt = $e: expr),
        *
    }
    => {
        {
            let mut out = std::collections::HashMap::new();
            $(out.insert($k.to_string(), $e.into());) * out
        }
    };
}
