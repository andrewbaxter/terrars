use serde::Serialize;
use crate::{
    TfPrimitiveType,
    list_ref::{
        ListRef,
        MapListRef,
    },
    DynamicBlock,
    rec_ref::MapRecRefToList,
};

pub enum ListField<T> {
    Literal(Vec<T>),
    Sentinel(String),
}

impl<T: Serialize> Serialize for ListField<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        match self {
            ListField::Literal(x) => x.serialize(serializer),
            ListField::Sentinel(t) => t.serialize2(serializer),
        }
    }
}

impl<T> From<Vec<T>> for ListField<T> {
    fn from(value: Vec<T>) -> Self {
        Self::Literal(value)
    }
}

impl<T> From<&ListRef<T>> for ListField<T> {
    fn from(value: &ListRef<T>) -> Self {
        Self::Sentinel(value.shared.add_sentinel(&value.base))
    }
}

impl<T> From<ListRef<T>> for ListField<T> {
    fn from(value: ListRef<T>) -> Self {
        (&value).into()
    }
}

impl<T> From<&MapListRef<T>> for ListField<T> {
    fn from(value: &MapListRef<T>) -> Self {
        Self::Sentinel(value.shared.add_sentinel(&format!("[for each in {}: {}]", value.base, value.map_base)))
    }
}

impl<T> From<MapListRef<T>> for ListField<T> {
    fn from(value: MapListRef<T>) -> Self {
        (&value).into()
    }
}

impl<T> From<&MapRecRefToList<T>> for ListField<T> {
    fn from(value: &MapRecRefToList<T>) -> Self {
        Self::Sentinel(value.shared.add_sentinel(&format!("[for k, v in {}: {}]", value.base, value.map_base)))
    }
}

impl<T> From<MapRecRefToList<T>> for ListField<T> {
    fn from(value: MapRecRefToList<T>) -> Self {
        (&value).into()
    }
}

pub enum BlockListField<T: Serialize> {
    Literal(Vec<T>),
    Dynamic(DynamicBlock<T>),
}

impl<T: Serialize> Serialize for BlockListField<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        match self {
            BlockListField::Literal(x) => x.serialize(serializer),
            BlockListField::Dynamic(t) => t.serialize(serializer),
        }
    }
}

impl<T: Serialize> From<Vec<T>> for BlockListField<T> {
    fn from(value: Vec<T>) -> Self {
        Self::Literal(value)
    }
}
