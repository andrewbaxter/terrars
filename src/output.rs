use std::{
    cell::{
        RefCell,
    },
    rc::Rc,
};
use serde::{
    Serialize,
};
use serde_json::{
    Value,
};
use crate::{
    PrimType,
    PrimField,
    Stack,
    utils::SerdeSkipDefault,
};

pub(crate) trait Output {
    fn extract_tf_id(&self) -> String;
    fn extract_value(&self) -> Value;
}

#[derive(Serialize)]
struct OutputImplData<T: PrimType> {
    #[serde(skip_serializing_if = "SerdeSkipDefault::is_default")]
    pub sensitive: PrimField<bool>,
    #[serde(skip_serializing_if = "SerdeSkipDefault::is_default")]
    pub value: PrimField<T>,
}

pub struct OutputImpl<T: PrimType> {
    tf_id: String,
    data: RefCell<OutputImplData<T>>,
}

impl<T: PrimType> OutputImpl<T> {
    pub fn set_sensitive(&self, v: impl Into<PrimField<bool>>) -> &Self {
        self.data.borrow_mut().sensitive = v.into();
        self
    }
}

impl<T: PrimType> Output for OutputImpl<T> {
    fn extract_tf_id(&self) -> String {
        self.tf_id.clone()
    }

    fn extract_value(&self) -> Value {
        let data = self.data.borrow();
        serde_json::to_value(&*data).unwrap()
    }
}

/// Create a new output.
pub struct BuildOutput<T: PrimType> {
    pub tf_id: String,
    pub value: PrimField<T>,
}

impl<T: PrimType + 'static> BuildOutput<T> {
    pub fn build(self, stack: &mut Stack) -> Rc<OutputImpl<T>> {
        let out = Rc::new(OutputImpl {
            tf_id: self.tf_id,
            data: RefCell::new(OutputImplData {
                sensitive: false.into(),
                value: self.value,
            }),
        });
        stack.outputs.push(out.clone());
        out
    }
}
