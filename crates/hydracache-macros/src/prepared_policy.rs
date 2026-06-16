use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Token, Type};

use crate::paths::{cache_key_builder_path, prepared_query_policy_path, refresh_policy_path};

pub(crate) fn expand(input: TokenStream2) -> syn::Result<TokenStream2> {
    let config: PreparedPolicyConfig = syn::parse2(input)?;
    config.validate()?;
    Ok(config.expand())
}

#[derive(Default)]
struct PreparedPolicyConfig {
    name: Option<Expr>,
    per_entity: Option<Type>,
    entity: Option<Expr>,
    collection: Option<Expr>,
    key: Option<Expr>,
    key_segments: Option<SegmentList>,
    ttl: Option<Expr>,
    ttl_secs: Option<Expr>,
    refresh_ahead_secs: Option<Expr>,
    stale_while_revalidate_secs: Option<Expr>,
    stale_on_loader_error_secs: Option<Expr>,
    tags: Vec<Expr>,
    tag_segments: Vec<SegmentList>,
    collection_tags: Vec<Expr>,
}

impl Parse for PreparedPolicyConfig {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut config = Self::default();

        while !input.is_empty() {
            let option: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match option.to_string().as_str() {
                "name" => parse_unique_expr(input, &mut config.name, &option)?,
                "per_entity" => parse_unique_type(input, &mut config.per_entity, &option)?,
                "entity" => parse_unique_expr(input, &mut config.entity, &option)?,
                "collection" => parse_unique_expr(input, &mut config.collection, &option)?,
                "key" => parse_unique_expr(input, &mut config.key, &option)?,
                "key_segments" => {
                    parse_unique_segment_list(input, &mut config.key_segments, &option)?
                }
                "ttl" => parse_unique_expr(input, &mut config.ttl, &option)?,
                "ttl_secs" => parse_unique_expr(input, &mut config.ttl_secs, &option)?,
                "refresh_ahead_secs" => {
                    parse_unique_expr(input, &mut config.refresh_ahead_secs, &option)?
                }
                "stale_while_revalidate_secs" => {
                    parse_unique_expr(input, &mut config.stale_while_revalidate_secs, &option)?
                }
                "stale_on_loader_error_secs" => {
                    parse_unique_expr(input, &mut config.stale_on_loader_error_secs, &option)?
                }
                "tag" => config.tags.push(input.parse()?),
                "tag_segments" => {
                    config
                        .tag_segments
                        .extend(input.parse::<SegmentGroups>()?.groups);
                }
                "collection_tag" => config.collection_tags.push(input.parse()?),
                _ => {
                    return Err(syn::Error::new(
                        option.span(),
                        "unsupported prepared_query_policy option",
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

impl PreparedPolicyConfig {
    fn validate(&self) -> syn::Result<()> {
        let key_sources = [
            self.per_entity.is_some(),
            self.entity.is_some(),
            self.collection.is_some(),
            self.key.is_some(),
            self.key_segments.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count();

        if key_sources == 0 {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "prepared_query_policy requires one key source: per_entity, entity, collection, key, or key_segments",
            ));
        }

        if key_sources > 1 {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "prepared_query_policy accepts only one key source: per_entity, entity, collection, key, or key_segments",
            ));
        }

        if self.ttl.is_some() && self.ttl_secs.is_some() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "prepared_query_policy accepts only one TTL option: ttl or ttl_secs",
            ));
        }

        Ok(())
    }

    fn expand(&self) -> TokenStream2 {
        let policy_path = prepared_query_policy_path();
        let refresh_path = refresh_policy_path();
        let key_builder_path = cache_key_builder_path();
        let base = if let Some(entity) = &self.per_entity {
            quote!(#policy_path::per_entity().cache_entity::<#entity>())
        } else if let Some(entity) = &self.entity {
            quote!(#policy_path::for_entity(#entity))
        } else if let Some(collection) = &self.collection {
            quote!(#policy_path::new().collection(#collection))
        } else if let Some(key) = &self.key {
            quote!(#policy_path::new().key(#key))
        } else {
            let key_segments = self
                .key_segments
                .as_ref()
                .expect("validated key segments should exist");
            let key_builder = segment_builder_tokens(&key_builder_path, &key_segments.segments);
            quote!(#policy_path::new().key_builder(#key_builder))
        };

        let name = self.name.as_ref().map(|name| quote!(.with_name(#name)));
        let tags = self.tags.iter().map(|tag| quote!(.tag(#tag)));
        let segment_tags = self.tag_segments.iter().map(|tag_segments| {
            let tag_builder = segment_builder_tokens(&key_builder_path, &tag_segments.segments);
            quote!(.tag(#tag_builder.build_string()))
        });
        let collection_tags = self
            .collection_tags
            .iter()
            .map(|tag| quote!(.collection_tag(#tag)));
        let ttl = self.ttl.as_ref().map(|ttl| quote!(.ttl(#ttl)));
        let ttl_secs = self
            .ttl_secs
            .as_ref()
            .map(|ttl_secs| quote!(.ttl(::std::time::Duration::from_secs(#ttl_secs))));
        let refresh_policy = self.refresh_policy_tokens(&refresh_path);

        quote! {
            #base
                #name
                #(#tags)*
                #(#segment_tags)*
                #(#collection_tags)*
                #ttl
                #ttl_secs
                #refresh_policy
        }
    }

    fn refresh_policy_tokens(&self, refresh_path: &TokenStream2) -> Option<TokenStream2> {
        if self.refresh_ahead_secs.is_none()
            && self.stale_while_revalidate_secs.is_none()
            && self.stale_on_loader_error_secs.is_none()
        {
            return None;
        }

        let refresh_ahead = self
            .refresh_ahead_secs
            .as_ref()
            .map(|seconds| quote!(.refresh_ahead(::std::time::Duration::from_secs(#seconds))));
        let stale_while_revalidate = self.stale_while_revalidate_secs.as_ref().map(
            |seconds| quote!(.stale_while_revalidate(::std::time::Duration::from_secs(#seconds))),
        );
        let stale_on_loader_error = self.stale_on_loader_error_secs.as_ref().map(
            |seconds| quote!(.stale_on_loader_error(::std::time::Duration::from_secs(#seconds))),
        );

        Some(quote! {
            .refresh_policy(
                #refresh_path::new()
                    #refresh_ahead
                    #stale_while_revalidate
                    #stale_on_loader_error
            )
        })
    }
}

struct SegmentList {
    segments: Vec<Expr>,
}

impl Parse for SegmentList {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);
        let segments = parse_segment_exprs(
            &content,
            "prepared_query_policy segment list cannot be empty",
        )?;

        Ok(Self { segments })
    }
}

struct SegmentGroups {
    groups: Vec<SegmentList>,
}

impl Parse for SegmentGroups {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);

        if content.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "prepared_query_policy tag_segments requires at least one segment group",
            ));
        }

        let mut groups = Vec::new();
        while !content.is_empty() {
            if !content.peek(syn::token::Bracket) {
                return Err(content.error(
                    "prepared_query_policy tag_segments expects nested segment groups like [[...], [...]]",
                ));
            }

            groups.push(content.parse()?);

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else if !content.is_empty() {
                return Err(content.error(
                    "prepared_query_policy tag_segments expects comma-separated segment groups",
                ));
            }
        }

        Ok(Self { groups })
    }
}

fn parse_segment_exprs(input: ParseStream<'_>, empty_message: &str) -> syn::Result<Vec<Expr>> {
    let mut segments = Vec::new();

    while !input.is_empty() {
        segments.push(input.parse()?);

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        } else if !input.is_empty() {
            return Err(input
                .error("prepared_query_policy segment list expects comma-separated expressions"));
        }
    }

    if segments.is_empty() {
        Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            empty_message,
        ))
    } else {
        Ok(segments)
    }
}

fn segment_builder_tokens(builder_path: &TokenStream2, segments: &[Expr]) -> TokenStream2 {
    let segments = segments.iter().map(|segment| quote!(.segment(#segment)));

    quote!(#builder_path::new()#(#segments)*)
}

fn parse_unique_segment_list(
    input: ParseStream<'_>,
    current: &mut Option<SegmentList>,
    option: &Ident,
) -> syn::Result<()> {
    reject_duplicate(current, option)?;
    *current = Some(input.parse()?);
    Ok(())
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
            format!("duplicate prepared_query_policy {} option", option),
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
    fn expands_cache_entity_policy_with_name_ttl_and_refresh() {
        let output = expand_to_string(quote! {
            per_entity = User,
            name = "load-user",
            collection_tag = "tenant:7",
            ttl_secs = 300,
            refresh_ahead_secs = 10,
        });

        assert!(output.contains("PreparedQueryPolicy :: per_entity"));
        assert!(output.contains(". cache_entity :: < User >"));
        assert!(output.contains(". with_name (\"load-user\")"));
        assert!(output.contains(". collection_tag (\"tenant:7\")"));
        assert!(output.contains("Duration :: from_secs (300)"));
        assert!(output.contains("RefreshPolicy :: new"));
    }

    #[test]
    fn expands_collection_policy() {
        let output = expand_to_string(quote! {
            collection = "users",
            name = "list-users",
        });

        assert!(output.contains("PreparedQueryPolicy :: new"));
        assert!(output.contains(". collection (\"users\")"));
        assert!(output.contains(". with_name (\"list-users\")"));
    }

    #[test]
    fn expands_manual_entity_policy_without_per_entity_preset() {
        let output = expand_to_string(quote! {
            entity = "user",
            name = "load-user",
        });

        assert!(output.contains("PreparedQueryPolicy :: for_entity"));
        assert!(!output.contains("PreparedQueryPolicy :: per_entity"));
    }

    #[test]
    fn expands_segmented_static_key_and_tags() {
        let output = expand_to_string(quote! {
            key_segments = ["tenant", tenant_id, "q", query],
            tag_segments = [["tenant", tenant_id], ["users"]],
            ttl_secs = 30,
        });

        assert!(output.contains(". key_builder"));
        assert!(output.contains("CacheKeyBuilder :: new"));
        assert!(output.contains(". segment (\"tenant\")"));
        assert!(output.contains(". tag"));
        assert!(output.contains(". build_string"));
    }

    #[test]
    fn rejects_missing_key_source() {
        let error = expand(quote!(name = "load-user")).unwrap_err();

        assert!(error.to_string().contains("requires one key source"));
    }

    #[test]
    fn rejects_conflicting_key_sources() {
        let error = expand(quote!(per_entity = User, key = "user:1")).unwrap_err();

        assert!(error.to_string().contains("accepts only one key source"));
    }

    #[test]
    fn rejects_duplicate_options() {
        let error = expand(quote!(key = "one", key = "two")).unwrap_err();

        assert!(error
            .to_string()
            .contains("duplicate prepared_query_policy key"));
    }

    #[test]
    fn rejects_unknown_options() {
        let error = expand(quote!(key = "one", table = "users")).unwrap_err();

        assert!(error
            .to_string()
            .contains("unsupported prepared_query_policy option"));
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

    #[test]
    fn rejects_flat_tag_segments() {
        let error = expand(quote! {
            key = "users",
            tag_segments = ["tenant", tenant_id],
        })
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("tag_segments expects nested segment groups"));
    }
}
