use {
    crate::PrimField,
    serde::{
        Serialize,
        ser::SerializeMap,
    },
    serde_json::Value,
    std::collections::BTreeMap,
};

impl From<BackendLocal> for Backend {
    fn from(v: BackendLocal) -> Backend {
        Backend(Box::new(v))
    }
}

impl From<BackendS3> for Backend {
    fn from(v: BackendS3) -> Backend {
        Backend(Box::new(v))
    }
}

impl From<BackendGcs> for Backend {
    fn from(v: BackendGcs) -> Backend {
        Backend(Box::new(v))
    }
}

impl From<BackendAzurerm> for Backend {
    fn from(v: BackendAzurerm) -> Backend {
        Backend(Box::new(v))
    }
}

impl From<BackendHttp> for Backend {
    fn from(v: BackendHttp) -> Backend {
        Backend(Box::new(v))
    }
}

impl From<BackendPg> for Backend {
    fn from(v: BackendPg) -> Backend {
        Backend(Box::new(v))
    }
}

impl From<BackendKubernetes> for Backend {
    fn from(v: BackendKubernetes) -> Backend {
        Backend(Box::new(v))
    }
}

impl From<BackendConsul> for Backend {
    fn from(v: BackendConsul) -> Backend {
        Backend(Box::new(v))
    }
}

impl From<BackendRemote> for Backend {
    fn from(v: BackendRemote) -> Backend {
        Backend(Box::new(v))
    }
}

impl From<BackendCustom> for Backend {
    fn from(v: BackendCustom) -> Backend {
        Backend(Box::new(v))
    }
}

erased_serde::serialize_trait_object!(BackendType);

pub trait BackendType: erased_serde::Serialize {
    fn backend_type(&self) -> &str;
}

pub struct Backend(Box<dyn BackendType>);

impl Serialize for Backend {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        let mut m = serializer.serialize_map(Some(1))?;
        m.serialize_entry(self.0.backend_type(), &self.0)?;
        m.end()
    }
}

#[derive(Serialize)]
pub struct BackendAzurerm {
    #[serde(skip_serializing_if = "Option::is_none")]
    access_key: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_secret: Option<PrimField<String>>,
    container_name: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<PrimField<String>>,
    key: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource_group_name: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sas_token: Option<PrimField<String>>,
    storage_account_name: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subscription_id: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tenant_id: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_azuread_auth: Option<PrimField<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_msi: Option<PrimField<bool>>,
}

impl BackendAzurerm {
    pub fn set_resource_group_name(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.resource_group_name = Some(v.into());
        self
    }

    pub fn set_subscription_id(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.subscription_id = Some(v.into());
        self
    }

    pub fn set_tenant_id(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.tenant_id = Some(v.into());
        self
    }

    pub fn set_client_id(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.client_id = Some(v.into());
        self
    }

    pub fn set_client_secret(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.client_secret = Some(v.into());
        self
    }

    pub fn set_access_key(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.access_key = Some(v.into());
        self
    }

    pub fn set_sas_token(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.sas_token = Some(v.into());
        self
    }

    pub fn set_use_azuread_auth(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.use_azuread_auth = Some(v.into());
        self
    }

    pub fn set_use_msi(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.use_msi = Some(v.into());
        self
    }

    pub fn set_environment(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.environment = Some(v.into());
        self
    }
}

impl BackendType for BackendAzurerm {
    fn backend_type(&self) -> &str {
        "azurerm"
    }
}

#[derive(Serialize)]
pub struct BackendConsul {
    #[serde(skip_serializing_if = "Option::is_none")]
    access_token: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    datacenter: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gzip: Option<PrimField<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lock: Option<PrimField<bool>>,
    path: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scheme: Option<PrimField<String>>,
}

impl BackendConsul {
    pub fn set_address(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.address = Some(v.into());
        self
    }

    pub fn set_scheme(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.scheme = Some(v.into());
        self
    }

    pub fn set_access_token(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.access_token = Some(v.into());
        self
    }

    pub fn set_datacenter(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.datacenter = Some(v.into());
        self
    }

    pub fn set_gzip(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.gzip = Some(v.into());
        self
    }

    pub fn set_lock(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.lock = Some(v.into());
        self
    }
}

impl BackendType for BackendConsul {
    fn backend_type(&self) -> &str {
        "consul"
    }
}

pub struct BackendCustom {
    config: BTreeMap<String, Value>,
    type_: String,
}

impl BackendCustom {
    pub fn new(backend_type: impl Into<String>) -> Self {
        BackendCustom {
            type_: backend_type.into(),
            config: BTreeMap::new(),
        }
    }

    pub fn set(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.config.insert(key.into(), value.into());
        self
    }
}

impl BackendType for BackendCustom {
    fn backend_type(&self) -> &str {
        &self.type_
    }
}

impl Serialize for BackendCustom {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        self.config.serialize(serializer)
    }
}

#[derive(Serialize)]
pub struct BackendGcs {
    bucket: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    credentials: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encryption_key: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    impersonate_service_account: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kms_encryption_key: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prefix: Option<PrimField<String>>,
}

impl BackendGcs {
    pub fn set_prefix(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.prefix = Some(v.into());
        self
    }

    pub fn set_credentials(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.credentials = Some(v.into());
        self
    }

    pub fn set_impersonate_service_account(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.impersonate_service_account = Some(v.into());
        self
    }

    pub fn set_encryption_key(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.encryption_key = Some(v.into());
        self
    }

    pub fn set_kms_encryption_key(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.kms_encryption_key = Some(v.into());
        self
    }
}

impl BackendType for BackendGcs {
    fn backend_type(&self) -> &str {
        "gcs"
    }
}

#[derive(Serialize)]
pub struct BackendHttp {
    address: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lock_address: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lock_method: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_max: Option<PrimField<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_cert_verification: Option<PrimField<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unlock_address: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unlock_method: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    update_method: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<PrimField<String>>,
}

impl BackendHttp {
    pub fn set_update_method(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.update_method = Some(v.into());
        self
    }

    pub fn set_lock_address(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.lock_address = Some(v.into());
        self
    }

    pub fn set_lock_method(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.lock_method = Some(v.into());
        self
    }

    pub fn set_unlock_address(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.unlock_address = Some(v.into());
        self
    }

    pub fn set_unlock_method(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.unlock_method = Some(v.into());
        self
    }

    pub fn set_username(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.username = Some(v.into());
        self
    }

    pub fn set_password(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.password = Some(v.into());
        self
    }

    pub fn set_skip_cert_verification(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.skip_cert_verification = Some(v.into());
        self
    }

    pub fn set_retry_max(mut self, v: impl Into<PrimField<i64>>) -> Self {
        self.retry_max = Some(v.into());
        self
    }
}

impl BackendType for BackendHttp {
    fn backend_type(&self) -> &str {
        "http"
    }
}

#[derive(Serialize)]
pub struct BackendKubernetes {
    #[serde(skip_serializing_if = "Option::is_none")]
    config_context: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    config_path: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    in_cluster_config: Option<PrimField<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<PrimField<String>>,
    secret_suffix: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<PrimField<String>>,
}

impl BackendKubernetes {
    pub fn set_namespace(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.namespace = Some(v.into());
        self
    }

    pub fn set_config_path(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.config_path = Some(v.into());
        self
    }

    pub fn set_config_context(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.config_context = Some(v.into());
        self
    }

    pub fn set_in_cluster_config(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.in_cluster_config = Some(v.into());
        self
    }

    pub fn set_host(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.host = Some(v.into());
        self
    }

    pub fn set_token(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.token = Some(v.into());
        self
    }
}

impl BackendType for BackendKubernetes {
    fn backend_type(&self) -> &str {
        "kubernetes"
    }
}

#[derive(Serialize, Default)]
pub struct BackendLocal {
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_dir: Option<PrimField<String>>,
}

impl BackendLocal {
    pub fn set_path(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.path = Some(v.into());
        self
    }

    pub fn set_workspace_dir(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.workspace_dir = Some(v.into());
        self
    }
}

impl BackendType for BackendLocal {
    fn backend_type(&self) -> &str {
        "local"
    }
}

#[derive(Serialize)]
pub struct BackendPg {
    conn_str: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_name: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_index_creation: Option<PrimField<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_schema_creation: Option<PrimField<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_table_creation: Option<PrimField<bool>>,
}

impl BackendPg {
    pub fn set_schema_name(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.schema_name = Some(v.into());
        self
    }

    pub fn set_skip_schema_creation(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.skip_schema_creation = Some(v.into());
        self
    }

    pub fn set_skip_table_creation(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.skip_table_creation = Some(v.into());
        self
    }

    pub fn set_skip_index_creation(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.skip_index_creation = Some(v.into());
        self
    }
}

impl BackendType for BackendPg {
    fn backend_type(&self) -> &str {
        "pg"
    }
}

#[derive(Serialize)]
pub struct BackendRemote {
    #[serde(skip_serializing_if = "Option::is_none")]
    hostname: Option<PrimField<String>>,
    organization: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<PrimField<String>>,
    workspaces: RemoteWorkspaces,
}

impl BackendRemote {
    pub fn set_hostname(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.hostname = Some(v.into());
        self
    }

    pub fn set_token(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.token = Some(v.into());
        self
    }
}

impl BackendType for BackendRemote {
    fn backend_type(&self) -> &str {
        "remote"
    }
}

#[derive(Serialize)]
pub struct BackendS3 {
    #[serde(skip_serializing_if = "Option::is_none")]
    access_key: Option<PrimField<String>>,
    bucket: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dynamodb_table: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encrypt: Option<PrimField<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    endpoint: Option<PrimField<String>>,
    key: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kms_key_id: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role_arn: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secret_key: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_credentials_validation: Option<PrimField<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_region_validation: Option<PrimField<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_requesting_account_id: Option<PrimField<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_lockfile: Option<PrimField<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_path_style: Option<PrimField<bool>>,
}

impl BackendS3 {
    pub fn set_region(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.region = Some(v.into());
        self
    }

    pub fn set_profile(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.profile = Some(v.into());
        self
    }

    pub fn set_encrypt(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.encrypt = Some(v.into());
        self
    }

    pub fn set_kms_key_id(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.kms_key_id = Some(v.into());
        self
    }

    pub fn set_dynamodb_table(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.dynamodb_table = Some(v.into());
        self
    }

    pub fn set_use_lockfile(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.use_lockfile = Some(v.into());
        self
    }

    pub fn set_role_arn(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.role_arn = Some(v.into());
        self
    }

    pub fn set_access_key(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.access_key = Some(v.into());
        self
    }

    pub fn set_secret_key(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.secret_key = Some(v.into());
        self
    }

    pub fn set_endpoint(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.endpoint = Some(v.into());
        self
    }

    pub fn set_use_path_style(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.use_path_style = Some(v.into());
        self
    }

    pub fn set_skip_credentials_validation(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.skip_credentials_validation = Some(v.into());
        self
    }

    pub fn set_skip_region_validation(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.skip_region_validation = Some(v.into());
        self
    }

    pub fn set_skip_requesting_account_id(mut self, v: impl Into<PrimField<bool>>) -> Self {
        self.skip_requesting_account_id = Some(v.into());
        self
    }
}

impl BackendType for BackendS3 {
    fn backend_type(&self) -> &str {
        "s3"
    }
}

pub struct BuildBackendAzurerm {
    pub container_name: PrimField<String>,
    pub key: PrimField<String>,
    pub storage_account_name: PrimField<String>,
}

impl BuildBackendAzurerm {
    pub fn build(self) -> BackendAzurerm {
        BackendAzurerm {
            storage_account_name: self.storage_account_name,
            container_name: self.container_name,
            key: self.key,
            resource_group_name: None,
            subscription_id: None,
            tenant_id: None,
            client_id: None,
            client_secret: None,
            access_key: None,
            sas_token: None,
            use_azuread_auth: None,
            use_msi: None,
            environment: None,
        }
    }
}

pub struct BuildBackendConsul {
    pub path: PrimField<String>,
}

impl BuildBackendConsul {
    pub fn build(self) -> BackendConsul {
        BackendConsul {
            path: self.path,
            address: None,
            scheme: None,
            access_token: None,
            datacenter: None,
            gzip: None,
            lock: None,
        }
    }
}

pub struct BuildBackendGcs {
    pub bucket: PrimField<String>,
}

impl BuildBackendGcs {
    pub fn build(self) -> BackendGcs {
        BackendGcs {
            bucket: self.bucket,
            prefix: None,
            credentials: None,
            impersonate_service_account: None,
            encryption_key: None,
            kms_encryption_key: None,
        }
    }
}

pub struct BuildBackendHttp {
    pub address: PrimField<String>,
}

impl BuildBackendHttp {
    pub fn build(self) -> BackendHttp {
        BackendHttp {
            address: self.address,
            update_method: None,
            lock_address: None,
            lock_method: None,
            unlock_address: None,
            unlock_method: None,
            username: None,
            password: None,
            skip_cert_verification: None,
            retry_max: None,
        }
    }
}

pub struct BuildBackendKubernetes {
    pub secret_suffix: PrimField<String>,
}

impl BuildBackendKubernetes {
    pub fn build(self) -> BackendKubernetes {
        BackendKubernetes {
            secret_suffix: self.secret_suffix,
            namespace: None,
            config_path: None,
            config_context: None,
            in_cluster_config: None,
            host: None,
            token: None,
        }
    }
}

pub struct BuildBackendPg {
    pub conn_str: PrimField<String>,
}

impl BuildBackendPg {
    pub fn build(self) -> BackendPg {
        BackendPg {
            conn_str: self.conn_str,
            schema_name: None,
            skip_schema_creation: None,
            skip_table_creation: None,
            skip_index_creation: None,
        }
    }
}

pub struct BuildBackendRemote {
    pub organization: PrimField<String>,
    pub workspaces: RemoteWorkspaces,
}

impl BuildBackendRemote {
    pub fn build(self) -> BackendRemote {
        BackendRemote {
            organization: self.organization,
            workspaces: self.workspaces,
            hostname: None,
            token: None,
        }
    }
}

pub struct BuildBackendS3 {
    pub bucket: PrimField<String>,
    pub key: PrimField<String>,
}

impl BuildBackendS3 {
    pub fn build(self) -> BackendS3 {
        BackendS3 {
            bucket: self.bucket,
            key: self.key,
            region: None,
            profile: None,
            encrypt: None,
            kms_key_id: None,
            dynamodb_table: None,
            use_lockfile: None,
            role_arn: None,
            access_key: None,
            secret_key: None,
            endpoint: None,
            use_path_style: None,
            skip_credentials_validation: None,
            skip_region_validation: None,
            skip_requesting_account_id: None,
        }
    }
}

pub struct BuildCloud {
    pub organization: PrimField<String>,
    pub workspaces: CloudWorkspaces,
}

impl BuildCloud {
    pub fn build(self) -> Cloud {
        Cloud {
            organization: self.organization,
            workspaces: self.workspaces,
            hostname: None,
            token: None,
        }
    }
}

#[derive(Serialize)]
pub struct Cloud {
    #[serde(skip_serializing_if = "Option::is_none")]
    hostname: Option<PrimField<String>>,
    organization: PrimField<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<PrimField<String>>,
    workspaces: CloudWorkspaces,
}

impl Cloud {
    pub fn set_hostname(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.hostname = Some(v.into());
        self
    }

    pub fn set_token(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.token = Some(v.into());
        self
    }
}

#[derive(Serialize, Default)]
pub struct CloudWorkspaces {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<PrimField<String>>>,
}

impl CloudWorkspaces {
    pub fn set_tags(mut self, v: Vec<PrimField<String>>) -> Self {
        self.tags = Some(v);
        self
    }

    pub fn set_name(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.name = Some(v.into());
        self
    }

    pub fn set_project(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.project = Some(v.into());
        self
    }
}

#[derive(Serialize, Default)]
pub struct RemoteWorkspaces {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<PrimField<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prefix: Option<PrimField<String>>,
}

impl RemoteWorkspaces {
    pub fn set_name(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.name = Some(v.into());
        self
    }

    pub fn set_prefix(mut self, v: impl Into<PrimField<String>>) -> Self {
        self.prefix = Some(v.into());
        self
    }
}
