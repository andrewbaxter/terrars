use std::collections::{
    BTreeMap,
    HashMap,
    hash_map::Entry,
};
use proc_macro2::{
    Group,
    Ident,
    TokenStream,
    TokenTree,
};
use quote::{
    format_ident,
    quote,
};
use super::sourceschema::{
    ScalarTypeKey,
    ValueBehaviorHelper,
    Value,
    ValueSchema,
    ValueSchemaNested,
    AggObjType,
    AggCollType,
    AggCollTypeKey,
    NestedBlock,
    Block,
    NestingMode,
};

pub fn generate_simple_type(t: &ScalarTypeKey) -> (TokenStream, Option<(TokenStream, TokenStream)>) {
    let raw = match t {
        ScalarTypeKey::Number => quote!(f64),
        ScalarTypeKey::Integer => quote!(i64),
        ScalarTypeKey::String => quote!(std::string::String),
        ScalarTypeKey::Bool => quote!(bool),
        ScalarTypeKey::Dynamic => quote!(serde_json::Value),
    };
    (
        quote!(terrars:: PrimField < #raw >),
        Some((quote!(terrars:: PrimExpr), quote!(terrars:: PrimExpr < #raw >))),
    )
}

pub fn path_camel(path: &[String]) -> String {
    path
        .iter()
        .map(|level| to_camel(&level.split("_").filter(|s| !s.is_empty()).map(ToString::to_string).collect::<Vec<_>>()))
        .collect::<Vec<_>>()
        .join("_")
}

pub fn to_camel(v: &[String]) -> String {
    v.iter().map(|s| format!("{}{}", (&s[..1].to_string()).to_uppercase(), &s[1..])).collect()
}

pub fn to_snake(v: &[String]) -> String {
    v.as_ref().join("_")
}

pub fn sanitize(v: &str) -> (bool, String) {
    match v {
        "as" |
        "break" |
        "const" |
        "continue" |
        "crate" |
        "else" |
        "enum" |
        "extern" |
        "false" |
        "fn" |
        "for" |
        "if" |
        "impl" |
        "in" |
        "let" |
        "loop" |
        "match" |
        "mod" |
        "move" |
        "mut" |
        "pub" |
        "ref" |
        "return" |
        "self" |
        "Self" |
        "static" |
        "struct" |
        "super" |
        "trait" |
        "true" |
        "type" |
        "unsafe" |
        "use" |
        "where" |
        "while" |
        "async" |
        "await" |
        "dyn" |
        "abstract" |
        "become" |
        "box" |
        "do" |
        "final" |
        "macro" |
        "override" |
        "priv" |
        "typeof" |
        "unsized" |
        "virtual" |
        "yield" |
        "try" => (
            true,
            format!("{}_", v),
        ),
        s => (false, s.into()),
    }
}

#[derive(Default, Clone)]
pub struct TopLevelFields {
    pub extra_types: Vec<TokenStream>,
    pub fields: Vec<TokenStream>,
    pub ref_methods: Vec<TokenStream>,
    pub ref_ref_methods: Vec<TokenStream>,
    pub mut_methods: Vec<TokenStream>,
    pub builder_fields: Vec<TokenStream>,
    pub copy_builder_fields: Vec<TokenStream>,
    pub dynamic_block_fields: Vec<TokenStream>,
}

impl TopLevelFields {
    pub fn finish(&mut self, camel_name: &str) {
        if !self.dynamic_block_fields.is_empty() {
            let dynamic_ident = format_ident!("{}Dynamic", camel_name);
            let dynamic_fields = self.dynamic_block_fields.split_off(0);
            self.extra_types.push(quote!{
                #[derive(Serialize, Default)] struct #dynamic_ident {
                    #(#dynamic_fields,) *
                }
            });
            self.fields.push(quote!{
                dynamic: #dynamic_ident
            });
            self.copy_builder_fields.push(quote!{
                dynamic: core::default::Default::default()
            });
        };
    }
}

pub fn generate_field(
    out: &mut TopLevelFields,
    k: &str,
    rust_field_type: TokenStream,
    rust_field_ref_type: Option<(TokenStream, TokenStream)>,
    field_doc: &str,
    behavior: ValueBehaviorHelper,
    self_has_identity: bool,
    block: Option<TokenStream>,
) {
    let (sanitized, sanitized_name) = sanitize(k);
    let field_name = format_ident!("{}", sanitized_name);
    let set_field_name = format_ident!("set_{}", k);
    let set_doc = format!("Set the field `{}`.\n{}", field_name, field_doc);
    let ref_doc = format!("Get a reference to the value of field `{}` after provisioning.\n{}", field_name, field_doc);
    match behavior {
        ValueBehaviorHelper::UserRequired => {
            out.builder_fields.push(quote!(#[doc = #field_doc] pub #field_name: #rust_field_type));
            out.copy_builder_fields.push(quote!(#field_name: self.#field_name));
            if sanitized {
                out.fields.push(quote!(#[serde(rename = #k)] #field_name: #rust_field_type));
            } else {
                out.fields.push(quote!(#field_name: #rust_field_type));
            }
        },
        ValueBehaviorHelper::UserOptional | ValueBehaviorHelper::UserOptionalComputed => {
            out.copy_builder_fields.push(quote!(#field_name: core:: default:: Default:: default()));
            if sanitized {
                out
                    .fields
                    .push(
                        quote!(
                            #[
                                serde(rename = #k, skip_serializing_if = "std::option::Option::is_none")
                            ] #field_name: std::option::Option < #rust_field_type >
                        ),
                    );
            } else {
                out
                    .fields
                    .push(
                        quote!(
                            #[serde(skip_serializing_if = "std::option::Option::is_none")] #field_name: std::option::Option < #rust_field_type >
                        ),
                    );
            }
            let pat_mut_self;
            let access_mut_self;
            if self_has_identity {
                pat_mut_self = quote!(self);
                access_mut_self = quote!(self.0.data.borrow_mut());
            } else {
                pat_mut_self = quote!(mut self);
                access_mut_self = quote!(self);
            }
            if let Some(block_type) = block {
                out.dynamic_block_fields.push(quote!(#field_name: std::option::Option < terrars:: DynamicBlock < #block_type >>));
                out
                    .mut_methods
                    .push(
                        quote!(
                            #[
                                doc = #set_doc
                            ] pub fn #set_field_name(
                                #pat_mut_self,
                                v: impl std::convert::Into < terrars:: BlockAssignable < #block_type >>
                            ) -> Self {
                                match v.into() {
                                    terrars:: BlockAssignable:: Literal(v) => {
                                        #access_mut_self.#field_name = std::option::Option::Some(v);
                                    },
                                    terrars:: BlockAssignable:: Dynamic(d) => {
                                        #access_mut_self.dynamic.#field_name = std::option::Option::Some(d);
                                    }
                                }
                                self
                            }
                        ),
                    );
            } else {
                out
                    .mut_methods
                    .push(
                        quote!(
                            #[
                                doc = #set_doc
                            ] pub fn #set_field_name(#pat_mut_self, v: impl std::convert::Into < #rust_field_type >) -> Self {
                                #access_mut_self.#field_name = std::option::Option::Some(v.into());
                                self
                            }
                        ),
                    );
            }
        },
        ValueBehaviorHelper::Computed => {
            // nop
        },
    }
    if let Some((t1, t2)) = rust_field_ref_type {
        if self_has_identity {
            let ref_fmt = format!("{{}}.{}", k);
            out.ref_methods.push(quote!(#[doc = #ref_doc] pub fn #field_name(&self) -> #t2 {
                #t1:: new(self.shared().clone(), format!(#ref_fmt, self.extract_ref()))
            }));
        }
        let ref_ref_fmt = format!("{{}}.{}", k);
        out.ref_ref_methods.push(quote!(#[doc = #ref_doc] pub fn #field_name(&self) -> #t2 {
            #t1:: new(self.shared().clone(), format!(#ref_ref_fmt, self.base))
        }));
    }
}

fn generate_type(
    types: &mut NestedTypes,
    path: &Vec<String>,
    at: (Option<&ValueSchema>, Option<&ValueSchemaNested>),
) -> (TokenStream, Option<(TokenStream, TokenStream)>) {
    match at {
        (Some(ValueSchema::Simple(t)), None) => generate_simple_type(t),
        (Some(ValueSchema::AggColl(at)), None) => generate_agg_type_coll(types, path, at.as_ref()),
        (Some(ValueSchema::AggObj(at)), None) => generate_agg_type_obj(types, path, at.as_ref()),
        (None, Some(x)) => match x.nesting_mode {
            super::sourceschema::NestingMode::List => {
                let (element_type, element_ref_type) =
                    generate_agg_type_obj_nested(types, path, &x.attributes);
                (
                    quote!(std::vec::Vec < #element_type >),
                    element_ref_type.map(|(_, r2)| (quote!(terrars:: ListRef), quote!(terrars:: ListRef < #r2 >))),
                )
            },
            super::sourceschema::NestingMode::Set => {
                let (element_type, element_ref_type) =
                    generate_agg_type_obj_nested(types, path, &x.attributes);
                (
                    quote!(std::vec::Vec < #element_type >),
                    element_ref_type.map(|(_, r2)| (quote!(terrars:: SetRef), quote!(terrars:: SetRef < #r2 >))),
                )
            },
            super::sourceschema::NestingMode::Single | super::sourceschema::NestingMode::Group => {
                let (element_type, element_ref_type) =
                    generate_agg_type_obj_nested(types, path, &x.attributes);
                (element_type, element_ref_type)
            },
            super::sourceschema::NestingMode::Map => {
                let (element_type, element_ref_type) =
                    generate_agg_type_obj_nested(types, path, &x.attributes);
                (
                    quote!(std::collections::HashMap < std::string::String, #element_type >),
                    element_ref_type.map(|(_, r2)| (quote!(terrars:: RecRef), quote!(terrars:: RecRef < #r2 >))),
                )
            },
        },
        (None, None) | (Some(_), Some(_)) => unreachable!(),
    }
}

pub fn generate_agg_type_obj(
    types: &mut NestedTypes,
    path: &Vec<String>,
    at: &AggObjType,
) -> (TokenStream, Option<(TokenStream, TokenStream)>) {
    let mut raw_fields = TopLevelFields::default();
    generate_fields_from_valueschema_map(types, &mut raw_fields, &path, &at.1, false);
    let (rust_type, rust_ref_type) = generate_nonident_rust_type(types, path, raw_fields);
    (rust_type, Some((rust_ref_type.clone(), rust_ref_type)))
}

pub fn generate_agg_type_obj_nested(
    types: &mut NestedTypes,
    path: &Vec<String>,
    at: &BTreeMap<String, Value>,
) -> (TokenStream, Option<(TokenStream, TokenStream)>) {
    let mut raw_fields = TopLevelFields::default();
    generate_fields_from_value_map(types, &mut raw_fields, &path, &at, false);
    let (rust_type, rust_ref_type) = generate_nonident_rust_type(types, path, raw_fields);
    (rust_type, Some((rust_ref_type.clone(), rust_ref_type)))
}

fn generate_agg_type_coll(
    types: &mut NestedTypes,
    path: &Vec<String>,
    at: &AggCollType,
) -> (TokenStream, Option<(TokenStream, TokenStream)>) {
    match at.0 {
        AggCollTypeKey::List => {
            let (element_type, element_ref_type) = match &at.1 {
                ValueSchema::Simple(t) => generate_simple_type(&t),
                ValueSchema::AggColl(a) => generate_agg_type_coll(types, path, a.as_ref()),
                ValueSchema::AggObj(a) => generate_agg_type_obj(types, path, a.as_ref()),
            };
            (
                quote!(terrars:: ListField < #element_type >),
                element_ref_type.map(|(_, r2)| (quote!(terrars:: ListRef), quote!(terrars:: ListRef < #r2 >))),
            )
        },
        AggCollTypeKey::Set => {
            let (element_type, element_ref_type) = match &at.1 {
                ValueSchema::Simple(t) => generate_simple_type(&t),
                ValueSchema::AggColl(a) => generate_agg_type_coll(types, path, a.as_ref()),
                ValueSchema::AggObj(a) => generate_agg_type_obj(types, path, a.as_ref()),
            };
            (
                quote!(terrars:: SetField < #element_type >),
                element_ref_type.map(|(_, r2)| (quote!(terrars:: SetRef), quote!(terrars:: SetRef < #r2 >))),
            )
        },
        AggCollTypeKey::Rec => {
            let (element_type, element_ref_type) = match &at.1 {
                ValueSchema::Simple(t) => generate_simple_type(&t),
                ValueSchema::AggColl(a) => generate_agg_type_coll(types, path, a.as_ref()),
                ValueSchema::AggObj(a) => generate_agg_type_obj(types, path, a.as_ref()),
            };
            (
                quote!(terrars:: RecField < #element_type >),
                element_ref_type.map(|(_, r2)| (quote!(terrars:: RecRef), quote!(terrars:: RecRef < #r2 >))),
            )
        },
    }
}

fn generate_block_agg_obj(types: &mut NestedTypes, path: &Vec<String>, obj: &Block) -> (TokenStream, TokenStream) {
    let mut raw_fields = TopLevelFields::default();
    generate_fields_from_value_map(types, &mut raw_fields, &path, &obj.attributes, false);
    generate_block_fields(types, &mut raw_fields, &path, &obj.block_types, false);
    generate_nonident_rust_type(types, path, raw_fields)
}

pub fn generate_block_fields(
    types: &mut NestedTypes,
    out: &mut TopLevelFields,
    path: &Vec<String>,
    fields: &BTreeMap<String, NestedBlock>,
    self_has_identity: bool,
) {
    for (k, v) in fields {
        let mut path = path.clone();
        path.push(k.clone());
        let rust_type;
        let rust_ref_type;
        let block_type;
        match v.nesting_mode {
            NestingMode::List => {
                let (element_type, element_ref_type) =
                    generate_block_agg_obj(types, &path, &v.block);
                rust_type = quote!(std::vec::Vec < #element_type >);
                rust_ref_type = Some((quote!(terrars:: ListRef), quote!(terrars:: ListRef < #element_ref_type >)));
                block_type = Some(element_type);
            },
            NestingMode::Set => {
                let (element_type, _) = generate_block_agg_obj(types, &path, &v.block);
                rust_type = quote!(std::vec::Vec < #element_type >);
                rust_ref_type = None;
                block_type = Some(element_type);
            },
            NestingMode::Single | NestingMode::Group => {
                let (element_type, element_ref_type) =
                    generate_block_agg_obj(types, &path, &v.block);
                rust_type = element_type;
                rust_ref_type = Some((element_ref_type.clone(), element_ref_type));
                block_type = None;
            },
            NestingMode::Map => {
                let (element_type, element_ref_type) =
                    generate_block_agg_obj(types, &path, &v.block);
                rust_type = quote!(std::collections::HashMap < std::string::String, #element_type >);
                rust_ref_type = Some((quote!(terrars:: RecRef), quote!(terrars:: RecRef < #element_ref_type >)));
                block_type = None;
            },
        };
        generate_field(
            out,
            k,
            rust_type,
            rust_ref_type,
            "",
            super::sourceschema::ValueBehaviorHelper::UserOptional,
            self_has_identity,
            block_type,
        );
    }
}

pub fn generate_fields_from_value_map(
    types: &mut NestedTypes,
    out: &mut TopLevelFields,
    path: &Vec<String>,
    fields: &BTreeMap<String, Value>,
    self_has_identity: bool,
) {
    for (k, v) in fields {
        let mut path = path.clone();
        path.push(k.clone());
        let (rust_field_type, rust_field_ref_type) =
            generate_type(types, &path, (v.r#type.as_ref(), v.nested_type.as_ref()));
        generate_field(
            out,
            k,
            rust_field_type,
            rust_field_ref_type,
            &v.description.as_ref().map(|s| s.clone()).unwrap_or_else(String::new),
            v.behavior(),
            self_has_identity,
            None,
        );
    }
}

pub fn generate_fields_from_valueschema_map(
    types: &mut NestedTypes,
    out: &mut TopLevelFields,
    path: &Vec<String>,
    fields: &BTreeMap<String, ValueSchema>,
    self_has_identity: bool,
) {
    for (k, v) in fields {
        let mut path = path.clone();
        path.push(k.clone());
        let (rust_field_type, rust_field_ref_type) = generate_type(types, &path, (Some(v), None));
        generate_field(
            out,
            k,
            rust_field_type,
            rust_field_ref_type,
            "",
            super::sourceschema::ValueBehaviorHelper::UserOptional,
            self_has_identity,
            None,
        );
    }
}


const PLACEHOLDER_PREFIX: &str = "TerrarsNested";

fn placeholder(id: usize) -> String {
    format!("{}{}_", PLACEHOLDER_PREFIX, id)
}

/// Nested (non-identity) types for one output file, deduplicated by structure. Deep
/// schemas (ex: wafv2 statements) repeat the same blocks at many paths, and generating a
/// type per path blows up the output.
///
/// Types are generated with placeholder names since the final name (the shortest path
/// with that shape) isn't known until everything's generated. `resolve` replaces them.
#[derive(Default)]
pub struct NestedTypes {
    by_shape: HashMap<String, usize>,
    names: Vec<String>,
    types: Vec<TokenStream>,
}

impl NestedTypes {
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Only blocks with the same name are merged, otherwise unrelated blocks that happen to
    /// have the same fields end up sharing a misleading name.
    fn add(&mut self, path: &[String], render: impl Fn(&str) -> TokenStream) -> String {
        let camel_name = path_camel(path);
        let shape =
            format!(
                "{}\n{}",
                path.last().map(String::as_str).unwrap_or(""),
                render(&format!("{}Self_", PLACEHOLDER_PREFIX))
            );
        match self.by_shape.entry(shape) {
            Entry::Occupied(e) => {
                let id = *e.get();
                let name = &mut self.names[id];
                if (camel_name.len(), &camel_name) < (name.len(), name) {
                    *name = camel_name;
                }
                placeholder(id)
            },
            Entry::Vacant(e) => {
                let id = self.types.len();
                e.insert(id);
                self.names.push(camel_name);
                self.types.push(render(&placeholder(id)));
                placeholder(id)
            },
        }
    }

    fn resolve_ident(&self, i: &str) -> Option<String> {
        let start = i.find(PLACEHOLDER_PREFIX)?;
        let rest = &i[start + PLACEHOLDER_PREFIX.len()..];
        let digits = rest.find(|c: char| !c.is_ascii_digit())?;
        let id: usize = rest[..digits].parse().ok()?;
        let suffix = rest[digits..].strip_prefix("_")?;
        Some(format!("{}{}{}", &i[..start], self.names[id], suffix))
    }

    /// Replace placeholder names in generated code with the final names.
    pub fn resolve(&self, tokens: TokenStream) -> TokenStream {
        tokens.into_iter().map(|t| match t {
            TokenTree::Group(g) => {
                let mut out = Group::new(g.delimiter(), self.resolve(g.stream()));
                out.set_span(g.span());
                TokenTree::Group(out)
            },
            TokenTree::Ident(i) => match self.resolve_ident(&i.to_string()) {
                Some(name) => TokenTree::Ident(Ident::new(&name, i.span())),
                None => TokenTree::Ident(i),
            },
            t => t,
        }).collect()
    }

    /// The generated type definitions, with final names.
    pub fn finish(self) -> Vec<TokenStream> {
        self.types.iter().map(|t| self.resolve(t.clone())).collect()
    }
}

pub fn generate_nonident_rust_type(
    types: &mut NestedTypes,
    path: &Vec<String>,
    raw_fields: TopLevelFields,
) -> (TokenStream, TokenStream) {
    let name = types.add(&path, |camel_name| {
        let mut raw_fields = raw_fields.clone();
        raw_fields.finish(camel_name);
        let extra_types = raw_fields.extra_types;
        let builder_fields = raw_fields.builder_fields;
        let copy_builder_fields = raw_fields.copy_builder_fields;
        let resource_fields = raw_fields.fields;
        let resource_mut_methods = raw_fields.mut_methods;
        let ref_ref_methods = raw_fields.ref_ref_methods;
        let obj_ident = format_ident!("{}", camel_name);
        let obj_builder_ident = format_ident!("Build{}", camel_name);
        let obj_ref_ident = format_ident!("{}Ref", camel_name);
        quote!{
            #(#extra_types) * #[derive(Serialize)] pub struct #obj_ident {
                #(#resource_fields,) *
            }
            impl #obj_ident {
                #(#resource_mut_methods) *
            }
            impl terrars::ToListMappable for #obj_ident {
                type O = terrars::BlockAssignable < #obj_ident >;
                fn do_map(self, base: std::string::String) -> Self::O {
                    terrars::BlockAssignable::Dynamic(terrars::DynamicBlock {
                        for_each: format!("${{{}}}", base),
                        iterator: "each".into(),
                        content: self,
                    })
                }
            }
            pub struct #obj_builder_ident {
                #(#builder_fields,) *
            }
            impl #obj_builder_ident {
                pub fn build(self) -> #obj_ident {
                    #obj_ident {
                        #(#copy_builder_fields,) *
                    }
                }
            }
            #[allow(dead_code)] pub struct #obj_ref_ident {
                shared: terrars::StackShared,
                base: std::string::String
            }
            impl terrars::Ref for #obj_ref_ident {
                fn new(shared: terrars::StackShared, base: std::string::String) -> #obj_ref_ident {
                    #obj_ref_ident {
                        shared: shared,
                        base: base.to_string(),
                    }
                }
            }
            impl #obj_ref_ident {
                #[allow(dead_code)] fn shared(&self) -> &terrars::StackShared {
                    &self.shared
                }
                #(#ref_ref_methods) *
            }
        }
    });
    let obj_ident = format_ident!("{}", name);
    let obj_ref_ident = format_ident!("{}Ref", name);
    (quote!(#obj_ident), quote!(#obj_ref_ident))
}
