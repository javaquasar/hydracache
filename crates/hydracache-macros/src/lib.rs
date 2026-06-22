use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod cacheable;
mod config;
mod entity;
mod paths;
mod policy;
mod prepared_policy;

/// Derive `CacheEntity` metadata for database result-cache helpers.
///
/// # Example
///
/// ```text
/// use hydracache_db::{CacheEntity, HydraCacheEntity};
///
/// #[derive(HydraCacheEntity)]
/// #[hydracache(entity = "user", collection = "users")]
/// struct User {
///     #[hydracache(id)]
///     id: i64,
///     name: String,
/// }
///
/// assert_eq!(User::cache_key_for(&42), "user:42");
/// ```
///
/// `#[hydracache(id = Type)]` on the struct remains supported when the id type
/// cannot be inferred from one named field.
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
/// ```text
/// use hydracache_db::query_cache_policy;
///
/// let user_id = 42_i64;
/// let policy = query_cache_policy!(
///     preset = read_mostly,
///     name = "load-user",
///     entity = User,
///     id = user_id,
///     refresh_ahead_secs = 10,
///     stale_while_revalidate_secs = 300,
/// );
///
/// let search = query_cache_policy!(
///     name = "search-users",
///     key_segments = ["tenant", tenant_id, "q", query, "page", page],
///     tag_segments = [["tenant", tenant_id], ["users"]],
///     ttl_secs = 30,
/// );
/// ```
#[proc_macro]
pub fn query_cache_policy(input: TokenStream) -> TokenStream {
    policy::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Build a reusable `PreparedQueryPolicy` with less boilerplate.
///
/// # Example
///
/// ```text
/// use hydracache_db::prepared_query_policy;
///
/// let load_user = prepared_query_policy!(
///     per_entity = User,
///     name = "load-user",
///     ttl_secs = 300,
/// );
/// ```
#[proc_macro]
pub fn prepared_query_policy(input: TokenStream) -> TokenStream {
    prepared_policy::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Cache an ordinary async function with explicit local-cache metadata.
///
/// The decorated function must be async, return `Result<T, E>`, and receive the
/// cache as an explicit argument referenced by `cache = ...`. The generated
/// wrapper returns `hydracache::CacheResult<T>` because cache errors can also be
/// produced outside the user loader.
///
/// # Example
///
/// ```text
/// use hydracache::{cacheable, HydraCache};
///
/// #[cacheable(
///     cache = cache,
///     key_segments = ["profile", profile_id],
///     tag_segments = [["profile", profile_id], ["profiles"]],
///     ttl_secs = 60
/// )]
/// async fn load_profile(
///     cache: &HydraCache,
///     profile_id: u64,
/// ) -> Result<Profile, LoadError> {
///     repo_load_profile(profile_id).await
/// }
/// ```
#[proc_macro_attribute]
pub fn cacheable(args: TokenStream, item: TokenStream) -> TokenStream {
    cacheable::expand_attribute(args.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Cache an ordinary fallible async loader with explicit local-cache metadata.
///
/// The macro builds `CacheOptions` and calls `HydraCache::get_or_load`.
/// `cache`, `key`, and `load` are required. `tag = ...` can be repeated,
/// `tags = ...` accepts any iterable accepted by `CacheOptions::tags`, and
/// either `ttl = Duration` or `ttl_secs = u64` can be supplied.
///
/// # Example
///
/// ```text
/// use hydracache::{cacheable_loader, CacheKeyBuilder, HydraCache, TagSet};
///
/// let cache = HydraCache::local().build();
/// let user_id = 42_u64;
/// let key = CacheKeyBuilder::new().entity("user", user_id).build_string();
///
/// let value = cacheable_loader!(
///     cache = cache,
///     key = key.as_str(),
///     tags = TagSet::new().tag("users").entity("user", user_id),
///     ttl_secs = 60,
///     load = move || async move { Ok::<_, std::io::Error>(user_id) },
/// )
/// .await?;
/// ```
#[proc_macro]
pub fn cacheable_loader(input: TokenStream) -> TokenStream {
    cacheable::expand_loader(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Cache an ordinary async loader that cannot fail in application terms.
///
/// The macro builds `CacheOptions` and calls `HydraCache::get_or_insert_with`.
/// Use it when the loader returns `T` instead of `Result<T, E>`.
///
/// # Example
///
/// ```text
/// use hydracache::{cacheable_infallible, HydraCache};
///
/// let cache = HydraCache::local().build();
///
/// let value = cacheable_infallible!(
///     cache = cache,
///     key = "expensive:42",
///     tags = ["expensive"],
///     ttl_secs = 60,
///     load = || async { 42_u64 },
/// )
/// .await?;
/// ```
#[proc_macro]
pub fn cacheable_infallible(input: TokenStream) -> TokenStream {
    cacheable::expand_infallible(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
