use crate::{
    Stack,
    ToFuncArg,
    PrimExpr,
};

/// Generates a call to Terraform method `base64encode`.
pub fn tf_base64encode(stack: &Stack, e: impl ToFuncArg<String>) -> PrimExpr<String> {
    return stack.func("base64encode").a(e).into();
}

/// Generates a call to Terraform method `base64decode`.
pub fn tf_base64decode(stack: &Stack, e: impl ToFuncArg<String>) -> PrimExpr<String> {
    return stack.func("base64decode").a(e).into();
}

/// Generates a call to Terraform method `substr`.
pub fn tf_substr(stack: &Stack, e: impl ToFuncArg<String>, offset: usize, length: usize) -> PrimExpr<String> {
    return stack.func("substr").a(e).a(stack.expr_lit(offset as i64)).a(stack.expr_lit(length as i64)).into();
}

/// Generates a call to Terraform method `trimsuffix`.
pub fn tf_trim_suffix(
    stack: &Stack,
    original: impl ToFuncArg<String>,
    suffix: PrimExpr<String>,
) -> PrimExpr<String> {
    return stack.func("trimsuffix").a(original).a(suffix).into();
}

/// Generates a call to Terraform method `trimprefix`.
pub fn tf_trim_prefix(
    stack: &Stack,
    original: impl ToFuncArg<String>,
    prefix: PrimExpr<String>,
) -> PrimExpr<String> {
    return stack.func("trimprefix").a(original).a(prefix).into();
}
