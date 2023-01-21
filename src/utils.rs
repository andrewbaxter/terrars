use std::{
    cell::{
        RefCell,
    },
};
use serde::Serialize;

thread_local!{
    pub(crate) static REPLACE_EXPRS: RefCell<Option<Vec<(String, String)>>> = RefCell::new(None);
}

pub trait SerdeSkipDefault {
    fn is_default(&self) -> bool;
    fn is_not_default(&self) -> bool;
}

impl<T: Default + PartialEq> SerdeSkipDefault for T {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }

    fn is_not_default(&self) -> bool {
        !self.is_default()
    }
}

#[derive(Serialize)]
pub struct DynamicBlock<T: Serialize> {
    pub for_each: String,
    pub content: T,
}
