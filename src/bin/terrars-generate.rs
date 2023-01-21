use anyhow::{
    anyhow,
    Context,
    Result,
};
use clap::Parser;
use proc_macro2::{
    Ident,
    TokenStream,
};
use quote::{
    format_ident,
    quote,
};
use serde::{
    Serialize,
    Deserialize,
};
use serde_json::json;
use sloggers::{
    terminal::{
        Destination,
        TerminalLoggerBuilder,
    },
    types::Severity,
    Build,
};
use std::{
    collections::HashSet,
    fs::{
        self,
        create_dir_all,
        File,
        remove_dir_all,
    },
    io::Write,
    path::{
        Path,
        PathBuf,
    },
    process::Command,
    str::FromStr,
};
use crate::generatelib::{
    generate::{
        to_camel,
        to_snake,
        TopLevelFields,
        generate_fields_from_value_map,
        generate_block_fields,
    },
    sourceschema::ProviderSchemas,
};

pub mod generatelib;

pub trait CollCommand {
    fn run(&mut self) -> Result<()>;
}

impl CollCommand for Command {
    fn run(&mut self) -> Result<()> {
        match match self.output() {
            Ok(o) => {
                if o.status.success() {
                    Ok(())
                } else {
                    Err(anyhow!("Exit code indicated error: {:?}", o))
                }
            },
            Err(e) => Err(e.into()),
        } {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow!("Failed to run {:?}", &self).context(e)),
        }
    }
}

fn main() {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);
    let root_log = builder.build().unwrap();
    match es!({
        #[derive(Serialize, Deserialize)]
        struct Config {
            provider: String,
            version: String,
            include: Vec<String>,
            dest: Option<PathBuf>,
        }

        #[derive(Parser)]
        struct Arguments {
            configs: Vec<PathBuf>,
            #[arg(long)]
            dump: bool,
        }

        let args = Arguments::parse();
        if args.configs.is_empty() {
            return Err(anyhow!("No configs specified; nothing to do"));
        }
        for config in args.configs {
            let config =
                serde_json::from_slice::<Config>(
                    &fs::read(&config).context("Couldn't find config at specified location")?,
                ).context("Error deserializing config json")?;
            let (vendor, shortname) =
                config.provider.split_once("/").unwrap_or_else(|| ("hashicorp".into(), &config.provider));
            let provider_prefix = format!("{}_", shortname);
            let mut include: HashSet<&String> = config.include.iter().collect();
            let whitelist = !include.is_empty();

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
            Command::new("terraform")
                .args(&["init", "-no-color"])
                .current_dir(&dir)
                .run()
                .context("Error initializing terraform in export dir")?;
            let schema_raw =
                Command::new("terraform")
                    .args(&["providers", "schema", "-json", "-no-color"])
                    .current_dir(&dir)
                    .output()
                    .context("Error outputting terraform provider schema")?
                    .stdout;
            if args.dump {
                fs::write("dump.json", &schema_raw)?;
            }
            let schema: ProviderSchemas =
                serde_json::from_slice(&schema_raw).context("Error parsing provider schema json from terraform")?;

            // Generate
            fn write_file(path: &Path, contents: Vec<TokenStream>) -> Result<()> {
                match es!({
                    File::create(&path)
                        .context("Failed to create rust file")?
                        .write_all(
                            genemichaels::format_ast(syn::parse2::<syn::File>(quote!(#(#contents) *)).map_err(|e| {
                                anyhow!(
                                    "Failed to parse generated code AST for formatting: {}\n\n{}",
                                    e,
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
                                )
                            })?, &genemichaels::FormatConfig::default(), Default::default())?.rendered.as_bytes(),
                        )
                        .context("Failed to write rust file")?;
                    Ok(())
                }) {
                    Ok(_) => { },
                    Err(e) => Err(e).with_context(|| {
                        format!("Failed to write generated code to {}", path.to_string_lossy())
                    })?,
                }
                Ok(())
            }

            fn rustfile_template() -> Vec<TokenStream> {
                vec![quote!(
                    use serde::Serialize;
                    use std::cell::RefCell;
                    use std::rc::Rc;
                    use terrars::*;
                )]
            }

            // Provider type + provider
            let provider_schema = {
                let key = format!("registry.terraform.io/{}/{}", vendor, shortname);
                schema.provider_schemas.get(&key).ok_or_else(|| {
                    anyhow!("Missing provider schema for listed provider {}", config.provider)
                })?
            };
            let provider_name_parts = &shortname.split("-").map(ToString::to_string).collect::<Vec<String>>();
            let provider_snake_name = to_snake(provider_name_parts);
            let provider_dir = if let Some(dest) = config.dest {
                dest
            } else {
                PathBuf::from_str(&provider_snake_name).unwrap()
            };
            if provider_dir.exists() {
                remove_dir_all(&provider_dir)?;
            }
            create_dir_all(&provider_dir)?;
            let mut mod_out = vec![];
            let provider_ident: Ident;
            {
                let mut out = rustfile_template();
                let camel_name = to_camel(provider_name_parts);
                let provider_type_name = format_ident!("ProviderType{}", camel_name);
                let source = &config.provider;
                let version = &config.version;
                let provider_type_fn = format_ident!("provider_{}", provider_snake_name);
                let provider_inner_mut_ident = format_ident!("Provider{}Data", camel_name);
                let mut raw_fields = TopLevelFields::default();
                generate_fields_from_value_map(
                    &mut raw_fields,
                    &provider_name_parts,
                    &provider_schema.provider.block.attributes,
                    true,
                );
                let builder_fields = raw_fields.builder_fields;
                let copy_builder_fields = raw_fields.copy_builder_fields;
                let extra_types = raw_fields.extra_types;
                let provider_fields = raw_fields.fields;
                let provider_mut_methods = raw_fields.mut_methods;
                provider_ident = format_ident!("Provider{}", camel_name);
                let provider_inner_ident = format_ident!("Provider{}_", camel_name);
                let provider_builder_ident = format_ident!("BuildProvider{}", camel_name);
                out.push(quote!{
                    pub struct #provider_type_name;
                    impl ProviderType for #provider_type_name {
                        fn extract_tf_id(&self) -> String {
                            #shortname.into()
                        }
                        fn extract_required_provider(&self) -> serde_json::Value {
                            serde_json::json!({
                                "source": #source,
                                "version": #version,
                            })
                        }
                    }
                    pub fn #provider_type_fn(stack: &mut Stack) -> Rc < #provider_type_name > {
                        let out = Rc:: new(#provider_type_name);
                        stack.add_provider_type(out.clone());
                        out
                    }
                    #[derive(Serialize)] struct #provider_inner_mut_ident {
                        #[serde(skip_serializing_if = "Option::is_none")]
                        alias: Option<String>,
                        #(#provider_fields,) *
                    }
                    struct #provider_inner_ident {
                        data: RefCell < #provider_inner_mut_ident >,
                    }
                    pub struct #provider_ident(Rc < #provider_inner_ident >);
                    impl #provider_ident {
                        pub fn provider_ref(&self) -> String {
                            let data = self.0.data.borrow();
                            if let Some(alias) = &data.alias {
                                format!("{}.{}", #shortname, alias)
                            }
                            else {
                                #shortname.into()
                            }
                        }
                        pub fn set_alias(self, alias: String) -> Self {
                            self.0.data.borrow_mut().alias = Some(alias);
                            self
                        }
                        #(#provider_mut_methods) *
                    }
                    impl Provider for #provider_inner_ident {
                        fn extract_type_tf_id(&self) -> String {
                            #shortname.into()
                        }
                        fn extract_provider(&self) -> serde_json::Value {
                            serde_json::to_value(&self.data).unwrap()
                        }
                    }
                    pub struct #provider_builder_ident {
                        #(#builder_fields,) *
                    }
                    impl #provider_builder_ident {
                        pub fn build(self, _provider_type:& #provider_type_name, stack: &mut Stack) -> #provider_ident {
                            let out = #provider_ident(Rc:: new(#provider_inner_ident {
                                data: RefCell:: new(#provider_inner_mut_ident {
                                    alias: None,
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
                let mut out = rustfile_template();
                out.push(quote!(use super:: provider:: #provider_ident;));
                let use_name_parts = resource_name.strip_prefix(&provider_prefix).ok_or_else(|| {
                    anyhow!(
                        "resource [[{}]] name missing expected provider prefix [[{}]]",
                        resource_name,
                        provider_prefix
                    )
                })?.split("_").map(ToString::to_string).collect::<Vec<String>>();
                let nice_resource_name = to_snake(&use_name_parts);
                if whitelist && !include.remove(&nice_resource_name) {
                    continue;
                }
                println!("Generating {}", resource_name);
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
                        #[serde(skip_serializing_if = "Vec::is_empty")]
                        depends_on: Vec<String>,
                        #[serde(skip_serializing_if = "Option::is_none")]
                        provider: Option<String>,
                        #[serde(skip_serializing_if = "SerdeSkipDefault::is_default")]
                        lifecycle: ResourceLifecycle,
                        #[serde(skip_serializing_if = "Option::is_none")]
                        for_each: Option<String>,
                        #(#resource_fields,) *
                    }
                    struct #resource_inner_ident {
                        shared: StackShared,
                        tf_id: String,
                        data: RefCell < #resource_inner_mut_ident >,
                    }
                    #[derive(Clone)] pub struct #resource_ident(Rc < #resource_inner_ident >);
                    impl #resource_ident {
                        fn shared(&self) -> &StackShared {
                            &self.0.shared
                        }
                        pub fn depends_on(self, dep: &impl Resource) -> Self {
                            self.0.data.borrow_mut().depends_on.push(dep.extract_ref());
                            self
                        }
                        pub fn set_provider(self, provider:& #provider_ident) -> Self {
                            self.0.data.borrow_mut().provider = Some(provider.provider_ref());
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
                                Some(IgnoreChanges::All(IgnoreChangesAll::All));
                            self
                        }
                        pub fn ignore_changes_to_attr(self, attr: impl ToString) -> Self {
                            {
                                let mut d = self.0.data.borrow_mut();
                                if match &mut d.lifecycle.ignore_changes {
                                    Some(i) => match i {
                                        IgnoreChanges::All(_) => {
                                            true
                                        },
                                        IgnoreChanges::Refs(r) => {
                                            r.push(attr.to_string());
                                            false
                                        },
                                    },
                                    None => true,
                                } {
                                    d.lifecycle.ignore_changes = Some(IgnoreChanges::Refs(vec![attr.to_string()]));
                                }
                            }
                            self
                        }
                        pub fn replace_triggered_by_resource(self, r: &impl Resource) -> Self {
                            self.0.data.borrow_mut().lifecycle.replace_triggered_by.push(r.extract_ref());
                            self
                        }
                        pub fn replace_triggered_by_attr(self, attr: impl ToString) -> Self {
                            self.0.data.borrow_mut().lifecycle.replace_triggered_by.push(attr.to_string());
                            self
                        }
                        #(#resource_mut_methods) * #(#resource_ref_methods) *
                    }
                    impl Resource for #resource_ident {
                        fn extract_ref(&self) -> String {
                            format!("{}.{}", self.0.extract_resource_type(), self.0.extract_tf_id())
                        }
                    }
                    impl ToListMappable for #resource_ident {
                        type O = ListRef < #resource_ref_ident >;
                        fn do_map(self, base: String) -> Self::O {
                            self.0.data.borrow_mut().for_each = Some(format!("${{{}}}", base));
                            ListRef::new(self.0.shared.clone(), self.extract_ref())
                        }
                    }
                    impl Resource_ for #resource_inner_ident {
                        fn extract_resource_type(&self) -> String {
                            #resource_name.into()
                        }
                        fn extract_tf_id(&self) -> String {
                            self.tf_id.clone()
                        }
                        fn extract_value(&self) -> serde_json::Value {
                            serde_json::to_value(&self.data).unwrap()
                        }
                    }
                    pub struct #resource_builder_ident {
                        pub tf_id: String,
                        #(#builder_fields,) *
                    }
                    impl #resource_builder_ident {
                        pub fn build(self, stack: &mut Stack) -> #resource_ident {
                            let out = #resource_ident(Rc:: new(#resource_inner_ident {
                                shared: stack.shared.clone(),
                                tf_id: self.tf_id,
                                data: RefCell:: new(#resource_inner_mut_ident {
                                    depends_on: core::default::Default::default(),
                                    provider: None,
                                    lifecycle: core::default::Default::default(),
                                    for_each: None,
                                    #(#copy_builder_fields,) *
                                }),
                            }));
                            stack.add_resource(out.0.clone());
                            out
                        }
                    }
                    pub struct #resource_ref_ident {
                        shared: StackShared,
                        base: String
                    }
                    impl Ref for #resource_ref_ident {
                        fn new(shared: StackShared, base: String) -> Self {
                            Self {
                                shared: shared,
                                base: base,
                            }
                        }
                    }
                    impl #resource_ref_ident {
                        fn extract_ref(&self) -> String {
                            self.base.clone()
                        }
                        fn shared(&self) -> &StackShared {
                            &self.shared
                        }
                        #(#resource_ref_methods) *
                    }
                    #(#extra_types) *
                });
                write_file(&provider_dir.join(format!("{}.rs", nice_resource_name)), out)?;
                let path_ident = format_ident!("{}", nice_resource_name);
                mod_out.push(quote!(pub mod #path_ident; pub use #path_ident::*;));
            }

            // Data sources
            for (datasource_name, datasource) in &provider_schema.data_source_schemas {
                let mut out = rustfile_template();
                out.push(quote!(use super:: provider:: #provider_ident;));
                let use_name_parts =
                    ["data"].into_iter().chain(datasource_name.strip_prefix(&provider_prefix).ok_or_else(|| {
                        anyhow!(
                            "data source [[{}]] name missing expected provider prefix [[{}]]",
                            datasource_name,
                            provider_prefix
                        )
                    })?.split("_")).map(ToString::to_string).collect::<Vec<String>>();
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
                        #[serde(skip_serializing_if = "SerdeSkipDefault::is_default")]
                        provider: Option<String>,
                        #[serde(skip_serializing_if = "Option::is_none")]
                        for_each: Option<String>,
                        #(#datasource_fields,) *
                    }
                    struct #datasource_inner_ident {
                        shared: StackShared,
                        tf_id: String,
                        data: RefCell < #datasource_inner_mut_ident >,
                    }
                    #[derive(Clone)] pub struct #datasource_ident(Rc < #datasource_inner_ident >);
                    impl #datasource_ident {
                        fn shared(&self) -> &StackShared {
                            &self.0.shared
                        }
                        pub fn set_provider(&self, provider:& #provider_ident) ->& Self {
                            self.0.data.borrow_mut().provider = Some(provider.provider_ref());
                            self
                        }
                        #(#datasource_mut_methods) * #(#datasource_ref_methods) *
                    }
                    impl Datasource for #datasource_ident {
                        fn extract_ref(&self) -> String {
                            format!("data.{}.{}", self.0.extract_datasource_type(), self.0.extract_tf_id())
                        }
                    }
                    impl ToListMappable for #datasource_ident {
                        type O = ListRef < #datasource_ref_ident >;
                        fn do_map(self, base: String) -> Self::O {
                            self.0.data.borrow_mut().for_each = Some(format!("${{{}}}", base));
                            ListRef::new(self.0.shared.clone(), self.extract_ref())
                        }
                    }
                    impl Datasource_ for #datasource_inner_ident {
                        fn extract_datasource_type(&self) -> String {
                            #datasource_name.into()
                        }
                        fn extract_tf_id(&self) -> String {
                            self.tf_id.clone()
                        }
                        fn extract_value(&self) -> serde_json::Value {
                            serde_json::to_value(&self.data).unwrap()
                        }
                    }
                    pub struct #datasource_builder_ident {
                        pub tf_id: String,
                        #(#builder_fields,) *
                    }
                    impl #datasource_builder_ident {
                        pub fn build(self, stack: &mut Stack) -> #datasource_ident {
                            let out = #datasource_ident(Rc:: new(#datasource_inner_ident {
                                shared: stack.shared.clone(),
                                tf_id: self.tf_id,
                                data: RefCell:: new(#datasource_inner_mut_ident {
                                    provider: None,
                                    for_each: None,
                                    #(#copy_builder_fields,) *
                                }),
                            }));
                            stack.add_datasource(out.0.clone());
                            out
                        }
                    }
                    pub struct #datasource_ref_ident {
                        shared: StackShared,
                        base: String
                    }
                    impl Ref for #datasource_ref_ident {
                        fn new(shared: StackShared, base: String) -> Self {
                            Self {
                                shared: shared,
                                base: base,
                            }
                        }
                    }
                    impl #datasource_ref_ident {
                        fn shared(&self) -> &StackShared {
                            &self.shared
                        }
                        fn extract_ref(&self) -> String {
                            self.base.clone()
                        }
                        #(#datasource_ref_methods) *
                    }
                    #(#extra_types) *
                });
                write_file(&provider_dir.join(format!("{}.rs", nice_datasource_name)), out)?;
                let path_ident = format_ident!("{}", nice_datasource_name);
                mod_out.push(quote!(pub mod #path_ident; pub use #path_ident::*;));
            }
            write_file(&provider_dir.join("mod.rs"), mod_out)?;
            if whitelist && !include.is_empty() {
                return Err(anyhow!("The following included resources/datasources were not found: {:?}", include));
            }
        }
        Ok(())
    }) {
        Ok(_) => { },
        Err(e) => {
            err!(root_log, "Command failed with error", err = format!("{:?}", e));
            drop(root_log);
            std::process::exit(1);
        },
    }
}
