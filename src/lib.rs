use std::{collections::BTreeMap, rc::Rc, cell::RefCell};

use serde::Serialize;
use serde_json::{Value, json};

pub struct BuildStack {
    state_path: String,
}

impl BuildStack {
    build(self) -> Stack {
        return Stack{
            state_path: self.state_path,
            ..Default::default(),
        }
    }
}

#[derive(Default)]
pub struct Stack {
    state_path: String,
    provider_types: Vec<Rc<dyn ProviderType>>,
    providers: Vec<Rc<dyn Provider>>,
    variables: Vec<Rc<Variable>>,
    datasources: Vec<Rc<dyn Datasource>>,
    resources: Vec<Rc<dyn Resource>>,
    outputs: Vec<Output>,
}

impl Stack {
    fn serialize(&self) -> Vec<u8> {
        let mut required_providers = BTreeMap::new();
        for p in &self.provider_types {
            required_providers.insert(p.extract_tf_id(), p.extract_required_provider());
        }
        
        let mut providers = BTreeMap::new();
            for p in &self.providers {
                providers.entry(p.extract_type_tf_id()).or_insert_with(Vec::new)
                .push(p.extract_provider());
            }
        
            let mut variables = BTreeMap::new();
            for v in &self.variables {
                variables.insert(v.tf_id, &v.data);
            }

        let mut data = BTreeMap::new();
            for d in &self.datasources {
                data.entry(d.extract_datasource_type()).or_insert_with(BTreeMap::new).
                insert(d.extract_tf_id(), d.extract_datasource_type());
            }

        let mut resources = BTreeMap::new();
            for r in &self.resources {
                data.entry(r.extract_resource_type()).or_insert_with(BTreeMap::new)
                .insert(r.extract_tf_id(), r.extract_resource_type());
            }

        let mut outputs = BTreeMap::new();
        for o in &self.outputs {
            outputs.insert(o.tf_id, &o.data);
        }

        serde_json::to_vec(&json!({
            "terraform": {
                "backend": {
                    "local": {
                        "path": self.state_path,
                    },
                },
                "required_providers": required_providers,
            },
            "provider": providers,
            "variable": variables,
            "data": data,
            "resource": resources,
            "output": outputs,
        })).unwrap()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveType {
    String,
    Bool,
    Int,
    Float,
}

pub enum Primitive<T: Serialize> {
    Literal(T),
    Reference(String),
}

impl<T: Serialize> Serialize for Primitive<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Primitive::Literal(l) => l.serialize(serializer),
            Primitive::Reference(r) => r.serialize(serializer),
        }
    }
}

// Generated traits
pub trait ProviderType {
    fn extract_tf_id(&self) -> String;
    fn extract_required_provider(&self) -> Value;
}

pub trait Provider {
    fn extract_type_tf_id(&self) -> String;
    fn extract_provider(&self) -> Value;
}

pub trait Datasource {
    fn extract_datasource_type(&self) -> String;
    fn extract_tf_id(&self) -> String;
    fn extract_value(&self) -> Value;
}

pub trait Resource {
    fn extract_resource_type(&self) -> String;
    fn extract_tf_id(&self) -> String;
    fn extract_value(&self) -> Value;
}

// Variable
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct Variable_ {
    pub r#type: PrimitiveType,
    pub nullable: bool,
    pub sensitive: bool,
}

pub struct Variable{
    tf_id: String,
    data: RefCell<Variable_>,
};

impl Referable for Variable {
    fn refer(&self) -> String {
        format!("${{variable.{}}}", self.0.id)
    }
}

pub struct BuildVariable {
    pub id: String,
    pub r#type: PrimitiveType,
}

impl BuildVariable {
    pub fn build(self, stack: &mut Stack) -> Rc<Variable> {
        let out = Rc::new(Variable{
            tf_id: self.id,
            RefCell::new(Variable_{
                value: Variable_ {
                    r#type: self.r#type,
                    nullable: false,
                    sensitive: false,
                },
            }),
    });
    stack.variables.push(out.clone());
    out
    }
}

// Output
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct Output_ {
    pub sensitive: bool,
    pub value: Primitive<String>,
}

pub struct Output {
    tf_id: String,
    data: RefCell<Output_>,
};

pub struct BuildOutput {
    pub id: String,
    pub value: Primitive<String>,
}

impl BuildOutput {
    pub fn build(self, stack:&mut Stack) -> Rc<Output> {
        let out = Rc::new(Output{
            tf_id: self.id,
            data: RefCell::new(Output_ {
                value: self.value,
                sensitive: false,
            }),
        });
        stack.outputs.push(out.clone());
        out
    }
}