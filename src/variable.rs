use std::{
    cell::{
        RefCell,
    },
    fmt::Display,
    marker::PhantomData,
    rc::Rc,
};
use serde::{
    Serialize,
};
use serde_json::{
    Value,
};
use crate::{
    PrimField,
    PrimType,
    prim_ref::PrimExpr,
    StackShared,
    expr::{
        Expr,
    },
    manual_expr_impls,
    utils::SerdeSkipDefault,
    Stack,
};

pub(crate) trait VariableTrait {
    fn extract_tf_id(&self) -> String;
    fn extract_value(&self) -> Value;
}

#[derive(Serialize)]
struct VariableImplData {
    #[serde(skip_serializing_if = "SerdeSkipDefault::is_default")]
    pub r#type: String,
    #[serde(skip_serializing_if = "SerdeSkipDefault::is_not_default")]
    pub nullable: PrimField<bool>,
    #[serde(skip_serializing_if = "SerdeSkipDefault::is_default")]
    pub sensitive: PrimField<bool>,
}

struct Variable_<T: PrimType> {
    shared: StackShared,
    tf_id: String,
    data: RefCell<VariableImplData>,
    _p: PhantomData<T>,
}

pub struct Variable<T: PrimType>(Rc<Variable_<T>>);

impl<T: PrimType> VariableTrait for Variable_<T> {
    fn extract_tf_id(&self) -> String {
        self.tf_id.clone()
    }

    fn extract_value(&self) -> Value {
        let data = self.data.borrow();
        serde_json::to_value(&*data).unwrap()
    }
}

impl<T: PrimType> Variable<T> {
    pub fn set_nullable(self, v: impl Into<PrimField<bool>>) -> Self {
        self.0.data.borrow_mut().nullable = v.into();
        self
    }

    pub fn set_sensitive(self, v: impl Into<PrimField<bool>>) -> Self {
        self.0.data.borrow_mut().sensitive = v.into();
        self
    }
}

impl<T: PrimType> Expr<T> for Variable<T> {
    fn expr_raw(&self) -> (&StackShared, String) {
        (&self.0.shared, format!("var.{}", self.0.tf_id))
    }

    fn expr_sentinel(&self) -> String {
        let (shared, raw) = self.expr_raw();
        shared.add_sentinel(&raw)
    }
}

manual_expr_impls!(Variable);

pub struct BuildVariable {
    pub tf_id: String,
}

impl BuildVariable {
    pub fn build<T: PrimType + 'static>(self, stack: &mut Stack) -> Variable<T> {
        let out = Variable(Rc::new(Variable_ {
            shared: stack.shared.clone(),
            tf_id: self.tf_id,
            data: RefCell::new(VariableImplData {
                r#type: T::extract_variable_type(),
                nullable: false.into(),
                sensitive: false.into(),
            }),
            _p: Default::default(),
        }));
        stack.variables.push(out.0.clone());
        out
    }
}
