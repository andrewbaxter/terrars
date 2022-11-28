use anyhow::{anyhow, Context, Result};
use clap::Parser;
use once_cell::sync::Lazy;
use quote::format_ident;
use serde_json::{json, Value};
use sloggers::{
    terminal::{Destination, TerminalLoggerBuilder},
    types::Severity,
    Build,
};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    process::Command,
    str::FromStr,
};
use structre::structre;
use syn::Ident;
use tfschema_bindgen::{
    err, es,
    source_schema::{self, AggObjectType, AggSimpleType, ProviderSchemas, ValueType},
};

pub mod generatelib;

pub trait SimpleCommand {
    fn run(&mut self) -> Result<()>;
}

impl SimpleCommand for Command {
    fn run(&mut self) -> Result<()> {
        match match self.output() {
            Ok(o) => {
                if o.status.success() {
                    Ok(())
                } else {
                    Err(anyhow!("Exit code indicated error: {:?}", o))
                }
            }
            Err(e) => Err(e.into()),
        } {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow!("Failed to run {:?}", &self).context(e)),
        }
    }
}

#[derive(Clone)]
#[structre("^(?P<name>[^:]+):(?P<version>.*)$")]
struct ProviderVersion {
    pub name: String,
    pub version: String,
}

impl FromStr for ProviderVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        static re: Lazy<ProviderVersionFromRegex> = Lazy::new(|| ProviderVersionFromRegex::new());
        Ok(re.parse(s)?)
    }
}

#[derive(Parser)]
struct Arguments {
    #[arg(help = "List of providers with versions, like aws:4.0.999 or thirdparty/provider:1.0.0")]
    providers: Vec<ProviderVersion>,
}

fn main() {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);
    let root_log = builder.build().unwrap();
    match es!({
        let args = Arguments::parse();

        let dir = tempfile::tempdir()?;
        fs::write(
                dir.path().join("providers.tf.json"),
                &serde_json::to_vec(&json!({
                    "terraform": {
                        "required_providers": serde_json::to_value(
                            &args.providers.iter().map(|p|
                                (p.name.splitn(2,"/").next().unwrap().to_string(), json!({"source": p.name,
                                "version": p.version}))).collect::<BTreeMap<String, Value>>())?
                    }
                }))
                .unwrap(),
            )?;
        Command::new("terraform")
            .arg("init")
            .current_dir(&dir)
            .run()
            .context("Error initializing terraform in export dir")?;
        let schema: ProviderSchemas = serde_json::from_slice(
            Command::new("terraform")
                .args(&["providers", "schema", "-json"])
                .current_dir(&dir)
                .output()
                .context("Error outputting terraform provider schema")?
                .stdout,
        )?;

        // Generate
        fn quote_scalar_type(t: &ValueType) -> Ident {
            match t {
                source_schema::ScalarTypeKey::Number => format_ident!("f64"),
                source_schema::ScalarTypeKey::Integer => format_ident!("i64"),
                source_schema::ScalarTypeKey::String => format_ident!("String"),
                source_schema::ScalarTypeKey::Bool => format_ident!("bool"),
            }
        }

        fn add_el(v: &Vec<String>, e: &str) -> Vec<String> {
            let mut out = v.clone();
            out.push(e.to_string());
            out
        }

        fn generate_obj_agg_type(
            extra_types: &mut Vec<TokenStream>,
            path: Vec<String>,
            at: AggObjectType,
        ) -> Ident {
            let name = format_ident!("{}", to_camel(&path));
            let mut fields = vec![];
            for (field_name, subtype) in &at.1 {
                let field_ident = format_ident!("{}", field_name);
                let rusttype = generate_type(extra_types, add_el(&path, field_name), subtype);
                fields.push(quote!(#field_ident: Option<#rusttype>))
            }
            extra_types.push(quote!(
                pub struct #name {
                    #(#fields),*
                }
            ));
            name
        }

        fn generate_coll_agg_type(
            extra_types: &mut Vec<TokenStream>,
            path: Vec<String>,
            at: AggSimpleType,
        ) -> Ident {
            match at.0 {
                source_schema::AggSimpleTypeKey::Set => {
                    let element_type = match at.1 {
                        ValueType::Simple(t) => quote_scalar_type(t),
                        ValueType::AggSimple(a) => panic!("supposedly not supported by terraform"),
                        ValueType::AggObject(a) => panic!("supposedly not supported by terraform"),
                    };
                    quote!(HashSet<String, #element_type>)
                }
                source_schema::AggSimpleTypeKey::List => {
                    let element_type = match at.1 {
                        ValueType::Simple(t) => quote_scalar_type(t),
                        ValueType::AggSimple(a) => generate_coll_agg_type(add_vec(&path, "el"), a),
                        ValueType::AggObject(a) => generate_obj_agg_type(add_vec(&path, "el"), a),
                    };
                    quote!(Vec<#element_type>)
                }
                source_schema::AggSimpleTypeKey::Map => {
                    let element_type = match at.1 {
                        ValueType::Simple(t) => quote_scalar_type(t),
                        ValueType::AggSimple(a) => panic!("supposedly not supported by terraform"),
                        ValueType::AggObject(a) => panic!("supposedly not supported by terraform"),
                    };
                    quote!(HashMap<String, #element_type>)
                }
            };
        }

        fn generate_type(
            extra_types: &mut Vec<TokenStream>,
            path: Vec<String>,
            at: ValueType,
        ) -> Ident {
            match at {
                ValueType::Simple(t) => quote_scalar_type(t),
                ValueType::Aggregate(at) => {
                    generate_coll_agg_type(&mut out.extra_types, k.split("_"), at)
                }
            }
        }

        #[derive(Default)]
        struct TopLevelFields {
            extra_types: Vec<TokenStream>,
            provider_fields: Vec<TokenStream>,
            provider_ref_methods: Vec<TokenStream>,
            provider_mut_methods: Vec<TokenStream>,
            builder_fields: Vec<TokenStream>,
            copy_builder_fields: Vec<TokenStream>,
        }

        fn generate_fields(fields: BTreeMap<String, source_schema::Value>) -> TopLevelFields {
            let mut out = TopLevelFields::default();
            for (k, v) in fields {
                let field_name = format_ident!("{}", k);
                let rusttype = generate_type(v.r#type);
                let set_field_name = format_ident!("set_{}", k);
                let refpat = format!("${{{{}}.{}}}", k);
                if v.computed {
                    // nop
                } else {
                    if v.required {
                        out.builder_fields.push(quote!(#field_name: #rusttype));
                        out.copy_builder_fields
                            .push(quote!(#field_name: self.#field_name));
                        out.provider_fields.push(quote!(#field_name: #rusttype));
                    } else {
                        out.provider_fields
                            .push(quote!(#field_name: Option<#rusttype>));
                    }
                    out.provider_mut_methods.push(quote!(
                        fn #set_field_name(&self, v: #rusttype) -> &Self {
                            self.data.#field_name = v;
                            self
                        }
                    ));
                }
                out.provider_ref_methods.push(quote!(
                    fn #field_name(&self) -> Value<#rusttype> {
                        Value::Reference(format!(#refpat, self.tf_id.clone()))
                    }
                ));
            }
            out
        }

        for provider_dep in args.providers {
            let mut out = vec![];
            out.push(quote!(
                use core::default::Default;
                use terrarust::{Value, ProviderType, Provider, Resource, Datasource};
            ));

            // Provider type + provider
            let (vendor, shortname) = provider_dep
                .name
                .split_once("/")
                .unwrap_or_else(|| ("hashicorp".into(), provider_dep.name));
            let provider_schema = {
                let key = format!("registry.terraform.io/{}/{}", vendor, shortname);
                schema.provider_schemas.get(key).ok_or_else(|| {
                    anyhow!(
                        "Missing provider schema for listed provider {}",
                        provider_dep.name
                    )
                })?
            };
            let provider_type_name = format_ident!("ProviderType{}", camel_longname);
            let source = &provider_dep.name;
            let version = &provider_dep.version;
            let provider_type_fn = format_ident!("provider_{}", snake_longname);
            let name = format_ident!("terraform_provider_{}", p.name.split("/").last().unwrap());
            let provider_data_name = format_ident!("Provider{}Data", camel_longname);

            let raw_fields = generate_fields(provider_schema.provider.block.attributes);
            let builder_fields = raw_fields.builder_fields;
            let copy_builder_fields = raw_fields.copy_builder_fields;
            let extra_types = raw_fields.extra_types;
            let provider_fields = raw_fields.provider_fields;
            let provider_mut_methods = raw_fields.provider_mut_methods;
            let provider_ref_methods = raw_fields.provider_ref_methods;

            let provider_name = format_ident!("Provider{}", camel_longname);
            let provider_builder_name = format_ident!("BuildProvider{}", camel_longname);
            out.push(quote! {
                pub struct #provider_type_name;

                impl ProviderType for #provider_type_name {
                    fn extract_tf_id(&self) -> String {
                        #name.into()
                    }

                    fn extract_required_provider(&self) -> RequiredProviderData {
                        RequiredProviderData {
                            source: #source,
                            version: #version,
                        }
                    }
                }

                pub fn #provider_type_fn() -> Rc<#provider_type_name> {
                    return Rc::new(#provider_type_name);
                }

                struct #provider_data_name {
                    alias: Option<String>,
                    #(#provider_fields),*
                }

                pub struct #provider_name {
                    provider_type: Rc<#provider_type_name>,
                    data: RefCell<#provider_data_name>,
                }

                impl #provider_name {
                    fn set_alias(mut self, alias: String) -> Self {
                        self.data.alias = Some(alias);
                        self
                    }

                    #(#provider_methods)*
                }

                impl Provider for #provider_name {
                    fn extract_type_tf_id(&self) -> String {
                        self.provider_type.extract_tf_id()
                    }

                    fn extract_provider(&self) -> serde_json::Value {
                        serde_json::to_value(&self.data).unwrap()
                    }

                    fn provider_ref(&self) -> String {
                        if let Some(alias) = self.alias {
                            format!("{}.{}", self.provider_type.extract_tf_id(), alias)
                        } else {
                            self.provider_type.extract_tf_id()
                        }
                    }
                }

                pub struct #provider_builder_name {
                    #(#builder_fields),*
                }

                impl #provider_builder_name {
                    fn build(self, stack: &Stack) -> #provider_name {
                        let out = Rc::new(#provider_name {
                            provider_type: self.provider_type,
                            data: RefCell::new(#provider_data_name{
                                #(#builder_copy_fields)*
                            }),
                        });
                        stack.add_provider(out.clone());
                        out
                    }
                }

                #(#extra_types)*
            });

            // Resources
            for (resource_name, resource) in provider_schema.resource_schemas.unwrap_or_default() {
                let resource_data_name = format_ident!("Resource{}Data", camel_longname);
                let (resource_fields, resource_methods, builder_fields, builder_copy_fields) =
                    generate_fields(resource.block.attributes);
                let resource_name = format_ident!("Resource{}", camel_longname);
                let resource_builder_name = format_ident!("BuildResource{}", camel_longname);
                out.push(quote! {
                    struct #resource_data_name {
                        #(#resource_fields),*
                    }

                    pub struct #resource_name {
                        tf_id: String,
                        data: RefCell<#resource_data_name>,
                    }

                    impl #resource_name {
                        #(#resource_methods)*
                    }

                    impl Resource for #resource_name {
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

                    pub struct #resource_builder_name {
                        pub tf_id: String,
                        #(#builder_fields),*
                    }

                    impl #resource_builder_name {
                        fn build(self, stack: &Stack) -> #resource_name {
                            let out = Rc::new(#resource_name {
                                tf_id: self.tf_id,
                                data: RefCell::new(#resource_data_name{
                                    #(#builder_copy_fields)*
                                }),
                            });
                            stack.add_resource(out.clone());
                            out
                        }
                    }
                });
            }

            // Data sources
            for (datasource_name, datasource) in &provider_schema.data_source_schemas {
                let datasource_data_name = format_ident!("Data{}Data", camel_longname);
                let (datasource_fields, datasource_methods, builder_fields, builder_copy_fields) =
                    generate_fields(datasource.block.attributes);
                let datasource_name = format_ident!("Data{}", camel_longname);
                let datasource_builder_name = format_ident!("BuildData{}", camel_longname);
                out.push(quote! {
                    struct #datasource_data_name {
                        #(#datasource_fields),*
                    }

                    pub struct #datasource_name {
                        tf_id: String,
                        data: RefCell<#datasource_data_name>,
                    }

                    impl #datasource_name {
                        #(#datasource_methods)*
                    }

                    impl Data for #datasource_name {
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

                    pub struct #datasource_builder_name {
                        pub tf_id: String,
                        #(#builder_fields),*
                    }

                    impl #datasource_builder_name {
                        fn build(self, stack: &Stack) -> #datasource_name {
                            let out = Rc::new(#datasource_name {
                                tf_id: self.tf_id,
                                data: RefCell::new(#datasource_data_name{
                                    #(#builder_copy_fields)*
                                }),
                            });
                            stack.add_datasource(out.clone());
                            out
                        }
                    }
                });
            }

            // Write
            File::create(PathBuf::from_str(&format!("{}.rs", snake_provider_name)).unwrap())
                .context("Failed to create rust file")?
                .write_all(TokenStream::from_iter(&out).to_string().as_bytes())
                .context("Failed to write rust file");
        }

        Ok(())
    }) {
        Ok(_) => {}
        Err(e) => {
            err!(
                root_log,
                "Command failed with error",
                err = format!("{:?}", e)
            );
            drop(root_log);
            std::process::exit(1);
        }
    }
}
