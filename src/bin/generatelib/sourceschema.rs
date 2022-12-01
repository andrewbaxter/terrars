use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct ProviderSchemas {
    #[serde(default)]
    pub provider_schemas: BTreeMap<String, ProviderSchema>,
}

#[derive(Deserialize)]
pub struct ProviderSchema {
    pub provider: Provider,
    #[serde(default)]
    pub data_source_schemas: BTreeMap<String, SchemaItem>,
    #[serde(default)]
    pub resource_schemas: BTreeMap<String, SchemaItem>,
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
    #[serde(default)]
    pub attributes: BTreeMap<String, Value>,
    #[serde(default)]
    pub block_types: BTreeMap<String, NestedBlock>,
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
pub enum AggCollTypeKey {
    Set,
    List,
    Map,
}

#[derive(Deserialize)]
pub struct AggCollType(pub AggCollTypeKey, pub ValueSchema);

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggObjTypeKey {
    Object,
}

#[derive(Deserialize)]
pub struct AggObjType(pub AggObjTypeKey, pub BTreeMap<String, ValueSchema>);

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ValueSchema {
    Simple(ScalarTypeKey),
    AggColl(Box<AggCollType>),
    AggObject(Box<AggObjType>),
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DescriptionKind {
    Plain,
    Markdown,
}

pub enum ValueBehaviorHelper {
    UserRequired,
    UserOptional,
    Computed,
    UserOptionalComputed,
}

#[derive(Deserialize)]
pub struct Value {
    pub r#type: ValueSchema,
    pub description: Option<String>,
    pub description_kind: Option<DescriptionKind>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub computed: bool,
    #[serde(default)]
    pub sensitive: bool,
}

impl Value {
    pub fn behavior(&self) -> ValueBehaviorHelper {
        match (self.required, self.optional, self.computed) {
            (true, false, false) => ValueBehaviorHelper::UserRequired,
            (false, true, false) => ValueBehaviorHelper::UserOptional,
            (false, false, true) => ValueBehaviorHelper::Computed,
            (false, true, true) => ValueBehaviorHelper::UserOptionalComputed,
            _ => panic!(
                "Unsupported behavior {} {} {}",
                self.required, self.optional, self.computed
            ),
        }
    }
}

#[derive(Deserialize)]
pub struct NestedBlock {
    pub block: Block,
    pub nesting_mode: NestingMode,
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
