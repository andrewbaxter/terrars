use std::marker::PhantomData;
use crate::{
    prim_ref::{
        PrimRef,
        PrimExpr,
    },
    StackShared,
    list_ref::{
        RecToListMappable,
    },
};

pub trait ToObjMappable {
    type O;

    fn do_map_obj(self, base: String, k: PrimExpr<String>) -> Self::O;
}

pub trait ListToRecMappable {
    type O;

    fn do_map_obj(self, base: String, k: PrimExpr<String>) -> Self::O;
}

pub struct RecRef<T: PrimRef> {
    pub(crate) shared: StackShared,
    pub(crate) base: String,
    _pd: PhantomData<T>,
}

impl<T: PrimRef> PrimRef for RecRef<T> {
    fn new(shared: StackShared, base: String) -> Self {
        RecRef {
            shared: shared,
            base: base,
            _pd: Default::default(),
        }
    }
}

impl<T: PrimRef> RecRef<T> {
    pub fn get(&self, key: impl ToString) -> T {
        T::new(self.shared.clone(), format!("{}[\"{}\"]", &self.base, key.to_string()))
    }

    pub fn map<O: RecToListMappable>(&self, inner: impl FnOnce(PrimExpr<String>, T) -> O) -> O::O {
        let out = inner(PrimExpr::new(self.shared.clone(), "k".into()), T::new(self.shared.clone(), "v".into()));
        out.do_map(self.base.clone())
    }

    pub fn map_obj<
        O: ToObjMappable,
    >(&self, inner: impl FnOnce(PrimExpr<String>, T) -> (PrimExpr<String>, O)) -> O::O {
        let (k, out) =
            inner(PrimExpr::new(self.shared.clone(), "k".into()), T::new(self.shared.clone(), "v".into()));
        out.do_map_obj(self.base.clone(), k)
    }
}

pub struct MapRecRef<T> {
    pub(crate) shared: StackShared,
    pub(crate) base: String,
    pub(crate) map_base_key: String,
    pub(crate) map_base: String,
    _pd: PhantomData<T>,
}

impl<T> MapRecRef<T> {
    pub(crate) fn new(shared: StackShared, base: String, map_base_key: String, map_base: String) -> Self {
        MapRecRef {
            shared: shared,
            base: base,
            map_base_key: map_base_key,
            map_base: map_base,
            _pd: Default::default(),
        }
    }
}

pub struct MapRecRefToList<T> {
    pub(crate) shared: StackShared,
    pub(crate) base: String,
    pub(crate) map_base: String,
    _pd: PhantomData<T>,
}

impl<T> MapRecRefToList<T> {
    pub(crate) fn new(shared: StackShared, base: String, map_base: String) -> Self {
        MapRecRefToList {
            shared: shared,
            base: base,
            map_base: map_base,
            _pd: Default::default(),
        }
    }
}
