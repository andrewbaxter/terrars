use std::{
    cell::{
        RefCell,
    },
};

thread_local!{
    pub(crate) static REPLACE_EXPRS: RefCell<Option<Vec<(String, String)>>> = RefCell::new(None);
}
