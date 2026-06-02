use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;

use crate::config::EntityConfig;
use crate::paths::cache_entity_trait_path;

pub(crate) fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
    let config = EntityConfig::from_attrs(&input.attrs)?;
    let entity = config.required_entity(&input)?;
    let id = config.required_id(&input)?;
    let collection = config.collection_tokens();
    let trait_path = cache_entity_trait_path();
    let ident = input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #trait_path for #ident #type_generics #where_clause {
            type Id = #id;

            const ENTITY: &'static str = #entity;
            const COLLECTION: Option<&'static str> = #collection;
        }
    })
}

#[cfg(test)]
mod tests {
    use syn::{parse_quote, DeriveInput};

    use super::*;

    #[test]
    fn rejects_missing_entity() {
        let input: DeriveInput = parse_quote! {
            #[hydracache(id = i64)]
            struct User;
        };

        let error = expand(input).unwrap_err();

        assert!(error
            .to_string()
            .contains("missing #[hydracache(entity = \"...\")]"));
    }

    #[test]
    fn rejects_missing_id() {
        let input: DeriveInput = parse_quote! {
            #[hydracache(entity = "user")]
            struct User;
        };

        let error = expand(input).unwrap_err();

        assert!(error
            .to_string()
            .contains("missing #[hydracache(id = Type)]"));
    }

    #[test]
    fn expands_impl_for_generics() {
        let input: DeriveInput = parse_quote! {
            #[hydracache(entity = "wrapper", id = String)]
            struct Wrapper<T>
            where
                T: Clone,
            {
                value: T,
            }
        };

        let output = expand(input).unwrap().to_string();

        assert!(output.contains("impl < T >"));
        assert!(output.contains("CacheEntity for Wrapper < T >"));
        assert!(output.contains("type Id = String"));
        assert!(output.contains("const ENTITY"));
        assert!(output.contains("\"wrapper\""));
        assert!(output.contains("const COLLECTION"));
        assert!(output.contains("None"));
    }
}
