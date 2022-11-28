use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct ProviderSchemas {
    pub provider_schemas: BTreeMap<String, ProviderSchema>,
}

#[derive(Deserialize)]
pub struct ProviderSchema {
    pub provider: Provider,
    pub data_source_schemas: Option<BTreeMap<String, SchemaItem>>,
    pub resource_schemas: Option<BTreeMap<String, SchemaItem>>,
}

#[derive(Deserialize)]
pub struct Provider {
    pub block: Block,
}

#[derive(Deserialize)]
pub struct SchemaItem {
    pub block: Block,
}

#[derive(Deserialize)]
pub struct Block {
    pub attributes: Option<BTreeMap<String, Value>>,
    pub block_types: Option<BTreeMap<String, NestedBlock>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScalarTypeKey {
    Number,
    Integer,
    String,
    Bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggSimpleTypeKey {
    Set,
    List,
    Map,
}

#[derive(Deserialize)]
pub struct AggSimpleType(pub AggSimpleTypeKey, pub ValueType);

pub enum AggObjTypeKey {
    Object,
}

#[derive(Deserialize)]
pub struct AggObjectType(pub AggObjTypeKey, pub BTreeMap<String, ValueType>);

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ValueType {
    Simple(ScalarTypeKey),
    AggSimple(AggSimpleType),
    AggObject(AggObjType),
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DescriptionKind {
    Plain,
    Markdown,
}

#[derive(Deserialize)]
pub struct Value {
    pub r#type: ValueType,
    pub description: Option<String>,
    pub description_kind: Option<DescriptionKind>,
    pub required: bool,
    pub optional: bool,
    pub computed: bool,
    pub sensitive: bool,
}

#[derive(Deserialize)]
pub struct NestedBlock {
    pub block: Block,
    pub nesting_mode: Option<NestingMode>,
    pub min_items: Option<u64>,
    pub max_items: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NestingMode {
    List,
    Set,
    Single,
}
