extern crate proc_macro;
extern crate syn;
extern crate quote;

use std::fmt::Write;
use proc_macro::TokenStream;

#[proc_macro_derive(MatchVariants)]
pub fn derive_match_variants(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let enumast = match ast.data {
        syn::Data::Enum(ast) => ast,
        _ => panic!("#[derive(MatchVariants)] only works for enums")
    };

    let mut code = String::new();
    write!(&mut code, "impl {} {{ pub fn match_variants(&self, other: &Self) -> bool {{ match (self, other) {{", name.to_string()).unwrap();

    let mut buf = String::new();
    for variant in enumast.variants {
        let name = &variant.ident.to_string();
        let num_fields = variant.fields.len();
        if num_fields == 0 {
            write!(&mut code, "(Self::{}, Self::{}) => true,", name, name).unwrap();
            continue;
        }

        for i in 0..num_fields {
            if i != 0 { buf += ", "; }
            buf += "_";
        }

        write!(&mut code, "(Self::{}({}), Self::{}({})) => true,", name, buf, name, buf).unwrap();
        buf.clear();
    }

    write!(&mut code, "(_, _) => false}}}}}}").unwrap();
    code.parse().unwrap()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
