use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Token, Type};

use crate::paths::query_cache_policy_path;

pub(crate) fn expand(input: TokenStream2) -> syn::Result<TokenStream2> {
    let config: PolicyConfig = syn::parse2(input)?;
    config.validate()?;
    Ok(config.expand())
}

#[derive(Default)]
struct PolicyConfig {
    name: Option<Expr>,
    key: Option<Expr>,
    collection: Option<Expr>,
    entity: Option<Type>,
    id: Option<Expr>,
    ttl: Option<Expr>,
    ttl_secs: Option<Expr>,
    tags: Vec<Expr>,
    collection_tags: Vec<Expr>,
}

impl Parse for PolicyConfig {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut config = Self::default();

        while !input.is_empty() {
            let option: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match option.to_string().as_str() {
                "name" => parse_unique_expr(input, &mut config.name, &option)?,
                "key" => parse_unique_expr(input, &mut config.key, &option)?,
                "collection" => parse_unique_expr(input, &mut config.collection, &option)?,
                "entity" => parse_unique_type(input, &mut config.entity, &option)?,
                "id" => parse_unique_expr(input, &mut config.id, &option)?,
                "ttl" => parse_unique_expr(input, &mut config.ttl, &option)?,
                "ttl_secs" => parse_unique_expr(input, &mut config.ttl_secs, &option)?,
                "tag" => config.tags.push(input.parse()?),
                "collection_tag" => config.collection_tags.push(input.parse()?),
                _ => {
                    return Err(syn::Error::new(
                        option.span(),
                        "unsupported query_cache_policy option",
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(config)
    }
}

impl PolicyConfig {
    fn validate(&self) -> syn::Result<()> {
        if self.entity.is_some() && self.id.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "query_cache_policy entity requires id",
            ));
        }

        if self.entity.is_none() && self.id.is_some() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "query_cache_policy id requires entity",
            ));
        }

        let key_sources = [
            self.key.is_some(),
            self.collection.is_some(),
            self.entity.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count();

        if key_sources == 0 {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "query_cache_policy requires one key source: key, collection, or entity + id",
            ));
        }

        if key_sources > 1 {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "query_cache_policy accepts only one key source: key, collection, or entity + id",
            ));
        }

        if self.ttl.is_some() && self.ttl_secs.is_some() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "query_cache_policy accepts only one TTL option: ttl or ttl_secs",
            ));
        }

        Ok(())
    }

    fn expand(&self) -> TokenStream2 {
        let policy_path = query_cache_policy_path();
        let base = match &self.name {
            Some(name) => quote!(#policy_path::named(#name)),
            None => quote!(#policy_path::new()),
        };

        let key_source = if let Some(key) = &self.key {
            quote!(.key(#key))
        } else if let Some(collection) = &self.collection {
            quote!(.collection(#collection))
        } else {
            let entity = self.entity.as_ref().expect("validated entity should exist");
            let id = self.id.as_ref().expect("validated id should exist");
            quote!(.for_cache_entity::<#entity>(#id))
        };

        let tags = self.tags.iter().map(|tag| quote!(.tag(#tag)));
        let collection_tags = self
            .collection_tags
            .iter()
            .map(|tag| quote!(.collection_tag(#tag)));
        let ttl = self.ttl.as_ref().map(|ttl| quote!(.ttl(#ttl)));
        let ttl_secs = self
            .ttl_secs
            .as_ref()
            .map(|ttl_secs| quote!(.ttl(::std::time::Duration::from_secs(#ttl_secs))));

        quote! {
            #base
                #key_source
                #(#tags)*
                #(#collection_tags)*
                #ttl
                #ttl_secs
        }
    }
}

fn parse_unique_expr(
    input: ParseStream<'_>,
    current: &mut Option<Expr>,
    option: &Ident,
) -> syn::Result<()> {
    reject_duplicate(current, option)?;
    *current = Some(input.parse()?);
    Ok(())
}

fn parse_unique_type(
    input: ParseStream<'_>,
    current: &mut Option<Type>,
    option: &Ident,
) -> syn::Result<()> {
    reject_duplicate(current, option)?;
    *current = Some(input.parse()?);
    Ok(())
}

fn reject_duplicate<T>(current: &Option<T>, option: &Ident) -> syn::Result<()> {
    if current.is_some() {
        Err(syn::Error::new(
            option.span(),
            format!("duplicate query_cache_policy {} option", option),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand_to_string(input: TokenStream2) -> String {
        expand(input).unwrap().to_string()
    }

    #[test]
    fn expands_entity_policy_with_name_ttl_and_tags() {
        let output = expand_to_string(quote! {
            name = "load-user",
            entity = User,
            id = user_id,
            tag = "tenant:7",
            collection_tag = "users:active",
            ttl_secs = 60,
        });

        assert!(output.contains("QueryCachePolicy :: named"));
        assert!(output.contains(". for_cache_entity :: < User > (user_id)"));
        assert!(output.contains(". tag (\"tenant:7\")"));
        assert!(output.contains(". collection_tag (\"users:active\")"));
        assert!(output.contains("Duration :: from_secs (60)"));
    }

    #[test]
    fn expands_manual_key_policy_with_ttl_expr() {
        let output = expand_to_string(quote! {
            key = "users",
            ttl = ttl,
        });

        assert!(output.contains("QueryCachePolicy :: new"));
        assert!(output.contains(". key (\"users\")"));
        assert!(output.contains(". ttl (ttl)"));
    }

    #[test]
    fn expands_collection_policy() {
        let output = expand_to_string(quote! {
            collection = "users",
        });

        assert!(output.contains(". collection (\"users\")"));
    }

    #[test]
    fn rejects_missing_key_source() {
        let error = expand(quote!(name = "load-user")).unwrap_err();

        assert!(error.to_string().contains("requires one key source"));
    }

    #[test]
    fn rejects_conflicting_key_sources() {
        let error = expand(quote!(key = "user:1", collection = "users")).unwrap_err();

        assert!(error.to_string().contains("accepts only one key source"));
    }

    #[test]
    fn rejects_entity_without_id() {
        let error = expand(quote!(entity = User)).unwrap_err();

        assert!(error.to_string().contains("entity requires id"));
    }

    #[test]
    fn rejects_id_without_entity() {
        let error = expand(quote!(id = user_id)).unwrap_err();

        assert!(error.to_string().contains("id requires entity"));
    }

    #[test]
    fn rejects_duplicate_options() {
        let error = expand(quote!(key = "one", key = "two")).unwrap_err();

        assert!(error
            .to_string()
            .contains("duplicate query_cache_policy key"));
    }

    #[test]
    fn rejects_unknown_options() {
        let error = expand(quote!(key = "one", table = "users")).unwrap_err();

        assert!(error
            .to_string()
            .contains("unsupported query_cache_policy option"));
    }

    #[test]
    fn rejects_conflicting_ttl_options() {
        let error = expand(quote! {
            key = "one",
            ttl = ttl,
            ttl_secs = 60,
        })
        .unwrap_err();

        assert!(error.to_string().contains("accepts only one TTL option"));
    }
}
