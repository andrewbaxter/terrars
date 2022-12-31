use std::{
    cell::{
        RefCell,
    },
    collections::{
        BTreeMap,
        HashMap,
    },
    fmt::Display,
    fs::{
        self,
        create_dir_all,
    },
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
    str::FromStr,
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
pub struct BuildStack {}

impl BuildStack {
    pub fn build(self) -> Stack {
        return Stack {
            provider_types: Default::default(),
            providers: Default::default(),
            variables: Default::default(),
            datasources: Default::default(),
            resources: Default::default(),
            outputs: Default::default(),
            replace_exprs: Default::default(),
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
    #[error("Failed to prepare run directory {0:?}: {1:?}")]
    FsError(PathBuf, io::Error),
    #[error("Error serializing stack: {0:?}")]
    StackError(
        #[from]
        StackError,
    ),
    #[error("Failed to write configs: {0:?}")]
    FileError(
        #[from]
        io::Error,
    ),
    #[error("Failed to write or parse json: {0:?}")]
    JsonError(
        #[from]
        serde_json::Error,
    ),
    #[error("Command {0:?} failed with result {1:?}")]
    CommandError(Command, process::ExitStatus),
}

pub struct Stack {
    provider_types: Vec<Rc<dyn ProviderType>>,
    providers: Vec<Rc<dyn Provider>>,
    variables: Vec<Rc<dyn VariableTrait>>,
    datasources: Vec<Rc<dyn Datasource_>>,
    resources: Vec<Rc<dyn Resource_>>,
    outputs: Vec<Rc<dyn Output>>,
    replace_exprs: Vec<(String, String)>,
}

thread_local!{
    static REPLACE_EXPRS: RefCell<Option<Vec<(String, String)>>> = RefCell::new(None);
}

impl Stack {
    /// Convert the stack to json bytes.
    pub fn serialize(&self, state_path: &Path) -> Result<Vec<u8>, StackError> {
        REPLACE_EXPRS.with(move |f| {
            *f.borrow_mut() = Some(self.replace_exprs.clone());
        });
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
                    "path": state_path.to_string_lossy(),
                },
            },
            "required_providers": required_providers,
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
        REPLACE_EXPRS.with(|f| *f.borrow_mut() = None);
        let res = serde_json::to_vec_pretty(&out).unwrap();
        Ok(res)
    }

    pub fn add_provider_type(&mut self, v: Rc<dyn ProviderType>) {
        self.provider_types.push(v);
    }

    pub fn add_provider(&mut self, v: Rc<dyn Provider>) {
        self.providers.push(v);
    }

    pub fn add_datasource(&mut self, v: Rc<dyn Datasource_>) {
        self.datasources.push(v);
    }

    pub fn add_resource(&mut self, v: Rc<dyn Resource_>) {
        self.resources.push(v);
    }

    pub fn add_sentinel(&mut self, v: String) -> String {
        let k = format!("_TERRARS_SENTINEL_{}_", self.replace_exprs.len());
        self.replace_exprs.push((k.clone(), v));
        k
    }

    /// Serialize the stack to a file and run a Terraform command on it. If variables are
    /// provided, they must be a single-level struct where all values are primitives (i64,
    /// f64, String, bool).
    pub fn run<V: Serialize>(&self, path: &Path, variables: Option<&V>, mode: &str) -> Result<(), RunError> {
        create_dir_all(path).map_err(|e| RunError::FsError(path.to_path_buf(), e))?;
        let state_name = "state.tfstate";
        fs::write(&path.join("stack.tf.json"), &self.serialize(&PathBuf::from_str(state_name).unwrap())?)?;
        let state_path = path.join(state_name);
        if !state_path.exists() {
            let mut command = Command::new("terraform");
            command.current_dir(&path).arg("init");
            let res = command.status()?;
            if !res.success() {
                return Err(RunError::CommandError(command, res));
            }
        }
        let mut command = Command::new("terraform");
        command.current_dir(&path).arg(mode);
        if let Some(vars) = variables {
            let mut vars_file = tempfile::Builder::new().suffix(".json").tempfile()?;
            vars_file.as_file_mut().write_all(&serde_json::to_vec_pretty(&vars)?)?;
            command.arg(format!("-var-file={}", vars_file.path().to_string_lossy()));
            let res = command.status()?;
            if !res.success() {
                return Err(RunError::CommandError(command, res))?;
            }
        } else {
            let res = command.status()?;
            if !res.success() {
                return Err(RunError::CommandError(command, res))?;
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
    fn serialize2<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer;
}

impl TfPrimitiveType for String {
    fn extract_variable_type() -> String {
        "string".into()
    }

    fn serialize2<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        REPLACE_EXPRS.with(|f| {
            if let Some(vs) = f.borrow().as_ref() {
                let mut out = self.replace("%{", "%%{").replace("${", "$${");
                for (k, v) in vs {
                    out = out.replace(k, v);
                }
                out.serialize(serializer)
            } else {
                self.serialize(serializer)
            }
        })
    }
}

impl TfPrimitiveType for bool {
    fn extract_variable_type() -> String {
        "bool".into()
    }

    fn serialize2<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        self.serialize(serializer)
    }
}

impl TfPrimitiveType for i64 {
    fn extract_variable_type() -> String {
        "int".into()
    }

    fn serialize2<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        self.serialize(serializer)
    }
}

impl TfPrimitiveType for f64 {
    fn extract_variable_type() -> String {
        "float".into()
    }

    fn serialize2<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        self.serialize(serializer)
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
    Sentinel(String),
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
            (Self::Sentinel(l0), Self::Sentinel(r0)) => l0 == r0,
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
            Primitive::Literal(l) => l.serialize2(serializer),
            Primitive::Sentinel(r) => r.serialize2(serializer),
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
            Primitive::Sentinel(v) => v.fmt(f),
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
    fn extract_ref(&self) -> String;
}

pub trait Datasource_ {
    fn extract_datasource_type(&self) -> String;
    fn extract_tf_id(&self) -> String;
    fn extract_value(&self) -> Value;
}

pub trait Resource {
    fn extract_ref(&self) -> String;
}

pub trait Resource_ {
    fn extract_resource_type(&self) -> String;
    fn extract_tf_id(&self) -> String;
    fn extract_value(&self) -> Value;
}

// Expressions
pub trait TfExpr {
    fn to_string(&self, stack: &mut Stack) -> String {
        stack.add_sentinel(format!("${{{}}}", self.raw()))
    }

    fn to_prim<T: PrimitiveType>(&self, stack: &mut Stack) -> Primitive<T> {
        Primitive::Sentinel(self.to_string(stack))
    }
    fn raw(&self) -> String;
}

pub struct PrimExpr(String);

impl PrimExpr {
    pub fn str(val: impl ToString) -> PrimExpr {
        PrimExpr(format!("\"{}\"", val.to_string().replace("\"", "\\\"")))
    }

    pub fn bool(val: bool) -> PrimExpr {
        PrimExpr(if val {
            "true"
        } else {
            "false"
        }.into())
    }

    pub fn i64(val: i64) -> PrimExpr {
        PrimExpr(val.to_string())
    }

    pub fn f64(val: f64) -> PrimExpr {
        PrimExpr(val.to_string())
    }
}

impl TfExpr for PrimExpr {
    fn raw(&self) -> String {
        self.0.clone()
    }
}

// References
pub trait Ref {
    fn new(base: String) -> Self;
}

pub struct PrimRef<T: PrimitiveType> {
    base: String,
    _pd: PhantomData<T>,
}

impl<T: PrimitiveType> Ref for PrimRef<T> {
    fn new(base: String) -> PrimRef<T> {
        PrimRef {
            base: base,
            _pd: Default::default(),
        }
    }
}

impl<T: PrimitiveType> TfExpr for PrimRef<T> {
    fn raw(&self) -> String {
        self.base.clone()
    }
}

pub struct ListRef<T: Ref> {
    base: String,
    _pd: PhantomData<T>,
}

impl<T: Ref> Ref for ListRef<T> {
    fn new(base: String) -> Self {
        ListRef {
            base: base,
            _pd: Default::default(),
        }
    }
}

impl<T: Ref> ListRef<T> {
    pub fn get(&self, index: usize) -> T {
        T::new(format!("{}[{}]", &self.base, index))
    }
}

pub struct MapRef<T: Ref> {
    base: String,
    _pd: PhantomData<T>,
}

impl<T: Ref> Ref for MapRef<T> {
    fn new(base: String) -> Self {
        MapRef {
            base: base,
            _pd: Default::default(),
        }
    }
}

impl<T: Ref> MapRef<T> {
    pub fn get(&self, key: impl ToString) -> T {
        T::new(format!("{}[\"{}\"]", &self.base, key.to_string()))
    }
}

// Variable
trait VariableTrait {
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

struct Variable_<T: PrimitiveType> {
    tf_id: String,
    sentinel: String,
    data: RefCell<VariableImplData>,
    _p: PhantomData<T>,
}

pub struct Variable<T: PrimitiveType>(Rc<Variable_<T>>);

impl<T: PrimitiveType> VariableTrait for Variable_<T> {
    fn extract_tf_id(&self) -> String {
        self.tf_id.clone()
    }

    fn extract_value(&self) -> Value {
        let data = self.data.borrow();
        serde_json::to_value(&*data).unwrap()
    }
}

impl<T: PrimitiveType> Variable<T> {
    pub fn set_nullable(self, v: impl Into<Primitive<bool>>) -> Self {
        self.0.data.borrow_mut().nullable = v.into();
        self
    }

    pub fn set_sensitive(self, v: impl Into<Primitive<bool>>) -> Self {
        self.0.data.borrow_mut().sensitive = v.into();
        self
    }
}

impl<T: PrimitiveType> Into<Primitive<T>> for &Variable<T> {
    fn into(self) -> Primitive<T> {
        Primitive::Sentinel(self.0.sentinel.clone())
    }
}

impl<T: PrimitiveType> ToString for &Variable<T> {
    fn to_string(&self) -> String {
        self.0.sentinel.clone()
    }
}

pub struct BuildVariable {
    pub tf_id: String,
}

impl BuildVariable {
    pub fn build<T: PrimitiveType + 'static>(self, stack: &mut Stack) -> Variable<T> {
        let out = Variable(Rc::new(Variable_ {
            sentinel: stack.add_sentinel(format!("${{var.{}}}", self.tf_id)),
            tf_id: self.tf_id,
            data: RefCell::new(VariableImplData {
                r#type: T::extract_variable_type(),
                nullable: false.into(),
                sensitive: false.into(),
            }),
            _p: Default::default(),
        }));
        stack.variables.push(out.0.clone());
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
        Primitive::Sentinel(format!("${{variable.{}}}", self.tf_id))
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
    };
}

// Functions
pub fn tf_base64encode(e: impl TfExpr) -> PrimExpr {
    PrimExpr(format!("base64encode({})", e.raw()))
}

pub fn tf_base64decode(e: impl TfExpr) -> PrimExpr {
    PrimExpr(format!("base64decode({})", e.raw()))
}
