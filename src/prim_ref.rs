use std::marker::PhantomData;
use crate::{
    StackShared,
    PrimType,
    Expr,
    manual_expr_impls,
    prim_field::PrimField,
    list_ref::{
        MapListRef,
        MapListRefToRec,
        ToListMappable,
        RecToListMappable,
    },
    rec_ref::{
        MapRecRef,
        MapRecRefToList,
        ListToRecMappable,
        ToObjMappable,
    },
    Ref,
};
use std::fmt::Display;

pub struct PrimExpr<T: PrimType>(pub(crate) StackShared, pub(crate) String, pub(crate) PhantomData<T>);

impl<T: PrimType> Expr<T> for PrimExpr<T> {
    fn expr_raw(&self) -> (&StackShared, String) {
        (&self.0, self.1.clone())
    }

    fn expr_sentinel(&self) -> String {
        self.0.add_sentinel(&self.1)
    }
}

impl<T: PrimType> PrimExpr<T> {
    pub fn raw(&self) -> String {
        self.1.clone()
    }
}

manual_expr_impls!(PrimExpr);

// References
impl<T: PrimType> Ref for PrimExpr<T> {
    fn new(shared: StackShared, base: String) -> PrimExpr<T> {
        PrimExpr(shared, base, Default::default())
    }
}

impl<T: PrimType> ToListMappable for PrimExpr<T> {
    type O = MapListRef<PrimField<T>>;

    fn do_map(self, base: String) -> Self::O {
        MapListRef::new(self.0, base, self.1)
    }
}

impl<T: PrimType> ListToRecMappable for PrimExpr<T> {
    type O = MapListRefToRec<PrimField<T>>;

    fn do_map_rec(self, base: String, k: PrimExpr<String>) -> Self::O {
        MapListRefToRec::new(self.0, base, k.1, self.1)
    }
}

impl<T: PrimType> RecToListMappable for PrimExpr<T> {
    type O = MapRecRefToList<PrimField<T>>;

    fn do_map(self, base: String) -> Self::O {
        MapRecRefToList::new(self.0, base, self.1)
    }
}

impl<T: PrimType> ToObjMappable for PrimExpr<T> {
    type O = MapRecRef<PrimField<T>>;

    fn do_map_rec(self, base: String, k: PrimExpr<String>) -> Self::O {
        MapRecRef::new(self.0, base, k.1, self.1)
    }
}
