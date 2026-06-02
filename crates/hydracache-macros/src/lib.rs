use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod config;
mod entity;
mod paths;

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
