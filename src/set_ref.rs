use std::marker::PhantomData;
use crate::{
    StackShared,
    prim_ref::PrimRef,
    list_ref::ToListMappable,
};

// Implemented by things that can be mapped from a set data source
pub struct SetRef<T: PrimRef> {
    pub(crate) shared: StackShared,
    pub(crate) base: String,
    _pd: PhantomData<T>,
}

impl<T: PrimRef> PrimRef for SetRef<T> {
    fn new(shared: StackShared, base: String) -> Self {
        SetRef {
            shared: shared,
            base: base,
            _pd: Default::default(),
        }
    }
}

impl<T: PrimRef> SetRef<T> {
    pub fn map<O: ToListMappable>(&self, inner: impl FnOnce(T) -> O) -> O::O {
        let out = inner(T::new(self.shared.clone(), "each.value".into()));
        out.do_map(self.base.clone())
    }
}
