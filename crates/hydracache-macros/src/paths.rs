use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;

pub(crate) fn cache_entity_trait_path() -> TokenStream2 {
    resolve_exported_type_path(
        crate_name("hydracache-db").ok(),
        crate_name("hydracache-sqlx").ok(),
        "CacheEntity",
    )
}

pub(crate) fn query_cache_policy_path() -> TokenStream2 {
    resolve_exported_type_path(
        crate_name("hydracache-db").ok(),
        crate_name("hydracache-sqlx").ok(),
        "QueryCachePolicy",
    )
}

pub(crate) fn cache_options_path() -> TokenStream2 {
    resolve_runtime_type_path(crate_name("hydracache").ok(), "CacheOptions")
}

fn resolve_runtime_type_path(
    runtime_crate: Option<FoundCrate>,
    exported_type: &str,
) -> TokenStream2 {
    if let Some(found) = runtime_crate {
        exported_type_path_for("hydracache", found, exported_type)
    } else {
        let exported_type = syn::Ident::new(exported_type, Span::call_site());
        quote!(::hydracache::#exported_type)
    }
}

fn resolve_exported_type_path(
    db_crate: Option<FoundCrate>,
    sqlx_crate: Option<FoundCrate>,
    exported_type: &str,
) -> TokenStream2 {
    if let Some(found) = db_crate {
        exported_type_path_for("hydracache_db", found, exported_type)
    } else if let Some(found) = sqlx_crate {
        exported_type_path_for("hydracache_sqlx", found, exported_type)
    } else {
        let exported_type = syn::Ident::new(exported_type, Span::call_site());
        quote!(::hydracache_db::#exported_type)
    }
}

fn exported_type_path_for(
    default_name: &str,
    found: FoundCrate,
    exported_type: &str,
) -> TokenStream2 {
    let crate_name = match found {
        FoundCrate::Itself => default_name.to_owned(),
        FoundCrate::Name(name) => name,
    };
    let ident = syn::Ident::new(&crate_name.replace('-', "_"), Span::call_site());
    let exported_type = syn::Ident::new(exported_type, Span::call_site());

    quote!(::#ident::#exported_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_cache_entity_path_for_current_crate() {
        assert_eq!(
            exported_type_path_for("hydracache_db", FoundCrate::Itself, "CacheEntity").to_string(),
            ":: hydracache_db :: CacheEntity"
        );
    }

    #[test]
    fn resolves_cache_entity_path_for_renamed_crate() {
        assert_eq!(
            exported_type_path_for(
                "hydracache_db",
                FoundCrate::Name("cache-db".to_owned()),
                "CacheEntity",
            )
            .to_string(),
            ":: cache_db :: CacheEntity"
        );
    }

    #[test]
    fn resolves_query_cache_policy_path_for_renamed_crate() {
        assert_eq!(
            exported_type_path_for(
                "hydracache_db",
                FoundCrate::Name("cache-db".to_owned()),
                "QueryCachePolicy",
            )
            .to_string(),
            ":: cache_db :: QueryCachePolicy"
        );
    }

    #[test]
    fn fallback_paths_are_available_without_adapter_dependencies() {
        assert_eq!(
            cache_entity_trait_path().to_string(),
            ":: hydracache_db :: CacheEntity"
        );
        assert_eq!(
            query_cache_policy_path().to_string(),
            ":: hydracache_db :: QueryCachePolicy"
        );
    }

    #[test]
    fn fallback_runtime_path_is_available_without_runtime_dependency() {
        assert_eq!(
            cache_options_path().to_string(),
            ":: hydracache :: CacheOptions"
        );
    }

    #[test]
    fn resolves_runtime_path_for_current_crate() {
        assert_eq!(
            resolve_runtime_type_path(Some(FoundCrate::Itself), "CacheOptions").to_string(),
            ":: hydracache :: CacheOptions"
        );
    }

    #[test]
    fn resolves_runtime_path_for_renamed_crate() {
        assert_eq!(
            resolve_runtime_type_path(
                Some(FoundCrate::Name("local-cache".to_owned())),
                "CacheOptions",
            )
            .to_string(),
            ":: local_cache :: CacheOptions"
        );
    }

    #[test]
    fn resolver_prefers_database_neutral_crate() {
        assert_eq!(
            resolve_exported_type_path(
                Some(FoundCrate::Name("cache-db".to_owned())),
                Some(FoundCrate::Name("cache-sqlx".to_owned())),
                "QueryCachePolicy",
            )
            .to_string(),
            ":: cache_db :: QueryCachePolicy"
        );
    }

    #[test]
    fn resolver_falls_back_to_sqlx_adapter_crate() {
        assert_eq!(
            resolve_exported_type_path(
                None,
                Some(FoundCrate::Name("cache-sqlx".to_owned())),
                "QueryCachePolicy",
            )
            .to_string(),
            ":: cache_sqlx :: QueryCachePolicy"
        );
    }
}
