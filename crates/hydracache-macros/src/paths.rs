use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;

pub(crate) fn cache_entity_trait_path() -> TokenStream2 {
    crate_name("hydracache-db")
        .map(|found| cache_entity_path_for("hydracache_db", found))
        .or_else(|_| {
            crate_name("hydracache-sqlx")
                .map(|found| cache_entity_path_for("hydracache_sqlx", found))
        })
        .unwrap_or_else(|_| quote!(::hydracache_db::CacheEntity))
}

fn cache_entity_path_for(default_name: &str, found: FoundCrate) -> TokenStream2 {
    let crate_name = match found {
        FoundCrate::Itself => default_name.to_owned(),
        FoundCrate::Name(name) => name,
    };
    let ident = syn::Ident::new(&crate_name.replace('-', "_"), Span::call_site());

    quote!(::#ident::CacheEntity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_cache_entity_path_for_current_crate() {
        assert_eq!(
            cache_entity_path_for("hydracache_db", FoundCrate::Itself).to_string(),
            ":: hydracache_db :: CacheEntity"
        );
    }

    #[test]
    fn resolves_cache_entity_path_for_renamed_crate() {
        assert_eq!(
            cache_entity_path_for("hydracache_db", FoundCrate::Name("cache-db".to_owned()))
                .to_string(),
            ":: cache_db :: CacheEntity"
        );
    }
}
