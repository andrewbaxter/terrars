use serde::{
    ser::SerializeMap,
    Serialize,
};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct EncryptionKeyProvider {
    type_: String,
    name: String,
    config: BTreeMap<String, Value>,
}

impl EncryptionKeyProvider {
    pub fn custom(key_provider_type: impl Into<String>, name: impl Into<String>) -> Self {
        EncryptionKeyProvider {
            type_: key_provider_type.into(),
            name: name.into(),
            config: BTreeMap::new(),
        }
    }

    pub fn set(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.config.insert(key.into(), value.into());
        self
    }

    pub fn reference(&self) -> String {
        format!("${{key_provider.{}.{}}}", self.type_, self.name)
    }
}

pub struct BuildEncryptionKeyProviderPbkdf2 {
    pub name: String,
    pub passphrase: String,
}

impl BuildEncryptionKeyProviderPbkdf2 {
    pub fn build(self) -> EncryptionKeyProvider {
        EncryptionKeyProvider::custom("pbkdf2", self.name).set("passphrase", self.passphrase)
    }
}

#[derive(Clone)]
pub struct EncryptionMethod {
    type_: String,
    name: String,
    config: BTreeMap<String, Value>,
}

impl EncryptionMethod {
    pub fn custom(method_type: impl Into<String>, name: impl Into<String>) -> Self {
        EncryptionMethod {
            type_: method_type.into(),
            name: name.into(),
            config: BTreeMap::new(),
        }
    }

    pub fn set(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.config.insert(key.into(), value.into());
        self
    }

    pub fn reference(&self) -> String {
        format!("${{method.{}.{}}}", self.type_, self.name)
    }

    pub fn clone_ref(&self) -> Self {
        self.clone()
    }
}

pub struct BuildEncryptionMethodAesGcm<'a> {
    pub name: String,
    pub keys: &'a EncryptionKeyProvider,
}

impl<'a> BuildEncryptionMethodAesGcm<'a> {
    pub fn build(self) -> EncryptionMethod {
        EncryptionMethod::custom("aes_gcm", self.name).set("keys", self.keys.reference())
    }
}

pub struct BuildEncryptionMethodUnencrypted {
    pub name: String,
}

impl BuildEncryptionMethodUnencrypted {
    pub fn build(self) -> EncryptionMethod {
        EncryptionMethod::custom("unencrypted", self.name)
    }
}

#[derive(Serialize)]
struct EncryptionFallback {
    method: String,
}

#[derive(Serialize)]
pub struct EncryptionTarget {
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    enforced: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fallback: Option<EncryptionFallback>,
}

impl EncryptionTarget {
    pub fn new(method: &EncryptionMethod) -> Self {
        EncryptionTarget {
            method: method.reference(),
            enforced: None,
            fallback: None,
        }
    }

    pub fn set_enforced(mut self, v: bool) -> Self {
        self.enforced = Some(v);
        self
    }

    pub fn set_fallback(mut self, method: &EncryptionMethod) -> Self {
        self.fallback = Some(EncryptionFallback { method: method.reference() });
        self
    }
}

#[derive(Default)]
pub struct Encryption {
    key_providers: Vec<EncryptionKeyProvider>,
    methods: Vec<EncryptionMethod>,
    state: Option<EncryptionTarget>,
    plan: Option<EncryptionTarget>,
}

impl Encryption {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_key_provider(mut self, v: EncryptionKeyProvider) -> Self {
        self.key_providers.push(v);
        self
    }

    pub fn add_method(mut self, v: EncryptionMethod) -> Self {
        self.methods.push(v);
        self
    }

    pub fn set_state(mut self, v: EncryptionTarget) -> Self {
        self.state = Some(v);
        self
    }

    pub fn set_plan(mut self, v: EncryptionTarget) -> Self {
        self.plan = Some(v);
        self
    }
}

fn nest<'a>(
    items: impl Iterator<Item = (&'a str, &'a str, &'a BTreeMap<String, Value>)>,
) -> BTreeMap<&'a str, BTreeMap<&'a str, &'a BTreeMap<String, Value>>> {
    let mut out: BTreeMap<&str, BTreeMap<&str, &BTreeMap<String, Value>>> = BTreeMap::new();
    for (type_, name, config) in items {
        out.entry(type_).or_default().insert(name, config);
    }
    out
}

impl Serialize for Encryption {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        let mut m = serializer.serialize_map(None)?;
        if !self.key_providers.is_empty() {
            m.serialize_entry(
                "key_provider",
                &nest(self.key_providers.iter().map(|p| (p.type_.as_str(), p.name.as_str(), &p.config))),
            )?;
        }
        if !self.methods.is_empty() {
            m.serialize_entry(
                "method",
                &nest(self.methods.iter().map(|p| (p.type_.as_str(), p.name.as_str(), &p.config))),
            )?;
        }
        if let Some(state) = &self.state {
            m.serialize_entry("state", state)?;
        }
        if let Some(plan) = &self.plan {
            m.serialize_entry("plan", plan)?;
        }
        m.end()
    }
}
