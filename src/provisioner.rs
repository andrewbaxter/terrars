//! Support for Terraform [resource
//! provisioners](https://developer.hashicorp.com/terraform/language/resources/provisioners/syntax).
//!
//! Build one of the concrete provisioner types (ex: [`BuildProvisionerLocalExec`])
//! and attach it to a resource with the generated `add_provisioner` method. All
//! provisioner fields go through the usual field types so you can interpolate
//! references from other resources.
use crate::{
    ListField,
    PrimField,
    RecField,
};

use serde::{
    ser::SerializeMap,
    Serialize,
};

erased_serde::serialize_trait_object!(ProvisionerType);

/// Implemented by each concrete provisioner (`local-exec`, `remote-exec`, `file`).
/// Serializes as the body of the provisioner block; the block key comes from
/// [`ProvisionerType::provisioner_type`].
pub trait ProvisionerType: erased_serde::Serialize {
    /// The Terraform provisioner type, ex: `local-exec`.
    fn provisioner_type(&self) -> &'static str;
}

/// Build a [`ProvisionerConnection`].
pub struct BuildProvisionerConnection {}

impl BuildProvisionerConnection {
    pub fn build(self) -> ProvisionerConnection {
        ProvisionerConnection::default()
    }
}

/// Build a [`ProvisionerFile`]. Set exactly one of `source` or `content`.
pub struct BuildProvisionerFile {
    /// Path on the remote host to write to.
    pub destination: PrimField<String>,
}

impl BuildProvisionerFile {
    pub fn build(self) -> ProvisionerFile {
        ProvisionerFile {
            destination: self.destination,
            source: None,
            content: None,
            connection: None,
            when: None,
            on_failure: None,
        }
    }
}

/// Build a [`ProvisionerLocalExec`].
pub struct BuildProvisionerLocalExec {
    /// The command to run.
    pub command: PrimField<String>,
}

impl BuildProvisionerLocalExec {
    pub fn build(self) -> ProvisionerLocalExec {
        ProvisionerLocalExec {
            command: self.command,
            working_dir: None,
            interpreter: None,
            environment: None,
            when: None,
            on_failure: None,
            quiet: None,
        }
    }
}

/// Build a [`ProvisionerRemoteExec`]. Set exactly one of `inline`, `script`, or
/// `scripts`, and generally a `connection`.
pub struct BuildProvisionerRemoteExec {}

impl BuildProvisionerRemoteExec {
    pub fn build(self) -> ProvisionerRemoteExec {
        ProvisionerRemoteExec {
            inline: None,
            script: None,
            scripts: None,
            connection: None,
            when: None,
            on_failure: None,
        }
    }
}

/// A provisioner attached to a resource. Produced by converting one of the
/// concrete provisioner types (ex: [`ProvisionerLocalExec`]) via `.into()`, which
/// generated `add_provisioner` methods do for you.
///
/// Serializes to a single-key object `{ "<type>": { <body> } }` as expected in the
/// resource's `provisioner` array.
pub struct Provisioner(Box<dyn ProvisionerType>);

impl From<ProvisionerFile> for Provisioner {
    fn from(v: ProvisionerFile) -> Provisioner {
        Provisioner(Box::new(v))
    }
}

impl From<ProvisionerLocalExec> for Provisioner {
    fn from(v: ProvisionerLocalExec) -> Provisioner {
        Provisioner(Box::new(v))
    }
}

impl From<ProvisionerRemoteExec> for Provisioner {
    fn from(v: ProvisionerRemoteExec) -> Provisioner {
        Provisioner(Box::new(v))
    }
}

impl Serialize for Provisioner {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        let mut m = serializer.serialize_map(Some(1))?;
        m.serialize_entry(self.0.provisioner_type(), &self.0)?;
        m.end()
    }
}

/// A `connection` block, used by the `remote-exec` and `file` provisioners to
/// describe how to reach the remote host.
#[derive(Serialize, Default)]
pub struct ProvisionerConnection {
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<PrimField<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    certificate: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<PrimField<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    private_key: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<PrimField<String>>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    type_: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<PrimField<String>>,
}

impl ProvisionerConnection {
    pub fn set_agent(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.agent = Some(v.into());
        self
    }

    pub fn set_certificate(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.certificate = Some(v.into());
        self
    }

    pub fn set_host(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.host = Some(v.into());
        self
    }

    pub fn set_password(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.password = Some(v.into());
        self
    }

    pub fn set_port(mut self, v: impl Into<PrimField<i64>>) -> Self {
        self.port = Some(v.into());
        self
    }

    pub fn set_private_key(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.private_key = Some(v.into());
        self
    }

    pub fn set_timeout(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.timeout = Some(v.into());
        self
    }

    /// Connection type, ex: `ssh` (default) or `winrm`.
    pub fn set_type(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.type_ = Some(v.into());
        self
    }

    pub fn set_user(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.user = Some(v.into());
        self
    }
}

/// The
/// [`file`](https://developer.hashicorp.com/terraform/language/resources/provisioners/file)
/// provisioner, which copies files or directories to the remote resource.
#[derive(Serialize)]
pub struct ProvisionerFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    connection: Option<ProvisionerConnection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<PrimField<String>>,
    destination: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_failure: Option<ProvisionerOnFailure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    when: Option<ProvisionerWhen>,
}

impl ProvisionerFile {
    pub fn set_connection(mut self, v: ProvisionerConnection) -> Self {
        self.connection = Some(v);
        self
    }

    /// Inline content to write to the destination.
    pub fn set_content(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.content = Some(v.into());
        self
    }

    pub fn set_on_failure(mut self, v: ProvisionerOnFailure) -> Self {
        self.on_failure = Some(v);
        self
    }

    /// Local path of the file/directory to copy.
    pub fn set_source(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.source = Some(v.into());
        self
    }

    pub fn set_when(mut self, v: ProvisionerWhen) -> Self {
        self.when = Some(v);
        self
    }
}

impl ProvisionerType for ProvisionerFile {
    fn provisioner_type(&self) -> &'static str {
        "file"
    }
}

/// The
/// [`local-exec`](https://developer.hashicorp.com/terraform/language/resources/provisioners/local-exec)
/// provisioner, which runs a command on the machine running Terraform.
#[derive(Serialize)]
pub struct ProvisionerLocalExec {
    command: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<RecField<PrimField<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    interpreter: Option<ListField<PrimField<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_failure: Option<ProvisionerOnFailure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quiet: Option<PrimField<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    when: Option<ProvisionerWhen>,
    #[serde(skip_serializing_if = "Option::is_none")]
    working_dir: Option<PrimField<String>>,
}

impl ProvisionerLocalExec {
    /// Environment variables to set for the command.
    pub fn set_environment(mut self, v: impl Into<RecField<PrimField<String>>>) -> Self {
        self.environment = Some(v.into());
        self
    }

    /// Interpreter + flags to run the command with, ex: `["/bin/bash", "-c"]`.
    pub fn set_interpreter(mut self, v: impl Into<ListField<PrimField<String>>>) -> Self {
        self.interpreter = Some(v.into());
        self
    }

    pub fn set_on_failure(mut self, v: ProvisionerOnFailure) -> Self {
        self.on_failure = Some(v);
        self
    }

    /// Suppress printing the command being run.
    pub fn set_quiet(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.quiet = Some(v.into());
        self
    }

    pub fn set_when(mut self, v: ProvisionerWhen) -> Self {
        self.when = Some(v);
        self
    }

    /// Directory to run the command in (defaults to the working directory).
    pub fn set_working_dir(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.working_dir = Some(v.into());
        self
    }
}

impl ProvisionerType for ProvisionerLocalExec {
    fn provisioner_type(&self) -> &'static str {
        "local-exec"
    }
}

/// What to do if a provisioner fails.
#[derive(Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProvisionerOnFailure {
    /// Ignore the error and continue.
    Continue,
    /// Abort the apply (and mark the resource tainted). This is the default.
    Fail,
}

/// The
/// [`remote-exec`](https://developer.hashicorp.com/terraform/language/resources/provisioners/remote-exec)
/// provisioner, which runs a command on the remote resource after it's created.
#[derive(Serialize)]
pub struct ProvisionerRemoteExec {
    #[serde(skip_serializing_if = "Option::is_none")]
    connection: Option<ProvisionerConnection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inline: Option<ListField<PrimField<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_failure: Option<ProvisionerOnFailure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    script: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scripts: Option<ListField<PrimField<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    when: Option<ProvisionerWhen>,
}

impl ProvisionerRemoteExec {
    pub fn set_connection(mut self, v: ProvisionerConnection) -> Self {
        self.connection = Some(v);
        self
    }

    /// A list of commands to run inline.
    pub fn set_inline(mut self, v: impl Into<ListField<PrimField<String>>>) -> Self {
        self.inline = Some(v.into());
        self
    }

    pub fn set_on_failure(mut self, v: ProvisionerOnFailure) -> Self {
        self.on_failure = Some(v);
        self
    }

    /// Path to a local script to copy and run on the remote host.
    pub fn set_script(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.script = Some(v.into());
        self
    }

    /// Paths to local scripts to copy and run on the remote host.
    pub fn set_scripts(mut self, v: impl Into<ListField<PrimField<String>>>) -> Self {
        self.scripts = Some(v.into());
        self
    }

    pub fn set_when(mut self, v: ProvisionerWhen) -> Self {
        self.when = Some(v);
        self
    }
}

impl ProvisionerType for ProvisionerRemoteExec {
    fn provisioner_type(&self) -> &'static str {
        "remote-exec"
    }
}

/// When a provisioner should run. Defaults to at creation time.
#[derive(Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProvisionerWhen {
    Create,
    Destroy,
}
