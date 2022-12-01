use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeMap;

use super::{
    generatefields::generate_fields,
    generateshared::{add_path, generate_field, to_camel, to_snake, TopLevelFields},
    sourceschema::{Block, NestedBlock, NestingMode},
};

fn generate_block_agg_obj(
    superout: &mut TopLevelFields,
    path: &Vec<String>,
    obj: &Block,
) -> TokenStream {
    let camel_name = to_camel(&path);
    let snake_name = to_snake(&path);

    let mut raw_fields = TopLevelFields::default();
    generate_fields(&mut raw_fields, &snake_name, &path, &obj.attributes, false);
    generate_block_fields(&mut raw_fields, &snake_name, &path, &obj.block_types, false);
    let builder_fields = raw_fields.builder_fields;
    let copy_builder_fields = raw_fields.copy_builder_fields;
    superout.extra_types.extend(raw_fields.extra_types);
    let resource_fields = raw_fields.fields;
    let resource_mut_methods = raw_fields.mut_methods;

    let obj_ident = format_ident!("{}", camel_name);
    let obj_builder_ident = format_ident!("Build{}", camel_name);
    superout.extra_types.push(quote! {
        #[derive(Serialize)]
        pub struct #obj_ident {
            #(#resource_fields,)*
        }

        impl #obj_ident {
            #(#resource_mut_methods)*
        }

        pub struct #obj_builder_ident {
            #(#builder_fields,)*
        }

        impl #obj_builder_ident {
            pub fn build(self) -> #obj_ident {
                #obj_ident {
                    #(#copy_builder_fields,)*
                }
            }
        }
    });
    quote!(#obj_ident)
}

pub fn generate_block_fields(
    out: &mut TopLevelFields,
    type_name: &str,
    path: &Vec<String>,
    fields: &BTreeMap<String, NestedBlock>,
    self_has_identity: bool,
) {
    for (k, v) in fields {
        let mut path = path.clone();
        path.extend(k.split("_").map(ToString::to_string));
        let rusttype = match v.nesting_mode {
            NestingMode::List => {
                let element_type = generate_block_agg_obj(out, &add_path(&path, "el"), &v.block);
                quote!(std::vec::Vec<#element_type>)
            }
            NestingMode::Set => {
                let element_type = generate_block_agg_obj(out, &add_path(&path, "el"), &v.block);
                quote!(std::vec::Vec<#element_type>)
            }
            NestingMode::Single => generate_block_agg_obj(out, &add_path(&path, "el"), &v.block),
        };
        generate_field(
            out,
            type_name,
            k,
            rusttype,
            "",
            super::sourceschema::ValueBehaviorHelper::UserOptional,
            false,
            self_has_identity,
        );
    }
}
