use proc_macro2::TokenStream;
use quote::{
    format_ident,
    quote,
};
use super::sourceschema::{
    ScalarTypeKey,
    ValueBehaviorHelper,
};

pub fn generate_simple_type(t: &ScalarTypeKey) -> TokenStream {
    match t {
        ScalarTypeKey::Number => quote!(Primitive < f64 >),
        ScalarTypeKey::Integer => quote!(Primitive < i64 >),
        ScalarTypeKey::String => quote!(Primitive < String >),
        ScalarTypeKey::Bool => quote!(Primitive < bool >),
    }
}

pub fn add_path(v: &Vec<String>, e: &str) -> Vec<String> {
    let mut out = v.clone();
    for s in e.split("_") {
        out.push(s.to_string());
    }
    out
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

#[derive(Default)]
pub struct TopLevelFields {
    pub extra_types: Vec<TokenStream>,
    pub fields: Vec<TokenStream>,
    pub ref_methods: Vec<TokenStream>,
    pub mut_methods: Vec<TokenStream>,
    pub builder_fields: Vec<TokenStream>,
    pub copy_builder_fields: Vec<TokenStream>,
}

pub fn generate_field(
    out: &mut TopLevelFields,
    type_name: &str,
    k: &str,
    rusttype: TokenStream,
    field_doc: &str,
    behavior: ValueBehaviorHelper,
    generate_ref: bool,
    self_has_identity: bool,
) {
    let (sanitized, sanitized_name) = sanitize(k);
    let field_name = format_ident!("{}", sanitized_name);
    let set_field_name = format_ident!("set_{}", k);
    let set_doc = format!("Set the field `{}`.\n{}", field_name, field_doc);
    let ref_doc = format!("Get a reference to the value of field `{}` after provisioning.\n{}", field_name, field_doc);
    let refpat = format!("${{{{{}.{{}}.{}}}}}", type_name, k);
    match behavior {
        ValueBehaviorHelper::UserRequired => {
            out.builder_fields.push(quote!(#[doc = #field_doc] pub #field_name: #rusttype));
            out.copy_builder_fields.push(quote!(#field_name: self.#field_name));
            if sanitized {
                out.fields.push(quote!(#[serde(rename = #k)] #field_name: #rusttype));
            } else {
                out.fields.push(quote!(#field_name: #rusttype));
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
                                serde(rename = #k, skip_serializing_if = "Option::is_none")
                            ] #field_name: Option < #rusttype >
                        ),
                    );
            } else {
                out
                    .fields
                    .push(
                        quote!(#[serde(skip_serializing_if = "Option::is_none")] #field_name: Option < #rusttype >),
                    );
            }
            if self_has_identity {
                out
                    .mut_methods
                    .push(quote!(#[doc = #set_doc] pub fn #set_field_name(&self, v: impl Into < #rusttype >) ->& Self {
                        self.data.borrow_mut().#field_name = Some(v.into());
                        self
                    }));
            } else {
                out
                    .mut_methods
                    .push(
                        quote!(#[doc = #set_doc] pub fn #set_field_name(mut self, v: impl Into < #rusttype >) -> Self {
                            self.#field_name = Some(v.into());
                            self
                        }),
                    );
            }
        },
        ValueBehaviorHelper::Computed => {
            // nop
        },
    }
    if self_has_identity && generate_ref {
        out.ref_methods.push(quote!(#[doc = #ref_doc] pub fn #field_name(&self, stack: &mut Stack) -> #rusttype {
            Primitive::Sentinel(stack.add_sentinel(format!(#refpat, self.tf_id)))
        }));
    }
}
