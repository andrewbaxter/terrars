use crate::{
    expr::{
        Expr,
    },
    prim_ref::PrimExpr,
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
