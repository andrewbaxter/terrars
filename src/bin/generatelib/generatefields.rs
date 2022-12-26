use std::collections::BTreeMap;
use proc_macro2::TokenStream;
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
) -> TokenStream {
    let name = format_ident!("{}", to_camel(path));
    let mut fields = vec![];
    for (field_name, subtype) in &at.1 {
        let (sanitized, sanitized_name) = sanitize(field_name);
        let field_ident = format_ident!("{}", sanitized_name);
        let rusttype = generate_type(extra_types, &add_path(path, field_name), (Some(subtype), None));
        if sanitized {
            fields.push(
                quote!(
                    #[
                        serde(rename = #field_name, skip_serializing_if = "Option::is_none")
                    ] #field_ident: Option < #rusttype >
                ),
            )
        } else {
            fields.push(
                quote!(#[serde(skip_serializing_if = "Option::is_none")] #field_ident: Option < #rusttype >),
            )
        }
    }
    extra_types.push(quote!(#[derive(Serialize)] pub struct #name {
        #(#fields),
        *
    }));
    quote!(#name)
}

pub fn generate_obj_agg_type_nested(
    extra_types: &mut Vec<TokenStream>,
    path: &Vec<String>,
    at: &BTreeMap<String, Value>,
) -> TokenStream {
    let name = format_ident!("{}", to_camel(path));
    let mut fields = vec![];
    for (field_name, subtype) in at {
        let (sanitized, sanitized_name) = sanitize(field_name);
        let field_ident = format_ident!("{}", sanitized_name);
        let rusttype =
            generate_type(
                extra_types,
                &add_path(path, field_name),
                (subtype.r#type.as_ref(), subtype.nested_type.as_ref()),
            );
        if sanitized {
            fields.push(
                quote!(
                    #[
                        serde(rename = #field_name, skip_serializing_if = "Option::is_none")
                    ] #field_ident: Option < #rusttype >
                ),
            )
        } else {
            fields.push(
                quote!(#[serde(skip_serializing_if = "Option::is_none")] #field_ident: Option < #rusttype >),
            )
        }
    }
    extra_types.push(quote!(#[derive(Serialize)] pub struct #name {
        #(#fields),
        *
    }));
    quote!(#name)
}

fn generate_coll_agg_type(extra_types: &mut Vec<TokenStream>, path: &Vec<String>, at: &AggCollType) -> TokenStream {
    match at.0 {
        AggCollTypeKey::Set => {
            let element_type = match &at.1 {
                ValueSchema::Simple(t) => generate_simple_type(&t),
                ValueSchema::AggColl(a) => {
                    generate_coll_agg_type(extra_types, &add_path(&path, "el"), a.as_ref())
                },
                ValueSchema::AggObject(a) => {
                    generate_obj_agg_type(extra_types, &add_path(&path, "el"), a.as_ref())
                },
            };
            quote!(std:: vec:: Vec < #element_type >)
        },
        AggCollTypeKey::List => {
            let element_type = match &at.1 {
                ValueSchema::Simple(t) => generate_simple_type(&t),
                ValueSchema::AggColl(a) => {
                    generate_coll_agg_type(extra_types, &add_path(&path, "el"), a.as_ref())
                },
                ValueSchema::AggObject(a) => {
                    generate_obj_agg_type(extra_types, &add_path(&path, "el"), a.as_ref())
                },
            };
            quote!(std:: vec:: Vec < #element_type >)
        },
        AggCollTypeKey::Map => {
            let element_type = match &at.1 {
                ValueSchema::Simple(t) => generate_simple_type(&t),
                ValueSchema::AggColl(_) => {
                    panic!("supposedly not supported by terraform")
                },
                ValueSchema::AggObject(_) => {
                    panic!("supposedly not supported by terraform")
                },
            };
            quote!(std::collections::HashMap < String, #element_type >)
        },
    }
}

fn generate_type(
    extra_types: &mut Vec<TokenStream>,
    path: &Vec<String>,
    at: (Option<&ValueSchema>, Option<&ValueSchemaNested>),
) -> TokenStream {
    match at {
        (Some(ValueSchema::Simple(t)), None) => generate_simple_type(t),
        (Some(ValueSchema::AggColl(at)), None) => generate_coll_agg_type(extra_types, path, at.as_ref()),
        (Some(ValueSchema::AggObject(at)), None) => generate_obj_agg_type(extra_types, path, at.as_ref()),
        (None, Some(x)) => match x.nesting_mode {
            super::sourceschema::NestingMode::List => {
                let element_type = generate_obj_agg_type_nested(extra_types, &add_path(&path, "el"), &x.attributes);
                quote!(std:: vec:: Vec < #element_type >)
            },
            super::sourceschema::NestingMode::Set => {
                let element_type = generate_obj_agg_type_nested(extra_types, &add_path(&path, "el"), &x.attributes);
                quote!(std:: vec:: Vec < #element_type >)
            },
            super::sourceschema::NestingMode::Single => unreachable!(),
        },
        (None, None) | (Some(_), Some(_)) => unreachable!(),
    }
}

pub fn generate_fields(
    out: &mut TopLevelFields,
    type_name: &str,
    path: &Vec<String>,
    fields: &BTreeMap<String, Value>,
    self_has_identity: bool,
) {
    for (k, v) in fields {
        let mut path = path.clone();
        path.extend(k.split("_").map(ToString::to_string));
        let rusttype = generate_type(&mut out.extra_types, &path, (v.r#type.as_ref(), v.nested_type.as_ref()));

        // Only generate readers for primitive fields -- I'm not sure where collection fields would
        // be useful (without for-each) and references only work with Primitive atm (which doesn't
        // support collections -- not sure how hard that would be to add, although it could be
        // simple).
        //
        // * Note that this may cause some unused types to be generated here (computed collections, not
        //    used above and not used below)
        let generate_ref = self_has_identity && match (&v.r#type, &v.nested_type) {
            (Some(ValueSchema::Simple(_)), None) => true,
            (Some(ValueSchema::AggColl(_)), None) => false,
            (Some(ValueSchema::AggObject(_)), None) => false,
            (None, Some(_)) => false,
            (None, None) | (Some(_), Some(_)) => unreachable!(),
        };
        generate_field(
            out,
            type_name,
            k,
            rusttype,
            &v.description.as_ref().map(|s| s.clone()).unwrap_or_else(String::new),
            v.behavior(),
            generate_ref,
            self_has_identity,
        );
    }
}
