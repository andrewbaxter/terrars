use serde::Serialize;
use crate::{
    TfPrimitiveType,
    list_ref::{
        ListRef,
        MapListRef,
    },
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
        Self::Sentinel(
            value
                .shared
                .add_sentinel(
                    &format!(
                        "[for each in [for i, v in {}: {{ key = i, value = v }}]: {}]",
                        value.base,
                        value.map_base
                    ),
                ),
        )
    }
}

impl<T> From<MapListRef<T>> for ListField<T> {
    fn from(value: MapListRef<T>) -> Self {
        (&value).into()
    }
}

impl<T> From<&MapRecRefToList<T>> for ListField<T> {
    fn from(value: &MapRecRefToList<T>) -> Self {
        Self::Sentinel(
            value
                .shared
                .add_sentinel(
                    &format!(
                        "[for each in [for k, v in {}: {{ key = k, value = v }}]: {}]",
                        value.base,
                        value.map_base
                    ),
                ),
        )
    }
}

impl<T> From<MapRecRefToList<T>> for ListField<T> {
    fn from(value: MapRecRefToList<T>) -> Self {
        (&value).into()
    }
}
