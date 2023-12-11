use crate::{
    expr::{
        Expr,
    },
    prim_ref::PrimExpr,
    PrimType,
    StackShared,
    PrimField,
};

pub struct Func {
    pub(crate) shared: StackShared,
    pub(crate) data: String,
    pub(crate) first: bool,
}

pub trait ToFuncArg {
    fn to_func_arg(&self, out: &mut String);
}

impl ToFuncArg for String {
    fn to_func_arg(&self, out: &mut String) {
        self.as_str().to_func_arg(out)
    }
}

impl<'a> ToFuncArg for &'a str {
    fn to_func_arg(&self, out: &mut String) {
        out.push_str(&format!("\"{}\"", self.replace("\\", "\\\\").replace("\"", "\\\"")));
    }
}

impl ToFuncArg for i64 {
    fn to_func_arg(&self, out: &mut String) {
        out.push_str(&self.to_string());
    }
}

impl ToFuncArg for usize {
    fn to_func_arg(&self, out: &mut String) {
        out.push_str(&self.to_string());
    }
}

impl ToFuncArg for bool {
    fn to_func_arg(&self, out: &mut String) {
        if *self {
            out.push_str("true");
        } else {
            out.push_str("false");
        }
    }
}

impl<T: PrimType> ToFuncArg for PrimExpr<T> {
    fn to_func_arg(&self, out: &mut String) {
        out.push_str(&self.expr_raw().1);
    }
}

impl Func {
    /// Add an argument to the function call
    pub fn a(mut self, s: impl ToFuncArg) -> Self {
        if !self.first {
            self.data.push_str(", ");
        } else {
            self.first = false;
        }
        s.to_func_arg(&mut self.data);
        self
    }

    /// Return an expression representing indexing the result of the function call
    pub fn index<T: PrimType>(&self, i: usize) -> PrimExpr<T> {
        PrimExpr(self.shared.clone(), format!("{})[{}]", self.data, i), std::marker::PhantomData::default())
    }
}

impl<T: PrimType> Expr<T> for Func {
    fn expr_raw(&self) -> (&crate::StackShared, String) {
        (&self.shared, format!("{})", self.data))
    }
}

impl<T: PrimType> Into<PrimField<T>> for Func {
    fn into(self) -> PrimField<T> {
        PrimField::Sentinel(<dyn Expr<T>>::expr_sentinel(&self))
    }
}

impl<T: PrimType> Into<PrimExpr<T>> for Func {
    fn into(self) -> PrimExpr<T> {
        let (s, raw) = <Func as Expr<T>>::expr_raw(&self);
        return PrimExpr(s.clone(), raw, std::marker::PhantomData::default());
    }
}
