use serde::Serialize;
use crate::{
    TfPrimitiveType,
    DynamicBlock,
    list_ref::MapListRef,
    rec_ref::MapRecRefToList,
    BlockListField,
};

pub enum SetField<T> {
    Literal(Vec<T>),
    Sentinel(String),
}

impl<T: Serialize> Serialize for SetField<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        match self {
            SetField::Literal(x) => x.serialize(serializer),
            SetField::Sentinel(t) => t.serialize2(serializer),
        }
    }
}

impl<T> From<Vec<T>> for SetField<T> {
    fn from(value: Vec<T>) -> Self {
        Self::Literal(value)
    }
}

impl<T> From<&MapListRef<T>> for SetField<T> {
    fn from(value: &MapListRef<T>) -> Self {
        Self::Sentinel(
            value.shared.add_sentinel(&format!("toset([for each in {}: {}])", value.base, value.map_base)),
        )
    }
}

impl<T> From<MapListRef<T>> for SetField<T> {
    fn from(value: MapListRef<T>) -> Self {
        (&value).into()
    }
}

impl<T> From<&MapRecRefToList<T>> for SetField<T> {
    fn from(value: &MapRecRefToList<T>) -> Self {
        Self::Sentinel(
            value.shared.add_sentinel(&format!("toset([for k, v in {}: {}])", value.base, value.map_base)),
        )
    }
}

impl<T> From<MapRecRefToList<T>> for SetField<T> {
    fn from(value: MapRecRefToList<T>) -> Self {
        (&value).into()
    }
}

pub enum BlockSetField<T: Serialize> {
    Literal(Vec<T>),
    Dynamic(DynamicBlock<T>),
}

impl<T: Serialize> Serialize for BlockSetField<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        match self {
            BlockSetField::Literal(x) => x.serialize(serializer),
            BlockSetField::Dynamic(t) => t.serialize(serializer),
        }
    }
}

impl<T: Serialize> From<Vec<T>> for BlockSetField<T> {
    fn from(value: Vec<T>) -> Self {
        Self::Literal(value)
    }
}

impl<T: Serialize> From<BlockListField<T>> for BlockSetField<T> {
    fn from(value: BlockListField<T>) -> Self {
        match value {
            BlockListField::Literal(l) => BlockSetField::Literal(l),
            BlockListField::Dynamic(d) => {
                Self::Dynamic(DynamicBlock {
                    for_each: format!("toset({})", d.for_each),
                    content: d.content,
                })
            },
        }
    }
}
