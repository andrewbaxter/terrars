use std::{
    cell::{
        RefCell,
    },
    collections::{
        BTreeMap,
        HashMap,
    },
    fs::{
        self,
        create_dir_all,
    },
    io::{
        self,
        Write,
    },
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

pub(crate) mod utils;
pub mod expr;
pub mod func;
pub mod list_field;
pub mod list_ref;
pub mod rec_field;
pub mod rec_ref;
pub mod output;
pub mod prim_field;
pub mod prim_ref;
pub mod set_field;
pub mod set_ref;
pub mod variable;

pub use expr::*;
pub use func::*;
pub use list_field::*;
pub use list_ref::*;
pub use rec_field::*;
pub use rec_ref::*;
pub use output::*;
pub use prim_field::*;
pub use prim_ref::*;
pub use set_field::*;
pub use set_ref::*;
pub use utils::*;
pub use variable::*;

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
            shared: StackShared(Rc::new(RefCell::new(StackShared_ { replace_exprs: Default::default() }))),
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

struct StackShared_ {
    replace_exprs: Vec<(String, String)>,
}

#[derive(Clone)]
pub struct StackShared(Rc<RefCell<StackShared_>>);

impl StackShared {
    pub fn add_sentinel(&self, v: &str) -> String {
        let mut m = self.0.borrow_mut();
        let k = format!("_TERRARS_SENTINEL_{}_", m.replace_exprs.len());
        m.replace_exprs.push((k.clone(), format!("${{{}}}", v)));
        k
    }
}

pub struct Stack {
    provider_types: Vec<Rc<dyn ProviderType>>,
    providers: Vec<Rc<dyn Provider>>,
    variables: Vec<Rc<dyn VariableTrait>>,
    datasources: Vec<Rc<dyn Datasource_>>,
    resources: Vec<Rc<dyn Resource_>>,
    outputs: Vec<Rc<dyn Output>>,
    pub shared: StackShared,
}

impl Stack {
    pub fn str_expr(&self, expr: impl ToString) -> PrimExpr<String> {
        PrimExpr(self.shared.clone(), expr.to_string(), Default::default())
    }

    pub fn expr<T: PrimType>(&self, expr: impl ToString) -> PrimExpr<T> {
        PrimExpr(self.shared.clone(), expr.to_string(), Default::default())
    }

    pub fn string(&self, val: impl ToString) -> PrimExpr<String> {
        PrimExpr(self.shared.clone(), format!("\"{}\"", val.to_string().replace("\"", "\\\"")), Default::default())
    }

    pub fn bool(&self, val: bool) -> PrimExpr<bool> {
        PrimExpr(self.shared.clone(), if val {
            "true"
        } else {
            "false"
        }.into(), Default::default())
    }

    pub fn i64(&self, val: i64) -> PrimExpr<i64> {
        PrimExpr(self.shared.clone(), val.to_string(), Default::default())
    }

    pub fn f64(&self, val: f64) -> PrimExpr<f64> {
        PrimExpr(self.shared.clone(), val.to_string(), Default::default())
    }

    /// Convert the stack to json bytes.
    pub fn serialize(&self, state_path: &Path) -> Result<Vec<u8>, StackError> {
        REPLACE_EXPRS.with(move |f| {
            *f.borrow_mut() = Some(self.shared.0.borrow().replace_exprs.clone());
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

// Primitives Generated traits
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_changes: Option<IgnoreChanges>,
    pub replace_triggered_by: Vec<String>,
}
