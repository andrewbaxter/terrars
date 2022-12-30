use std::collections::BTreeMap;
use proc_macro2::{
    TokenStream,
};
use quote::{
    format_ident,
    quote,
};
use super::{
    generateshared::{
        add_path,
        generate_field,
        generate_simple_type,
        sanitize,
        to_camel,
        TopLevelFields,
    },
    sourceschema::{
        AggCollType,
        AggCollTypeKey,
        AggObjType,
        Value,
        ValueSchema,
        ValueSchemaNested,
    },
};

pub fn generate_obj_agg_type(
    extra_types: &mut Vec<TokenStream>,
    path: &Vec<String>,
    at: &AggObjType,
) -> (TokenStream, Option<(TokenStream, TokenStream)>) {
    let name = format_ident!("{}", to_camel(path));
    let ref_name = format_ident!("{}Ref", name);
    let mut fields = vec![];
    let mut ref_methods = vec![];
    for (field_name, subtype) in &at.1 {
        let (sanitized, sanitized_name) = sanitize(field_name);
        let field_ident = format_ident!("{}", sanitized_name);
        let (rust_type, rust_ref_type) =
            generate_type(extra_types, &add_path(path, field_name), (Some(subtype), None));
        if sanitized {
            fields.push(
                quote!(
                    #[
                        serde(rename = #field_name, skip_serializing_if = "Option::is_none")
                    ] pub #field_ident: Option < #rust_type >
                ),
            );
        } else {
            fields.push(
                quote!(#[serde(skip_serializing_if = "Option::is_none")] pub #field_ident: Option < #rust_type >),
            );
        }
        if let Some((r1, r2)) = rust_ref_type {
            let ref_fmt = format!("{{}}.{}", field_name);
            ref_methods.push(quote!(fn #field_ident() -> #r2 {
                #r1:: new(format!(#ref_fmt, self.base))
            }));
        }
    }
    extra_types.push(quote!{
        #[derive(Serialize)] pub struct #name {
            #(#fields),
            *
        }
        pub struct #ref_name {
            base: String
        }
        impl #ref_name {
            #(#ref_methods) *
        }
        impl Ref for #ref_name {
            fn new(base: String) -> #ref_name {
                #ref_name {
                    base: base.to_string(),
                }
            }
        }
    });
    (quote!(#name), Some((quote!(#ref_name), quote!(#ref_name))))
}

pub fn generate_obj_agg_type_nested(
    extra_types: &mut Vec<TokenStream>,
    path: &Vec<String>,
    at: &BTreeMap<String, Value>,
) -> (TokenStream, Option<(TokenStream, TokenStream)>) {
    let name = format_ident!("{}", to_camel(path));
    let ref_name = format_ident!("{}Ref", name);
    let mut fields = vec![];
    let mut ref_fields = vec![];
    for (field_name, subtype) in at {
        let (sanitized, sanitized_name) = sanitize(field_name);
        let field_ident = format_ident!("{}", sanitized_name);
        let (rust_type, rust_ref_type) =
            generate_type(
                extra_types,
                &add_path(path, field_name),
                (subtype.r#type.as_ref(), subtype.nested_type.as_ref()),
            );
        match subtype.behavior() {
            super::sourceschema::ValueBehaviorHelper::Computed => { },
            super::sourceschema::ValueBehaviorHelper::UserRequired => {
                if sanitized {
                    fields.push(quote!(#[serde(rename = #field_name)] pub #field_ident: #rust_type));
                } else {
                    fields.push(quote!(pub #field_ident: #rust_type));
                }
            },
            super::sourceschema::ValueBehaviorHelper::UserOptional |
            super::sourceschema::ValueBehaviorHelper::UserOptionalComputed => {
                if sanitized {
                    fields.push(
                        quote!(
                            #[
                                serde(rename = #field_name, skip_serializing_if = "Option::is_none")
                            ] pub #field_ident: Option < #rust_type >
                        ),
                    );
                } else {
                    fields.push(
                        quote!(
                            #[serde(skip_serializing_if = "Option::is_none")] pub #field_ident: Option < #rust_type >
                        ),
                    );
                }
            },
        };
        if let Some((r1, r2)) = rust_ref_type {
            let ref_fmt = format!("{{}}.{}", field_name);
            ref_fields.push(quote!(fn #field_ident() -> #r2 {
                #r1:: new(format!(#ref_fmt, self.base))
            }));
        }
    }
    extra_types.push(quote!{
        #[derive(Serialize)] pub struct #name {
            #(#fields),
            *
        }
        pub struct #ref_name {
            base: String
        }
        impl #ref_name {
            #(#ref_fields) *
        }
        impl Ref for #ref_name {
            fn new(base: String) -> #ref_name {
                #ref_name {
                    base: base.to_string(),
                }
            }
        }
    });
    (quote!(#name), Some((quote!(#ref_name), quote!(#ref_name))))
}

fn generate_coll_agg_type(
    extra_types: &mut Vec<TokenStream>,
    path: &Vec<String>,
    at: &AggCollType,
) -> (TokenStream, Option<(TokenStream, TokenStream)>) {
    match at.0 {
        AggCollTypeKey::Set => {
            let (element_type, _) = match &at.1 {
                ValueSchema::Simple(t) => generate_simple_type(&t),
                ValueSchema::AggColl(a) => generate_coll_agg_type(extra_types, &add_path(&path, "el"), a.as_ref()),
                ValueSchema::AggObject(a) => generate_obj_agg_type(extra_types, &add_path(&path, "el"), a.as_ref()),
            };
            (quote!(std:: vec:: Vec < #element_type >), None)
        },
        AggCollTypeKey::List => {
            let (element_type, element_ref_type) = match &at.1 {
                ValueSchema::Simple(t) => generate_simple_type(&t),
                ValueSchema::AggColl(a) => generate_coll_agg_type(extra_types, &add_path(&path, "el"), a.as_ref()),
                ValueSchema::AggObject(a) => generate_obj_agg_type(extra_types, &add_path(&path, "el"), a.as_ref()),
            };
            (
                quote!(std:: vec:: Vec < #element_type >),
                element_ref_type.map(|(_, r2)| (quote!(ListRef), quote!(ListRef < #r2 >))),
            )
        },
        AggCollTypeKey::Map => {
            let (element_type, element_ref_type) = match &at.1 {
                ValueSchema::Simple(t) => generate_simple_type(&t),
                ValueSchema::AggColl(_) => {
                    panic!("supposedly not supported by terraform")
                },
                ValueSchema::AggObject(_) => {
                    panic!("supposedly not supported by terraform")
                },
            };
            (
                quote!(std::collections::HashMap < String, #element_type >),
                element_ref_type.map(|(_, r2)| (quote!(MapRef), quote!(MapRef < #r2 >))),
            )
        },
    }
}

fn generate_type(
    extra_types: &mut Vec<TokenStream>,
    path: &Vec<String>,
    at: (Option<&ValueSchema>, Option<&ValueSchemaNested>),
) -> (TokenStream, Option<(TokenStream, TokenStream)>) {
    match at {
        (Some(ValueSchema::Simple(t)), None) => generate_simple_type(t),
        (Some(ValueSchema::AggColl(at)), None) => generate_coll_agg_type(extra_types, path, at.as_ref()),
        (Some(ValueSchema::AggObject(at)), None) => generate_obj_agg_type(extra_types, path, at.as_ref()),
        (None, Some(x)) => match x.nesting_mode {
            super::sourceschema::NestingMode::List => {
                let (element_type, element_ref_type) =
                    generate_obj_agg_type_nested(extra_types, &add_path(&path, "el"), &x.attributes);
                (
                    quote!(Vec < #element_type >),
                    element_ref_type.map(|(_, r2)| (quote!(ListRef), quote!(ListRef < #r2 >))),
                )
            },
            super::sourceschema::NestingMode::Set => {
                let (element_type, _) =
                    generate_obj_agg_type_nested(extra_types, &add_path(&path, "el"), &x.attributes);
                (quote!(Vec < #element_type >), None)
            },
            super::sourceschema::NestingMode::Single => unreachable!(),
        },
        (None, None) | (Some(_), Some(_)) => unreachable!(),
    }
}

pub fn generate_fields(
    out: &mut TopLevelFields,
    path: &Vec<String>,
    fields: &BTreeMap<String, Value>,
    self_has_identity: bool,
) {
    for (k, v) in fields {
        let mut path = path.clone();
        path.extend(k.split("_").map(ToString::to_string));
        let (rust_field_type, rust_field_ref_type) =
            generate_type(&mut out.extra_types, &path, (v.r#type.as_ref(), v.nested_type.as_ref()));
        generate_field(
            out,
            k,
            rust_field_type,
            rust_field_ref_type,
            &v.description.as_ref().map(|s| s.clone()).unwrap_or_else(String::new),
            v.behavior(),
            self_has_identity,
        );
    }
}
