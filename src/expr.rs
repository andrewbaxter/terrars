use crate::{
    PrimType,
    StackShared,
};

// Expressions
pub trait Expr<T: PrimType> {
    fn expr_raw(&self) -> (&StackShared, String);
    fn expr_sentinel(&self) -> String;
}

// More crazy rust limitation workarounds
#[macro_export]
macro_rules! manual_expr_impls{
    ($t: ident) => {
        impl < T: PrimType > Into < String > for $t < T > {
            fn into(self) -> String {
                self.expr_sentinel()
            }
        }
        impl < T: PrimType > Into < String > for & $t < T > {
            fn into(self) -> String {
                self.expr_sentinel()
            }
        }
        impl < T: PrimType > Display for $t < T > {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.expr_sentinel().fmt(f)
            }
        }
        impl < T: PrimType > Into < PrimField < T >> for $t < T > {
            fn into(self) -> PrimField<T> {
                PrimField::Sentinel(self.expr_sentinel())
            }
        }
        impl < T: PrimType > Into < PrimField < T >> for & $t < T > {
            fn into(self) -> PrimField<T> {
                PrimField::Sentinel(self.expr_sentinel())
            }
        }
        impl Into < PrimExpr < String >> for $t < bool > {
            fn into(self) -> PrimExpr<String> {
                let (shared, raw) = self.expr_raw();
                PrimExpr(shared.clone(), raw, Default::default())
            }
        }
        impl Into < PrimExpr < String >> for & $t < bool > {
            fn into(self) -> PrimExpr<String> {
                let (shared, raw) = self.expr_raw();
                PrimExpr(shared.clone(), raw, Default::default())
            }
        }
        impl Into < PrimExpr < String >> for $t < i64 > {
            fn into(self) -> PrimExpr<String> {
                let (shared, raw) = self.expr_raw();
                PrimExpr(shared.clone(), raw, Default::default())
            }
        }
        impl Into < PrimExpr < String >> for & $t < i64 > {
            fn into(self) -> PrimExpr<String> {
                let (shared, raw) = self.expr_raw();
                PrimExpr(shared.clone(), raw, Default::default())
            }
        }
        impl Into < PrimExpr < String >> for $t < f64 > {
            fn into(self) -> PrimExpr<String> {
                let (shared, raw) = self.expr_raw();
                PrimExpr(shared.clone(), raw, Default::default())
            }
        }
        impl Into < PrimExpr < String >> for & $t < f64 > {
            fn into(self) -> PrimExpr<String> {
                let (shared, raw) = self.expr_raw();
                PrimExpr(shared.clone(), raw, Default::default())
            }
        }
    };
}
