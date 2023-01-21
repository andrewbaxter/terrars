use std::marker::PhantomData;
use crate::{
    StackShared,
    ref_::Ref,
    list_ref::ToListMappable,
    MapKV,
    ListRef,
};

// Implemented by things that can be mapped from a set data source
pub struct SetRef<T: Ref> {
    pub(crate) shared: StackShared,
    pub(crate) base: String,
    _pd: PhantomData<T>,
}

impl<T: Ref> Ref for SetRef<T> {
    fn new(shared: StackShared, base: String) -> Self {
        SetRef {
            shared: shared,
            base: base,
            _pd: Default::default(),
        }
    }
}

impl<T: Ref> SetRef<T> {
    pub fn map<O: ToListMappable>(&self, inner: impl FnOnce(MapKV<T>) -> O) -> O::O {
        let out = inner(MapKV::new(self.shared.clone()));
        out.do_map(self.base.clone())
    }

    pub fn as_list(self) -> ListRef<T> {
        ListRef::new(self.shared, format!("tolist({})", self.base))
    }
}
