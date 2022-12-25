use std::{
    cell::RefCell,
    collections::{
        BTreeMap,
        HashMap,
    },
    fmt::Display,
    fs,
    hash::Hash,
    io::{
        self,
        Write,
    },
    marker::PhantomData,
    path::{
        Path,
        PathBuf,
    },
    process::{
        self,
        Command,
        Stdio,
    },
    rc::Rc,
};
use serde::{
    de::DeserializeOwned,
    Deserialize,
    Serialize,
};
use serde_json::{
    json,
    Value,
};
use thiserror::Error;

pub trait SerdeSkipDefault {
    fn is_default(&self) -> bool;
    fn is_not_default(&self) -> bool;
}

impl<T: Default + PartialEq> SerdeSkipDefault for T {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }

    fn is_not_default(&self) -> bool {
        !self.is_default()
    }
}

/// Use this to create a new stack.
pub struct BuildStack {
    pub state_path: PathBuf,
}

impl BuildStack {
    pub fn build(self) -> Stack {
        return Stack {
            state_path: self.state_path,
            provider_types: Default::default(),
            providers: Default::default(),
            variables: Default::default(),
            datasources: Default::default(),
            resources: Default::default(),
            outputs: Default::default(),
        };
    }
}

#[derive(Debug)]
pub enum ComponentType {
    ProviderType,
    Provider,
    Variable,
    Datasource,
    Resource,
    Output,
}

#[derive(Error, Debug)]
pub enum StackError {
    #[error("Duplicate {0:?} with tf_id {1}")]
    Duplicate(ComponentType, String),
}

#[derive(Error, Debug)]
pub enum RunError {
    #[error("Error serializing stack: {0:?}")]
    StackError(#[from]
    StackError),
    #[error("Failed to write configs: {0:?}")]
    FileError(#[from]
    io::Error),
    #[error("Failed to write or parse json: {0:?}")]
    JsonError(#[from]
    serde_json::Error),
    #[error("Command {0:?} failed with result {1:?}")]
    CommandError(Command, process::ExitStatus),
}

pub struct Stack {
    state_path: PathBuf,
    provider_types: Vec<Rc<dyn ProviderType>>,
    providers: Vec<Rc<dyn Provider>>,
    variables: Vec<Rc<dyn Variable>>,
    datasources: Vec<Rc<dyn Datasource>>,
    resources: Vec<Rc<dyn Resource>>,
    outputs: Vec<Rc<dyn Output>>,
}

impl Stack {
    /// Convert the stack to json bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, StackError> {
        let mut required_providers = BTreeMap::new();
        for p in &self.provider_types {
            if required_providers.insert(p.extract_tf_id(), p.extract_required_provider()).is_some() {
                Err(StackError::Duplicate(ComponentType::ProviderType, p.extract_tf_id()))?;
            }
        }
        let mut providers = BTreeMap::new();
        for p in &self.providers {
            providers.entry(p.extract_type_tf_id()).or_insert_with(Vec::new).push(p.extract_provider());
        }
        let mut variables = BTreeMap::new();
        for v in &self.variables {
            if variables.insert(v.extract_tf_id(), v.extract_value()).is_some() {
                Err(StackError::Duplicate(ComponentType::Variable, v.extract_tf_id()))?;
            }
        }
        let mut data = BTreeMap::new();
        for d in &self.datasources {
            if data
                .entry(d.extract_datasource_type())
                .or_insert_with(BTreeMap::new)
                .insert(d.extract_tf_id(), d.extract_value())
                .is_some() {
                Err(StackError::Duplicate(ComponentType::Datasource, d.extract_tf_id()))?;
            }
        }
        let mut resources = BTreeMap::new();
        for r in &self.resources {
            if resources
                .entry(r.extract_resource_type())
                .or_insert_with(BTreeMap::new)
                .insert(r.extract_tf_id(), r.extract_value())
                .is_some() {
                Err(StackError::Duplicate(ComponentType::Resource, r.extract_tf_id()))?;
            }
        }
        let mut outputs = BTreeMap::new();
        for o in &self.outputs {
            if outputs.insert(o.extract_tf_id(), o.extract_value()).is_some() {
                Err(StackError::Duplicate(ComponentType::Output, o.extract_tf_id()))?;
            }
        }
        let mut out = BTreeMap::new();
        out.insert("terraform", json!({
            "backend": {
                "local": {
                    "path": self.state_path,
                }
                ,
            }
            ,
            "required_providers": required_providers
        }));
        if !providers.is_empty() {
            out.insert("provider", json!(providers));
        }
        if !variables.is_empty() {
            out.insert("variable", json!(variables));
        }
        if !data.is_empty() {
            out.insert("data", json!(data));
        }
        if !resources.is_empty() {
            out.insert("resource", json!(resources));
        }
        if !outputs.is_empty() {
            out.insert("output", json!(outputs));
        }
        Ok(serde_json::to_vec_pretty(&out).unwrap())
    }

    pub fn add_provider_type(&mut self, v: Rc<dyn ProviderType>) {
        self.provider_types.push(v);
    }

    pub fn add_provider(&mut self, v: Rc<dyn Provider>) {
        self.providers.push(v);
    }

    pub fn add_datasource(&mut self, v: Rc<dyn Datasource>) {
        self.datasources.push(v);
    }

    pub fn add_resource(&mut self, v: Rc<dyn Resource>) {
        self.resources.push(v);
    }

    /// Serialize the stack to a file and run a Terraform command on it. If variables are
    /// provided, they must be a single-level struct where all values are primitives (i64,
    /// f64, String, bool).
    pub fn run<V: Serialize>(&self, path: &Path, variables: Option<&V>, mode: &str) -> Result<(), RunError> {
        let stack_path = path.join("stack.tf.json");
        fs::write(&stack_path, &self.serialize()?)?;
        let mut command = Command::new("terraform");
        command.current_dir(&path).arg(mode);
        if let Some(vars) = variables {
            let mut vars_file = tempfile::Builder::new().suffix(".json").tempfile()?;
            vars_file.as_file_mut().write_all(&serde_json::to_vec_pretty(&vars)?)?;
            command.arg(format!("-var-file={}", vars_file.path().to_string_lossy()));
            let res = command.status()?;
            if !res.success() {
                Err(RunError::CommandError(command, res))?;
            }
        } else {
            let res = command.status()?;
            if !res.success() {
                Err(RunError::CommandError(command, res))?;
            }
        }
        Ok(())
    }

    /// Gets the current outputs from an applied stack. `path` is the directory in which the
    /// .tf.json file was written. The output struct must be a single level and only have
    /// primitive values (i64, f64, String, bool).
    pub fn get_output<O: DeserializeOwned>(&self, path: &Path) -> Result<O, RunError> {
        let mut command = Command::new("terraform");
        let res = command.current_dir(&path).stderr(Stdio::inherit()).args(&["output", "-json"]).output()?;
        if !res.status.success() {
            return Err(RunError::CommandError(command, res.status));
        }

        // Redeserialize... hack
        #[derive(Deserialize)]
        struct Var {
            value: Value,
        }

        Ok(
            serde_json::from_slice(
                &serde_json::to_vec(
                    &serde_json::from_slice::<HashMap<String, Var>>(&res.stdout)?
                        .into_iter()
                        .map(|(k, v)| (k, v.value))
                        .collect::<HashMap<String, Value>>(),
                )?,
            )?,
        )
    }
}

// Primitives
pub trait TfPrimitiveType {
    fn extract_variable_type() -> String;
}

impl TfPrimitiveType for String {
    fn extract_variable_type() -> String {
        "string".into()
    }
}

impl TfPrimitiveType for bool {
    fn extract_variable_type() -> String {
        "bool".into()
    }
}

impl TfPrimitiveType for i64 {
    fn extract_variable_type() -> String {
        "int".into()
    }
}

impl TfPrimitiveType for f64 {
    fn extract_variable_type() -> String {
        "float".into()
    }
}

pub trait PrimitiveType: Serialize + Clone + TfPrimitiveType + Default + PartialEq { }

impl<T: Serialize + Clone + TfPrimitiveType + Default + PartialEq> PrimitiveType for T { }

/// In Terraform, all fields, regardless of whether a it's an int or bool or whatever,
/// can also take references like `${}`. `Primitive` represents this sort of value.
/// Base types `i64` `f64` `String` and `bool` are supported, and you should be able
/// to convert to `Primitive` with `into()`. Resource methods will return typed
/// references that can also be used here.
#[derive(Clone)]
pub enum Primitive<T: PrimitiveType> {
    Literal(T),
    Reference(String),
}

impl<T: PrimitiveType> Default for Primitive<T> {
    fn default() -> Self {
        Primitive::Literal(T::default())
    }
}

impl<T: PrimitiveType + Hash> Hash for Primitive<T> {
    // Conditional derive somehow?
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl<T: PrimitiveType + PartialEq> std::cmp::PartialEq for Primitive<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Literal(l0), Self::Literal(r0)) => l0 == r0,
            (Self::Reference(l0), Self::Reference(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl<T: PrimitiveType + std::cmp::Eq + PartialEq> std::cmp::Eq for Primitive<T> { }

impl<T: PrimitiveType> Serialize for Primitive<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        match self {
            Primitive::Literal(l) => l.serialize(serializer),
            Primitive::Reference(r) => r.serialize(serializer),
        }
    }
}

impl<T: PrimitiveType> From<&T> for Primitive<T> {
    fn from(v: &T) -> Self {
        Primitive::Literal(v.clone())
    }
}

impl<T: PrimitiveType> From<T> for Primitive<T> {
    fn from(v: T) -> Self {
        Primitive::Literal(v)
    }
}

impl From<&str> for Primitive<String> {
    fn from(v: &str) -> Self {
        Primitive::Literal(v.to_string())
    }
}

impl<T: PrimitiveType + Display> Display for Primitive<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Primitive::Literal(v) => v.fmt(f),
            Primitive::Reference(v) => v.fmt(f),
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
    fn provider_ref(&self) -> String;
}

impl<T: Provider> From<T> for Primitive<String> {
    fn from(value: T) -> Self {
        Primitive::Reference(value.provider_ref())
    }
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
    fn resource_ref(&self) -> String;
}

// Variable
trait Variable {
    fn extract_tf_id(&self) -> String;
    fn extract_value(&self) -> Value;
}

#[derive(Serialize)]
struct VariableImplData {
    #[serde(skip_serializing_if = "SerdeSkipDefault::is_default")]
    pub r#type: String,
    #[serde(skip_serializing_if = "SerdeSkipDefault::is_not_default")]
    pub nullable: Primitive<bool>,
    #[serde(skip_serializing_if = "SerdeSkipDefault::is_default")]
    pub sensitive: Primitive<bool>,
}

pub struct VariableImpl<T: PrimitiveType> {
    tf_id: String,
    data: RefCell<VariableImplData>,
    _p: PhantomData<T>,
}

impl<T: PrimitiveType> Variable for VariableImpl<T> {
    fn extract_tf_id(&self) -> String {
        self.tf_id.clone()
    }

    fn extract_value(&self) -> Value {
        let data = self.data.borrow();
        serde_json::to_value(&*data).unwrap()
    }
}

impl<T: PrimitiveType> VariableImpl<T> {
    pub fn set_nullable(&self, v: impl Into<Primitive<bool>>) -> &Self {
        self.data.borrow_mut().nullable = v.into();
        self
    }

    pub fn set_sensitive(&self, v: impl Into<Primitive<bool>>) -> &Self {
        self.data.borrow_mut().sensitive = v.into();
        self
    }
}

impl<T: PrimitiveType> Into<Primitive<T>> for &VariableImpl<T> {
    fn into(self) -> Primitive<T> {
        Primitive::Reference(format!("${{var.{}}}", self.tf_id))
    }
}

/// Create a new variable.
pub struct BuildVariable {
    pub tf_id: String,
}

impl BuildVariable {
    pub fn build<T: PrimitiveType + 'static>(self, stack: &mut Stack) -> Rc<VariableImpl<T>> {
        let out = Rc::new(VariableImpl {
            tf_id: self.tf_id,
            data: RefCell::new(VariableImplData {
                r#type: T::extract_variable_type(),
                nullable: false.into(),
                sensitive: false.into(),
            }),
            _p: Default::default(),
        });
        stack.variables.push(out.clone());
        out
    }
}

// Output
trait Output {
    fn extract_tf_id(&self) -> String;
    fn extract_value(&self) -> Value;
}

#[derive(Serialize)]
struct OutputImplData<T: PrimitiveType> {
    #[serde(skip_serializing_if = "SerdeSkipDefault::is_default")]
    pub sensitive: Primitive<bool>,
    #[serde(skip_serializing_if = "SerdeSkipDefault::is_default")]
    pub value: Primitive<T>,
}

pub struct OutputImpl<T: PrimitiveType> {
    tf_id: String,
    data: RefCell<OutputImplData<T>>,
}

impl<T: PrimitiveType> OutputImpl<T> {
    pub fn set_sensitive(&self, v: impl Into<Primitive<bool>>) -> &Self {
        self.data.borrow_mut().sensitive = v.into();
        self
    }
}

impl<T: PrimitiveType> Into<Primitive<T>> for &OutputImpl<T> {
    fn into(self) -> Primitive<T> {
        Primitive::Reference(format!("${{variable.{}}}", self.tf_id))
    }
}

impl<T: PrimitiveType> Output for OutputImpl<T> {
    fn extract_tf_id(&self) -> String {
        self.tf_id.clone()
    }

    fn extract_value(&self) -> Value {
        let data = self.data.borrow();
        serde_json::to_value(&*data).unwrap()
    }
}

/// Create a new output.
pub struct BuildOutput<T: PrimitiveType> {
    pub tf_id: String,
    pub value: Primitive<T>,
}

impl<T: PrimitiveType + 'static> BuildOutput<T> {
    pub fn build(self, stack: &mut Stack) -> Rc<OutputImpl<T>> {
        let out = Rc::new(OutputImpl {
            tf_id: self.tf_id,
            data: RefCell::new(OutputImplData {
                sensitive: false.into(),
                value: self.value,
            }),
        });
        stack.outputs.push(out.clone());
        out
    }
}

// Provider extras
#[derive(Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IgnoreChangesAll {
    All,
}

#[derive(Serialize, PartialEq)]
#[serde(untagged)]
pub enum IgnoreChanges {
    All(IgnoreChangesAll),
    Refs(Vec<String>),
}

#[derive(Serialize, Default, PartialEq)]
pub struct ResourceLifecycle {
    pub create_before_destroy: bool,
    pub prevent_destroy: bool,
    pub ignore_changes: Option<IgnoreChanges>,
    pub replace_triggered_by: Vec<String>,
}

#[macro_export]
macro_rules! primvec{
    ($($e: expr), *) => {
        vec![$(terrars:: Primitive:: from($e)), *]
    }
}
