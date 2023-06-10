use std::fmt::Write;
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{Data, DataStruct, parse_macro_input, DeriveInput};

fn create_struct_body(_ident: &syn::Ident,
                      _attrs: &Vec<syn::Attribute>,
                      fields: syn::Fields) -> String {

    let mut code = String::with_capacity(1024);
    write!(&mut code, "write!(w, \"{{{{\")?;").unwrap();
    for (i, field) in fields.iter().enumerate() {
        let key = field.ident.as_ref()
            .expect("anonymous struct field names are not supported")
            .to_string();
        if i == 0 {
            write!(&mut code, "write!(w, \"{{:?}}: \", {:?})?;", key).unwrap();
        } else {
            write!(&mut code, "write!(w, \", {{:?}}: \", {:?})?;", key).unwrap();
        }

        write!(&mut code, "JSONifyable::to_json(&self.{}, w)?;", key).unwrap();
    }
    write!(&mut code, "write!(w, \"}}}}\")?; Ok(())").unwrap();
    code
}

#[proc_macro_derive(JSONifyable)]
pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, attrs, data, .. } = parse_macro_input!(input);
    let body: proc_macro2::TokenStream = (match data {
        Data::Struct(DataStruct { fields, .. }) => create_struct_body(&ident, &attrs, fields),
        _ => panic!("Deriving JSONifyable for non-struct types is unimplemented")
    }).parse().unwrap();

    let output = quote! {
        impl JSONifyable for #ident {
            fn to_json(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result {
                #body
            }
        }
    };
    output.into()
}


