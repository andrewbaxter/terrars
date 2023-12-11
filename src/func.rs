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

impl Func {
    /// Add an argument to the function call
    pub fn a<T: PrimType>(mut self, s: PrimExpr<T>) -> Self {
        if !self.first {
            self.data.push_str(", ");
        } else {
            self.first = false;
        }
        let (_, s) = s.expr_raw();
        self.data.push_str(&s);
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
