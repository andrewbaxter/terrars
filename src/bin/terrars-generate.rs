pub mod generatelib;

use {
    aargvark::{
        Aargvark,
        AargvarkJson,
        vark,
    },
    crate::generatelib::{
        generate::{
            TopLevelFields,
            generate_block_fields,
            generate_fields_from_value_map,
            to_camel,
            to_snake,
        },
        sourceschema::ProviderSchemas,
    },
    loga::{
        DebugDisplay,
        ResultContext,
        ea,
        fatal,
    },
    proc_macro2::{
        Ident,
        TokenStream,
    },
    quote::{
        format_ident,
        quote,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    serde_json::json,
    std::{
        collections::HashSet,
        fs::{
            File,
            create_dir_all,
            remove_dir_all,
            self,
        },
        io::Write,
        path::{
            Path,
            PathBuf,
        },
        process::Command,
    },
};

pub trait CollCommand {
    fn run(&mut self) -> Result<(), loga::Error>;
}

impl CollCommand for Command {
    fn run(&mut self) -> Result<(), loga::Error> {
        match self.output() {
            Ok(o) => {
                if o.status.success() {
                    Ok(())
                } else {
                    Err(loga::err_with("Exit code indicated error", ea!(status = o.dbg_str())))
                }
            },
            Err(e) => Err(e.into()),
        }.context_with("Failed to run", ea!(command = self.dbg_str()))?;
        return Ok(());
    }
}

fn main() {
    match es!({
        #[derive(Serialize, Deserialize)]
        struct Config {
            provider: String,
            version: String,
            include: Option<Vec<String>>,
            exclude: Option<Vec<String>>,
            dest: PathBuf,
            feature_gate: Option<PathBuf>,
            binary: Option<String>,
        }

        #[derive(Aargvark)]
        struct Arguments {
            /// Path to terrars config jsons.
            configs: Vec<AargvarkJson<Config>>,
            /// Save the provider json in this dir (debug helper).
            dump: Option<()>,
        }

        let args = vark::<Arguments>();
        if args.configs.is_empty() {
            return Err(loga::err("No configs specified; nothing to do"));
        }
        for config in args.configs {
            let config = config.value;
            let (vendor, shortname) =
                config.provider.split_once("/").unwrap_or_else(|| ("hashicorp".into(), &config.provider));
            let provider_prefix = format!("{}_", shortname);
            let mut include: HashSet<&String> = config.include.iter().flatten().collect();
            let mut exclude: HashSet<&String> = config.exclude.iter().flatten().collect();
            let whitelist = !include.is_empty();

            // Feature output
            let mut features = vec![];

            // Get provider schema
            let dir = tempfile::tempdir()?;
            fs::write(dir.path().join("providers.tf.json"), &serde_json::to_vec(&json!({
                "terraform": {
                    "required_providers": {
                        shortname: {
                            "source": config.provider,
                            "version": config.version,
                        }
                    }
                }
            })).unwrap()).context("Failed to write bootstrap terraform code for provider schema extraction")?;
            let binary = config.binary.as_deref().unwrap_or("terraform");
            Command::new(binary)
                .args(&["init", "-no-color"])
                .current_dir(&dir)
                .run()
                .context("Error initializing terraform in export dir")?;
            let schema_raw =
                Command::new(binary)
                    .args(&["providers", "schema", "-json", "-no-color"])
                    .current_dir(&dir)
                    .output()
                    .context("Error outputting terraform provider schema")?
                    .stdout;
            if args.dump.is_some() {
                fs::write("dump.json", &schema_raw)?;
            }
            let schema: ProviderSchemas =
                serde_json::from_slice(&schema_raw).context("Error parsing provider schema json from terraform")?;

            // Generate
            fn write_file(path: &Path, contents: Vec<TokenStream>) -> Result<(), loga::Error> {
                es!({
                    File::create(&path)
                        .context("Failed to create rust file")?
                        .write_all(
                            genemichaels_lib::format_ast(
                                syn::parse2::<syn::File>(
                                    quote!(#(#contents) *),
                                ).context_with(
                                    "Failed to parse generated code AST for formatting",
                                    ea!(
                                        context =
                                            contents
                                                .iter()
                                                .map(|s| s.to_string())
                                                .collect::<Vec<String>>()
                                                .join("\n")
                                                .lines()
                                                .enumerate()
                                                .map(|(ln, l)| format!("{:0>4} {}", ln + 1, l))
                                                .collect::<Vec<String>>()
                                                .join("\n")
                                    ),
                                )?,
                                &genemichaels_lib::FormatConfig::default(),
                                Default::default(),
                            )
                                .map_err(|e| loga::err_with("Error formatting generated code", ea!(err = e)))?
                                .rendered
                                .as_bytes(),
                        )
                        .context("Failed to write rust file")?;
                    Ok(())
                }).context_with("Failed to write generate code", ea!(path = path.to_string_lossy()))?;
                Ok(())
            }

            fn rustfile_template(trait_imports: &[&str]) -> Vec<TokenStream> {
                let imports = if trait_imports.is_empty() {
                    quote!()
                } else {
                    let idents = trait_imports.iter().map(|t| format_ident!("{}", t));
                    quote!(use terrars:: {
                        #(#idents as _,) *
                    };)
                };
                return vec![quote!(
                    use serde::Serialize;
                    #imports
                )];
            }

            // Provider type + provider
            let provider_schema = {
                let key = format!("registry.terraform.io/{}/{}", vendor, shortname);
                schema
                    .provider_schemas
                    .get(&key)
                    .context_with("Missing provider schema for listed provider", ea!(provider = config.provider))?
            };
            let provider_name_parts = &shortname.split("-").map(ToString::to_string).collect::<Vec<String>>();
            let provider_dir = config.dest;
            if provider_dir.exists() {
                remove_dir_all(&provider_dir)?;
            }
            create_dir_all(&provider_dir)?;
            let mut mod_out = vec![];
            let provider_ident: Ident;
            {
                let camel_name = to_camel(provider_name_parts);
                let source = &config.provider;
                let version = &config.version;
                let provider_inner_mut_ident = format_ident!("Provider{}Data", camel_name);
                let mut raw_fields = TopLevelFields::default();
                generate_fields_from_value_map(
                    &mut raw_fields,
                    &provider_name_parts,
                    &provider_schema.provider.block.attributes,
                    true,
                );
                let mut out = rustfile_template(if raw_fields.extra_types.is_empty() {
                    &[]
                } else {
                    &["Ref"]
                });
                let builder_fields = raw_fields.builder_fields;
                let copy_builder_fields = raw_fields.copy_builder_fields;
                let extra_types = raw_fields.extra_types;
                let provider_fields = raw_fields.fields;
                let provider_mut_methods = raw_fields.mut_methods;
                provider_ident = format_ident!("Provider{}", camel_name);
                let provider_inner_ident = format_ident!("Provider{}_", camel_name);
                let provider_builder_ident = format_ident!("BuildProvider{}", camel_name);
                out.push(quote!{
                    #[derive(Serialize)] struct #provider_inner_mut_ident {
                        #[serde(skip_serializing_if = "std::option::Option::is_none")] alias: std::option::Option < std::string::String >,
                        #(#provider_fields,) *
                    }
                    struct #provider_inner_ident {
                        data: std:: cell:: RefCell < #provider_inner_mut_ident >,
                    }
                    pub struct #provider_ident(std:: rc:: Rc < #provider_inner_ident >);
                    impl #provider_ident {
                        pub fn provider_ref(&self) -> std::string::String {
                            let data = self.0.data.borrow();
                            if let std::option::Option::Some(alias) = &data.alias {
                                format!("{}.{}", #shortname, alias)
                            }
                            else {
                                #shortname.into()
                            }
                        }
                        pub fn set_alias(self, alias: impl std::string::ToString) -> Self {
                            self.0.data.borrow_mut().alias = std::option::Option::Some(alias.to_string());
                            self
                        }
                        #(#provider_mut_methods) *
                    }
                    impl terrars:: Provider for #provider_inner_ident {
                        fn extract_type_tf_id(&self) -> std::string::String {
                            #shortname.into()
                        }
                        fn extract_provider_type(&self) -> serde_json::Value {
                            serde_json::json!({
                                "source": #source,
                                "version": #version,
                            })
                        }
                        fn extract_provider(&self) -> serde_json::Value {
                            serde_json::to_value(&self.data).unwrap()
                        }
                    }
                    pub struct #provider_builder_ident {
                        #(#builder_fields,) *
                    }
                    impl #provider_builder_ident {
                        pub fn build(self, stack:& mut terrars:: Stack) -> #provider_ident {
                            let out = #provider_ident(std:: rc:: Rc:: new(#provider_inner_ident {
                                data: std:: cell:: RefCell:: new(#provider_inner_mut_ident {
                                    alias: std::option::Option::None,
                                    #(#copy_builder_fields,) *
                                }),
                            }));
                            stack.add_provider(out.0.clone());
                            out
                        }
                    }
                    #(#extra_types) *
                });
                write_file(&provider_dir.join("provider.rs"), out)?;
                let path_ident = format_ident!("provider");
                mod_out.push(quote!(pub mod #path_ident; pub use #path_ident::*;));
            }

            // Resources
            for (resource_name, resource) in &provider_schema.resource_schemas {
                let mut out = rustfile_template(&["Referable", "Resource_", "Ref"]);
                out.push(quote!(use super:: provider:: #provider_ident;));
                let use_name_parts =
                    resource_name
                        .strip_prefix(&provider_prefix)
                        .context_with(
                            "Name missing expected provider prefix",
                            ea!(resource = resource_name, prefix = provider_prefix),
                        )?
                        .split("_")
                        .map(ToString::to_string)
                        .collect::<Vec<String>>();
                let nice_resource_name = to_snake(&use_name_parts);
                if whitelist && !include.remove(&nice_resource_name) {
                    continue;
                }
                if exclude.remove(&nice_resource_name) {
                    continue;
                }
                println!("Generating {}", nice_resource_name);
                let camel_name = to_camel(&use_name_parts);
                let mut raw_fields = TopLevelFields::default();
                generate_fields_from_value_map(&mut raw_fields, &use_name_parts, &resource.block.attributes, true);
                generate_block_fields(&mut raw_fields, &use_name_parts, &resource.block.block_types, true);
                raw_fields.finish(&camel_name);
                let builder_fields = raw_fields.builder_fields;
                let copy_builder_fields = raw_fields.copy_builder_fields;
                let extra_types = raw_fields.extra_types;
                let resource_fields = raw_fields.fields;
                let resource_mut_methods = raw_fields.mut_methods;
                let resource_ref_methods = raw_fields.ref_methods;
                let resource_ident = format_ident!("{}", camel_name);
                let resource_inner_ident = format_ident!("{}_", camel_name);
                let resource_inner_mut_ident = format_ident!("{}Data", camel_name);
                let resource_builder_ident = format_ident!("Build{}", camel_name);
                let resource_ref_ident = format_ident!("{}Ref", camel_name);
                out.push(quote!{
                    #[derive(Serialize)] struct #resource_inner_mut_ident {
                        #[serde(skip_serializing_if = "std::vec::Vec::is_empty")] depends_on: std::vec::Vec < std::string::String >,
                        #[serde(skip_serializing_if = "std::option::Option::is_none")] provider: std::option::Option < std::string::String >,
                        #[
                            serde(skip_serializing_if = "terrars::SerdeSkipDefault::is_default")
                        ] lifecycle: terrars:: ResourceLifecycle,
                        #[serde(skip_serializing_if = "std::option::Option::is_none")] for_each: std::option::Option < std::string::String >,
                        #[serde(skip_serializing_if = "std::vec::Vec::is_empty")] provisioner: std::vec::Vec < terrars:: Provisioner >,
                        #(#resource_fields,) *
                    }
                    struct #resource_inner_ident {
                        shared: terrars:: StackShared,
                        tf_id: std::string::String,
                        data: std:: cell:: RefCell < #resource_inner_mut_ident >,
                    }
                    #[derive(Clone)] pub struct #resource_ident(std:: rc:: Rc < #resource_inner_ident >);
                    impl #resource_ident {
                        fn shared(&self) -> &terrars::StackShared {
                            &self.0.shared
                        }
                        pub fn depends_on(self, dep: &impl terrars::Referable) -> Self {
                            self.0.data.borrow_mut().depends_on.push(dep.extract_ref());
                            self
                        }
                        pub fn set_provider(self, provider:& #provider_ident) -> Self {
                            self.0.data.borrow_mut().provider = std::option::Option::Some(provider.provider_ref());
                            self
                        }
                        pub fn set_create_before_destroy(self, v: bool) -> Self {
                            self.0.data.borrow_mut().lifecycle.create_before_destroy = v;
                            self
                        }
                        pub fn set_prevent_destroy(self, v: bool) -> Self {
                            self.0.data.borrow_mut().lifecycle.prevent_destroy = v;
                            self
                        }
                        pub fn ignore_changes_to_all(self) -> Self {
                            self.0.data.borrow_mut().lifecycle.ignore_changes =
                                std::option::Option::Some(terrars::IgnoreChanges::All(terrars::IgnoreChangesAll::All));
                            self
                        }
                        pub fn ignore_changes_to_attr(self, attr: impl std::string::ToString) -> Self {
                            {
                                let mut d = self.0.data.borrow_mut();
                                if match &mut d.lifecycle.ignore_changes {
                                    std::option::Option::Some(i) => match i {
                                        terrars::IgnoreChanges::All(_) => {
                                            true
                                        },
                                        terrars::IgnoreChanges::Refs(r) => {
                                            r.push(attr.to_string());
                                            false
                                        },
                                    },
                                    std::option::Option::None => true,
                                } {
                                    d.lifecycle.ignore_changes =
                                        std::option::Option::Some(terrars::IgnoreChanges::Refs(vec![attr.to_string()]));
                                }
                            }
                            self
                        }
                        pub fn replace_triggered_by_resource(self, r: &impl terrars::Resource) -> Self {
                            self.0.data.borrow_mut().lifecycle.replace_triggered_by.push(r.extract_ref());
                            self
                        }
                        pub fn replace_triggered_by_attr(self, attr: impl std::string::ToString) -> Self {
                            self.0.data.borrow_mut().lifecycle.replace_triggered_by.push(attr.to_string());
                            self
                        }
                        /// Add a provisioner (ex: `local-exec`, `remote-exec`, `file`) to this resource.
                        /// Provisioners run in the order they're added.
                        pub fn add_provisioner(self, v: impl std::convert::Into<terrars::Provisioner>) -> Self {
                            self.0.data.borrow_mut().provisioner.push(v.into());
                            self
                        }
                        pub fn add_lifecycle_precondition(
                            self,
                            condition: impl std::convert::Into<terrars::PrimField<bool>>,
                            error_message: impl std::convert::Into<terrars::PrimField<std::string::String>>,
                        ) -> Self {
                            self.0.data.borrow_mut().lifecycle.precondition.push(terrars::LifecycleCondition {
                                condition: condition.into(),
                                error_message: error_message.into(),
                            });
                            self
                        }
                        pub fn add_lifecycle_postcondition(
                            self,
                            condition: impl std::convert::Into<terrars::PrimField<bool>>,
                            error_message: impl std::convert::Into<terrars::PrimField<std::string::String>>,
                        ) -> Self {
                            self.0.data.borrow_mut().lifecycle.postcondition.push(terrars::LifecycleCondition {
                                condition: condition.into(),
                                error_message: error_message.into(),
                            });
                            self
                        }
                        #(#resource_mut_methods) * #(#resource_ref_methods) *
                    }
                    impl terrars:: Referable for #resource_ident {
                        fn extract_ref(&self) -> std::string::String {
                            format!("{}.{}", self.0.extract_resource_type(), self.0.extract_tf_id())
                        }
                    }
                    impl terrars:: Resource for #resource_ident {
                    }
                    impl terrars:: ToListMappable for #resource_ident {
                        type O = terrars:: ListRef < #resource_ref_ident >;
                        fn do_map(self, base: std::string::String) -> Self::O {
                            self.0.data.borrow_mut().for_each = std::option::Option::Some(format!("${{{}}}", base));
                            terrars::ListRef::new(self.0.shared.clone(), self.extract_ref())
                        }
                    }
                    impl terrars:: Resource_ for #resource_inner_ident {
                        fn extract_resource_type(&self) -> std::string::String {
                            #resource_name.into()
                        }
                        fn extract_tf_id(&self) -> std::string::String {
                            self.tf_id.clone()
                        }
                        fn extract_value(&self) -> serde_json::Value {
                            serde_json::to_value(&self.data).unwrap()
                        }
                    }
                    pub struct #resource_builder_ident {
                        pub tf_id: std::string::String,
                        #(#builder_fields,) *
                    }
                    impl #resource_builder_ident {
                        pub fn build(self, stack:& mut terrars:: Stack) -> #resource_ident {
                            let out = #resource_ident(std:: rc:: Rc:: new(#resource_inner_ident {
                                shared: stack.shared.clone(),
                                tf_id: self.tf_id,
                                data: std:: cell:: RefCell:: new(#resource_inner_mut_ident {
                                    depends_on: core:: default:: Default:: default(),
                                    provider: std::option::Option::None,
                                    lifecycle: core:: default:: Default:: default(),
                                    for_each: std::option::Option::None,
                                    provisioner: core:: default:: Default:: default(),
                                    #(#copy_builder_fields,) *
                                }),
                            }));
                            stack.add_resource(out.0.clone());
                            out
                        }
                    }
                    pub struct #resource_ref_ident {
                        shared: terrars:: StackShared,
                        base: std::string::String
                    }
                    impl terrars:: Ref for #resource_ref_ident {
                        fn new(shared: terrars::StackShared, base: std::string::String) -> Self {
                            Self {
                                shared: shared,
                                base: base,
                            }
                        }
                    }
                    impl #resource_ref_ident {
                        fn extract_ref(&self) -> std::string::String {
                            self.base.clone()
                        }
                        fn shared(&self) -> &terrars::StackShared {
                            &self.shared
                        }
                        #(#resource_ref_methods) *
                    }
                    #(#extra_types) *
                });
                write_file(&provider_dir.join(format!("{}.rs", nice_resource_name)), out)?;
                let path_ident = format_ident!("{}", nice_resource_name);
                let feature_gate = if config.feature_gate.is_some() {
                    features.push(nice_resource_name.clone());
                    quote!(#[cfg(feature = #nice_resource_name)])
                } else {
                    quote!()
                };
                mod_out.push(quote!{
                    #feature_gate pub mod #path_ident;
                    #feature_gate pub use #path_ident::*;
                });
            }

            // Data sources
            for (datasource_name, datasource) in &provider_schema.data_source_schemas {
                let mut out = rustfile_template(&["Referable", "Datasource_", "Ref"]);
                out.push(quote!(use super:: provider:: #provider_ident;));
                let use_name_parts =
                    ["data"]
                        .into_iter()
                        .chain(
                            datasource_name
                                .strip_prefix(&provider_prefix)
                                .context_with(
                                    "Name missing expected provider prefix",
                                    ea!(datasource = datasource_name, prefix = provider_prefix),
                                )?
                                .split("_"),
                        )
                        .map(ToString::to_string)
                        .collect::<Vec<String>>();
                let nice_datasource_name = to_snake(&use_name_parts);
                if whitelist && !include.remove(&nice_datasource_name) {
                    continue;
                }
                println!("Generating datasource {}", datasource_name);
                let camel_name = to_camel(&use_name_parts);
                let mut raw_fields = TopLevelFields::default();
                generate_fields_from_value_map(&mut raw_fields, &use_name_parts, &datasource.block.attributes, true);
                generate_block_fields(&mut raw_fields, &use_name_parts, &datasource.block.block_types, true);
                raw_fields.finish(&camel_name);
                let builder_fields = raw_fields.builder_fields;
                let copy_builder_fields = raw_fields.copy_builder_fields;
                let extra_types = raw_fields.extra_types;
                let datasource_fields = raw_fields.fields;
                let datasource_mut_methods = raw_fields.mut_methods;
                let datasource_ref_methods = raw_fields.ref_methods;
                let datasource_ident = format_ident!("{}", camel_name);
                let datasource_inner_ident = format_ident!("{}_", camel_name);
                let datasource_inner_mut_ident = format_ident!("{}Data", camel_name);
                let datasource_builder_ident = format_ident!("Build{}", camel_name);
                let datasource_ref_ident = format_ident!("{}Ref", camel_name);
                out.push(quote!{
                    #[derive(Serialize)] struct #datasource_inner_mut_ident {
                        #[serde(skip_serializing_if = "std::vec::Vec::is_empty")] depends_on: std::vec::Vec < std::string::String >,
                        #[
                            serde(skip_serializing_if = "terrars::SerdeSkipDefault::is_default")
                        ] provider: std::option::Option < std::string::String >,
                        #[
                            serde(skip_serializing_if = "terrars::SerdeSkipDefault::is_default")
                        ] lifecycle: terrars:: DatasourceLifecycle,
                        #[serde(skip_serializing_if = "std::option::Option::is_none")] for_each: std::option::Option < std::string::String >,
                        #(#datasource_fields,) *
                    }
                    struct #datasource_inner_ident {
                        shared: terrars:: StackShared,
                        tf_id: std::string::String,
                        data: std:: cell:: RefCell < #datasource_inner_mut_ident >,
                    }
                    #[derive(Clone)] pub struct #datasource_ident(std:: rc:: Rc < #datasource_inner_ident >);
                    impl #datasource_ident {
                        fn shared(&self) -> &terrars::StackShared {
                            &self.0.shared
                        }
                        pub fn depends_on(self, dep: &impl terrars::Referable) -> Self {
                            self.0.data.borrow_mut().depends_on.push(dep.extract_ref());
                            self
                        }
                        pub fn set_provider(&self, provider:& #provider_ident) ->& Self {
                            self.0.data.borrow_mut().provider = std::option::Option::Some(provider.provider_ref());
                            self
                        }
                        pub fn add_lifecycle_precondition(
                            self,
                            condition: impl std::convert::Into<terrars::PrimField<bool>>,
                            error_message: impl std::convert::Into<terrars::PrimField<std::string::String>>,
                        ) -> Self {
                            self.0.data.borrow_mut().lifecycle.precondition.push(terrars::LifecycleCondition {
                                condition: condition.into(),
                                error_message: error_message.into(),
                            });
                            self
                        }
                        pub fn add_lifecycle_postcondition(
                            self,
                            condition: impl std::convert::Into<terrars::PrimField<bool>>,
                            error_message: impl std::convert::Into<terrars::PrimField<std::string::String>>,
                        ) -> Self {
                            self.0.data.borrow_mut().lifecycle.postcondition.push(terrars::LifecycleCondition {
                                condition: condition.into(),
                                error_message: error_message.into(),
                            });
                            self
                        }
                        #(#datasource_mut_methods) * #(#datasource_ref_methods) *
                    }
                    impl terrars:: Referable for #datasource_ident {
                        fn extract_ref(&self) -> std::string::String {
                            format!("data.{}.{}", self.0.extract_datasource_type(), self.0.extract_tf_id())
                        }
                    }
                    impl terrars:: Datasource for #datasource_ident {
                    }
                    impl terrars:: ToListMappable for #datasource_ident {
                        type O = terrars:: ListRef < #datasource_ref_ident >;
                        fn do_map(self, base: std::string::String) -> Self::O {
                            self.0.data.borrow_mut().for_each = std::option::Option::Some(format!("${{{}}}", base));
                            terrars::ListRef::new(self.0.shared.clone(), self.extract_ref())
                        }
                    }
                    impl terrars:: Datasource_ for #datasource_inner_ident {
                        fn extract_datasource_type(&self) -> std::string::String {
                            #datasource_name.into()
                        }
                        fn extract_tf_id(&self) -> std::string::String {
                            self.tf_id.clone()
                        }
                        fn extract_value(&self) -> serde_json::Value {
                            serde_json::to_value(&self.data).unwrap()
                        }
                    }
                    pub struct #datasource_builder_ident {
                        pub tf_id: std::string::String,
                        #(#builder_fields,) *
                    }
                    impl #datasource_builder_ident {
                        pub fn build(self, stack:& mut terrars:: Stack) -> #datasource_ident {
                            let out = #datasource_ident(std:: rc:: Rc:: new(#datasource_inner_ident {
                                shared: stack.shared.clone(),
                                tf_id: self.tf_id,
                                data: std:: cell:: RefCell:: new(#datasource_inner_mut_ident {
                                    depends_on: core:: default:: Default:: default(),
                                    provider: std::option::Option::None,
                                    lifecycle: core:: default:: Default:: default(),
                                    for_each: std::option::Option::None,
                                    #(#copy_builder_fields,) *
                                }),
                            }));
                            stack.add_datasource(out.0.clone());
                            out
                        }
                    }
                    pub struct #datasource_ref_ident {
                        shared: terrars:: StackShared,
                        base: std::string::String
                    }
                    impl terrars:: Ref for #datasource_ref_ident {
                        fn new(shared: terrars::StackShared, base: std::string::String) -> Self {
                            Self {
                                shared: shared,
                                base: base,
                            }
                        }
                    }
                    impl #datasource_ref_ident {
                        fn shared(&self) -> &terrars::StackShared {
                            &self.shared
                        }
                        fn extract_ref(&self) -> std::string::String {
                            self.base.clone()
                        }
                        #(#datasource_ref_methods) *
                    }
                    #(#extra_types) *
                });
                write_file(&provider_dir.join(format!("{}.rs", nice_datasource_name)), out)?;
                let path_ident = format_ident!("{}", nice_datasource_name);
                let feature_gate = if config.feature_gate.is_some() {
                    features.push(nice_datasource_name.clone());
                    quote!(#[cfg(feature = #nice_datasource_name)])
                } else {
                    quote!()
                };
                mod_out.push(quote!{
                    #feature_gate pub mod #path_ident;
                    #feature_gate pub use #path_ident::*;
                });
            }
            write_file(&provider_dir.join("mod.rs"), mod_out)?;
            if whitelist && !include.is_empty() {
                return Err(
                    loga::err_with("Included resources/datasources were not found", ea!(included = include.dbg_str())),
                );
            }
            if features.len() > 0 {
                let cargo_path = config.feature_gate.unwrap();
                let mut manifest =
                    cargo_toml::Manifest::from_slice(
                        &fs::read(
                            &cargo_path,
                        ).context_with(
                            "Error opening Cargo.toml to update features",
                            ea!(path = cargo_path.to_string_lossy()),
                        )?,
                    ).context_with("Error parsing Cargo.toml", ea!(path = cargo_path.to_string_lossy()))?;
                manifest.features.clear();
                for f in features {
                    manifest.features.insert(f, vec![]);
                }
                fs::write(
                    &cargo_path,
                    &toml::to_string(&manifest).context("Error serializing modified Cargo.toml")?.into_bytes(),
                ).context_with("Error writing to Cargo.toml", ea!(path = cargo_path.to_string_lossy()))?;
            }
        }
        Ok(())
    }) {
        Ok(_) => { },
        Err(e) => {
            fatal(e);
        },
    }
}
