use std::collections::HashMap;
use serde::Serialize;
use crate::{
    TfPrimitiveType,
    rec_ref::{
        RecRef,
        MapRecRef,
    },
    prim_ref::PrimRef,
    list_ref::MapListRefToRec,
};

pub enum RecField<T> {
    Literal(HashMap<String, T>),
    Sentinel(String),
}

impl<T: Serialize> Serialize for RecField<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        match self {
            RecField::Literal(x) => x.serialize(serializer),
            RecField::Sentinel(t) => t.serialize2(serializer),
        }
    }
}

impl<T> From<HashMap<String, T>> for RecField<T> {
    fn from(value: HashMap<String, T>) -> Self {
        Self::Literal(value)
    }
}

impl<T: PrimRef> From<&RecRef<T>> for RecField<T> {
    fn from(value: &RecRef<T>) -> Self {
        Self::Sentinel(value.shared.add_sentinel(&value.base))
    }
}

impl<T: PrimRef> From<&MapRecRef<T>> for RecField<T> {
    fn from(value: &MapRecRef<T>) -> Self {
        Self::Sentinel(
            value
                .shared
                .add_sentinel(
                    &format!("{{for k, v in {}: {} => {}}}", value.base, value.map_base_key, value.map_base),
                ),
        )
    }
}

impl<T: PrimRef> From<&MapListRefToRec<T>> for RecField<T> {
    fn from(value: &MapListRefToRec<T>) -> Self {
        Self::Sentinel(
            value
                .shared
                .add_sentinel(
                    &format!("{{for each in {}: {} => {}}}", value.base, value.map_base_key, value.map_base),
                ),
        )
    }
}
