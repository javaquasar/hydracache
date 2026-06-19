use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;

pub(crate) fn cache_entity_trait_path() -> TokenStream2 {
    resolve_database_exported_type_path("CacheEntity")
}

pub(crate) fn query_cache_policy_path() -> TokenStream2 {
    resolve_database_exported_type_path("QueryCachePolicy")
}

pub(crate) fn prepared_query_policy_path() -> TokenStream2 {
    resolve_database_exported_type_path("PreparedQueryPolicy")
}

pub(crate) fn refresh_policy_path() -> TokenStream2 {
    resolve_database_exported_type_path("RefreshPolicy")
}

pub(crate) fn declared_lint_mode_path() -> TokenStream2 {
    resolve_database_exported_type_path("DeclaredLintMode")
}

pub(crate) fn cache_key_builder_path() -> TokenStream2 {
    resolve_database_exported_type_path("CacheKeyBuilder")
}

pub(crate) fn cache_options_path() -> TokenStream2 {
    resolve_runtime_type_path(crate_name("hydracache").ok(), "CacheOptions")
}

pub(crate) fn cache_result_path() -> TokenStream2 {
    resolve_runtime_type_path(crate_name("hydracache").ok(), "CacheResult")
}

pub(crate) fn runtime_cache_key_builder_path() -> TokenStream2 {
    resolve_runtime_type_path(crate_name("hydracache").ok(), "CacheKeyBuilder")
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

fn resolve_database_exported_type_path(exported_type: &str) -> TokenStream2 {
    resolve_exported_type_path(
        &[
            ("hydracache_db", crate_name("hydracache-db").ok()),
            ("hydracache_sqlx", crate_name("hydracache-sqlx").ok()),
            ("hydracache_diesel", crate_name("hydracache-diesel").ok()),
            ("hydracache_seaorm", crate_name("hydracache-seaorm").ok()),
        ],
        "hydracache_db",
        exported_type,
    )
}

fn resolve_exported_type_path(
    candidates: &[(&str, Option<FoundCrate>)],
    fallback_default_name: &str,
    exported_type: &str,
) -> TokenStream2 {
    for (default_name, found) in candidates {
        if let Some(found) = found {
            return exported_type_path_for(default_name, found.clone(), exported_type);
        }
    }

    let fallback = syn::Ident::new(fallback_default_name, Span::call_site());
    let exported_type = syn::Ident::new(exported_type, Span::call_site());
    quote!(::#fallback::#exported_type)
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
        assert_eq!(
            prepared_query_policy_path().to_string(),
            ":: hydracache_db :: PreparedQueryPolicy"
        );
        assert_eq!(
            refresh_policy_path().to_string(),
            ":: hydracache_db :: RefreshPolicy"
        );
        assert_eq!(
            declared_lint_mode_path().to_string(),
            ":: hydracache_db :: DeclaredLintMode"
        );
        assert_eq!(
            cache_key_builder_path().to_string(),
            ":: hydracache_db :: CacheKeyBuilder"
        );
    }

    #[test]
    fn fallback_runtime_path_is_available_without_runtime_dependency() {
        assert_eq!(
            cache_options_path().to_string(),
            ":: hydracache :: CacheOptions"
        );
        assert_eq!(
            cache_result_path().to_string(),
            ":: hydracache :: CacheResult"
        );
        assert_eq!(
            runtime_cache_key_builder_path().to_string(),
            ":: hydracache :: CacheKeyBuilder"
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
                &[
                    (
                        "hydracache_db",
                        Some(FoundCrate::Name("cache-db".to_owned()))
                    ),
                    (
                        "hydracache_sqlx",
                        Some(FoundCrate::Name("cache-sqlx".to_owned()))
                    ),
                ],
                "hydracache_db",
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
                &[
                    ("hydracache_db", None),
                    (
                        "hydracache_sqlx",
                        Some(FoundCrate::Name("cache-sqlx".to_owned()))
                    ),
                ],
                "hydracache_db",
                "QueryCachePolicy",
            )
            .to_string(),
            ":: cache_sqlx :: QueryCachePolicy"
        );
    }

    #[test]
    fn resolver_can_use_diesel_adapter_reexports() {
        assert_eq!(
            resolve_exported_type_path(
                &[
                    ("hydracache_db", None),
                    ("hydracache_sqlx", None),
                    (
                        "hydracache_diesel",
                        Some(FoundCrate::Name("cache-diesel".to_owned()))
                    ),
                ],
                "hydracache_db",
                "RefreshPolicy",
            )
            .to_string(),
            ":: cache_diesel :: RefreshPolicy"
        );
    }
}
