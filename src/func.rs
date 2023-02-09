use crate::{
    expr::{
        Expr,
    },
    prim_ref::PrimExpr,
    PrimType,
    StackShared,
    PrimField,
};

pub fn tf_base64encode(e: PrimExpr<String>) -> PrimExpr<String> {
    let (shared, raw) = e.expr_raw();
    PrimExpr(shared.clone(), format!("base64encode({})", raw), Default::default())
}

pub fn tf_base64decode(e: PrimExpr<String>) -> PrimExpr<String> {
    let (shared, raw) = e.expr_raw();
    PrimExpr(shared.clone(), format!("base64decode({})", raw), Default::default())
}

pub fn tf_substr(e: PrimExpr<String>, offset: usize, length: usize) -> PrimExpr<String> {
    let (shared, raw) = e.expr_raw();
    PrimExpr(shared.clone(), format!("substr({}, {}, {})", raw, offset, length), Default::default())
}

pub struct Func {
    pub(crate) shared: StackShared,
    pub(crate) data: String,
    pub(crate) first: bool,
}

pub trait ToFuncLit {
    fn to_func_lit(&self, out: &mut String);
}

impl ToFuncLit for String {
    fn to_func_lit(&self, out: &mut String) {
        self.as_str().to_func_lit(out)
    }
}

impl<'a> ToFuncLit for &'a str {
    fn to_func_lit(&self, out: &mut String) {
        out.push_str(&format!("\"{}\"", self.replace("\\", "\\\\").replace("\"", "\\\"")));
    }
}

impl ToFuncLit for i64 {
    fn to_func_lit(&self, out: &mut String) {
        out.push_str(&self.to_string());
    }
}

impl ToFuncLit for bool {
    fn to_func_lit(&self, out: &mut String) {
        if *self {
            out.push_str("true");
        } else {
            out.push_str("false");
        }
    }
}

impl Func {
    /// Add a literal argument to the function call
    pub fn l(mut self, s: impl ToFuncLit) -> Self {
        if !self.first {
            self.data.push_str(", ");
        } else {
            self.first = false;
        }
        s.to_func_lit(&mut self.data);
        self
    }

    /// Add an expression argument to the function call
    pub fn e<T: PrimType>(mut self, expr: &dyn Expr<T>) -> Self {
        if !self.first {
            self.data.push_str(", ");
        } else {
            self.first = false;
        }
        self.data.push_str(&expr.expr_raw().1);
        self
    }

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
