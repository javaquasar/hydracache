use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod cacheable;
mod config;
mod entity;
mod paths;
mod policy;

/// Derive `CacheEntity` metadata for database result-cache helpers.
///
/// # Example
///
/// ```rust,ignore
/// use hydracache_db::{CacheEntity, HydraCacheEntity};
///
/// #[derive(HydraCacheEntity)]
/// #[hydracache(entity = "user", collection = "users", id = i64)]
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// assert_eq!(User::cache_key_for(&42), "user:42");
/// ```
#[proc_macro_derive(HydraCacheEntity, attributes(hydracache))]
pub fn derive_hydracache_entity(input: TokenStream) -> TokenStream {
    entity::expand(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Build a `QueryCachePolicy` with less boilerplate.
///
/// # Example
///
/// ```rust,ignore
/// use hydracache_db::{query_cache_policy, QueryCachePolicy};
///
/// let user_id = 42_i64;
/// let policy = query_cache_policy!(
///     name = "load-user",
///     entity = User,
///     id = user_id,
///     ttl_secs = 60,
/// );
/// ```
#[proc_macro]
pub fn query_cache_policy(input: TokenStream) -> TokenStream {
    policy::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Cache an ordinary async loader with explicit local-cache metadata.
///
/// # Example
///
/// ```rust,ignore
/// use hydracache::{cacheable, HydraCache};
///
/// let cache = HydraCache::local().build();
///
/// let value = cacheable!(
///     cache = cache,
///     key = "expensive:42",
///     tag = "expensive",
///     ttl_secs = 60,
///     load = || async { Ok::<_, std::io::Error>(42_u64) },
/// )
/// .await?;
/// ```
#[proc_macro]
pub fn cacheable(input: TokenStream) -> TokenStream {
    cacheable::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
