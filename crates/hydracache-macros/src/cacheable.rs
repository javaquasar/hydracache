use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{
    parse_quote, Expr, GenericArgument, Ident, ItemFn, PathArguments, ReturnType, Token, Type,
};

use crate::paths::{cache_options_path, cache_result_path, runtime_cache_key_builder_path};

pub(crate) fn expand_loader(input: TokenStream2) -> syn::Result<TokenStream2> {
    let config: CacheableConfig = syn::parse2(input)?;
    config.validate("cacheable_loader")?;
    Ok(config.expand(CacheableMode::Fallible))
}

pub(crate) fn expand_infallible(input: TokenStream2) -> syn::Result<TokenStream2> {
    let config: CacheableConfig = syn::parse2(input)?;
    config.validate("cacheable_infallible")?;
    Ok(config.expand(CacheableMode::Infallible))
}

pub(crate) fn expand_attribute(
    args: TokenStream2,
    item: TokenStream2,
) -> syn::Result<TokenStream2> {
    let config: CacheableAttributeConfig = syn::parse2(args)?;
    config.validate()?;
    let mut function: ItemFn = syn::parse2(item)?;
    let value_type = cacheable_value_type(&function)?;
    let original_output_type = original_result_type(&function)?;
    let original_block = function.block;
    let cache_result_path = cache_result_path();
    let options_path = cache_options_path();
    let key_builder_path = runtime_cache_key_builder_path();
    let cache = config.cache.as_ref().expect("validated cache should exist");
    let key_binding = config.key_binding_tokens(&key_builder_path);
    let tags_expr = config.tags_expr.as_ref().map(|tags| quote!(.tags(#tags)));
    let repeated_tags = config.repeated_tags.iter().map(|tag| quote!(.tag(#tag)));
    let segment_tags = config.tag_segments.iter().map(|tag_segments| {
        let tag_builder = segment_builder_tokens(&key_builder_path, &tag_segments.segments);
        quote!(.tag(#tag_builder.build_string()))
    });
    let ttl = config.ttl.as_ref().map(|ttl| quote!(.ttl(#ttl)));
    let ttl_secs = config
        .ttl_secs
        .as_ref()
        .map(|ttl_secs| quote!(.ttl(::std::time::Duration::from_secs(#ttl_secs))));

    function.sig.output = parse_quote!(-> #cache_result_path<#value_type>);
    function.block = Box::new(parse_quote!({
        #key_binding
        let __hydracache_options = #options_path::new()
            #tags_expr
            #(#repeated_tags)*
            #(#segment_tags)*
            #ttl
            #ttl_secs;

        (#cache)
            .get_or_load(__hydracache_key, __hydracache_options, move || async move {
                let __hydracache_loader_result: #original_output_type = (async move #original_block).await;
                __hydracache_loader_result
            })
            .await
    }));

    Ok(quote!(#function))
}

#[derive(Debug, Clone, Copy)]
enum CacheableMode {
    Fallible,
    Infallible,
}

#[derive(Default)]
struct CacheableConfig {
    cache: Option<Expr>,
    key: Option<Expr>,
    load: Option<Expr>,
    ttl: Option<Expr>,
    ttl_secs: Option<Expr>,
    tags_expr: Option<Expr>,
    repeated_tags: Vec<Expr>,
}

impl Parse for CacheableConfig {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut config = Self::default();

        while !input.is_empty() {
            let option: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match option.to_string().as_str() {
                "cache" => parse_unique_expr(input, &mut config.cache, &option)?,
                "key" => parse_unique_expr(input, &mut config.key, &option)?,
                "load" => parse_unique_expr(input, &mut config.load, &option)?,
                "ttl" => parse_unique_expr(input, &mut config.ttl, &option)?,
                "ttl_secs" => parse_unique_expr(input, &mut config.ttl_secs, &option)?,
                "tags" => parse_unique_expr(input, &mut config.tags_expr, &option)?,
                "tag" => config.repeated_tags.push(input.parse()?),
                _ => {
                    return Err(syn::Error::new(
                        option.span(),
                        "unsupported cacheable option",
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

#[derive(Default)]
struct CacheableAttributeConfig {
    cache: Option<Expr>,
    key: Option<Expr>,
    key_segments: Option<SegmentList>,
    ttl: Option<Expr>,
    ttl_secs: Option<Expr>,
    tags_expr: Option<Expr>,
    repeated_tags: Vec<Expr>,
    tag_segments: Vec<SegmentList>,
}

impl Parse for CacheableAttributeConfig {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut config = Self::default();

        while !input.is_empty() {
            let option: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match option.to_string().as_str() {
                "cache" => parse_unique_expr(input, &mut config.cache, &option)?,
                "key" => parse_unique_expr(input, &mut config.key, &option)?,
                "key_segments" => {
                    parse_unique_segment_list(input, &mut config.key_segments, &option)?
                }
                "ttl" => parse_unique_expr(input, &mut config.ttl, &option)?,
                "ttl_secs" => parse_unique_expr(input, &mut config.ttl_secs, &option)?,
                "tags" => parse_unique_expr(input, &mut config.tags_expr, &option)?,
                "tag" => config.repeated_tags.push(input.parse()?),
                "tag_segments" => {
                    config
                        .tag_segments
                        .extend(input.parse::<SegmentGroups>()?.groups);
                }
                _ => {
                    return Err(syn::Error::new(
                        option.span(),
                        "unsupported cacheable attribute option",
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

impl CacheableAttributeConfig {
    fn validate(&self) -> syn::Result<()> {
        if self.cache.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "cacheable attribute requires cache",
            ));
        }

        let key_sources = [self.key.is_some(), self.key_segments.is_some()]
            .into_iter()
            .filter(|present| *present)
            .count();

        if key_sources == 0 {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "cacheable attribute requires one key source: key or key_segments",
            ));
        }

        if key_sources > 1 {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "cacheable attribute accepts only one key source: key or key_segments",
            ));
        }

        if self.ttl.is_some() && self.ttl_secs.is_some() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "cacheable attribute accepts only one TTL option: ttl or ttl_secs",
            ));
        }

        Ok(())
    }

    fn key_binding_tokens(&self, key_builder_path: &TokenStream2) -> TokenStream2 {
        if let Some(key) = &self.key {
            quote! {
                let __hydracache_key_value = #key;
                let __hydracache_key = ::std::convert::AsRef::<str>::as_ref(&__hydracache_key_value);
            }
        } else {
            let key_segments = self
                .key_segments
                .as_ref()
                .expect("validated key segments should exist");
            let key_builder = segment_builder_tokens(key_builder_path, &key_segments.segments);
            quote! {
                let __hydracache_key_value = #key_builder.build_string();
                let __hydracache_key = __hydracache_key_value.as_str();
            }
        }
    }
}

impl CacheableConfig {
    fn validate(&self, macro_name: &str) -> syn::Result<()> {
        if self.cache.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("{macro_name} requires cache"),
            ));
        }

        if self.key.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("{macro_name} requires key"),
            ));
        }

        if self.load.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("{macro_name} requires load"),
            ));
        }

        if self.ttl.is_some() && self.ttl_secs.is_some() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("{macro_name} accepts only one TTL option: ttl or ttl_secs"),
            ));
        }

        Ok(())
    }

    fn expand(&self, mode: CacheableMode) -> TokenStream2 {
        let options_path = cache_options_path();
        let cache = self.cache.as_ref().expect("validated cache should exist");
        let key = self.key.as_ref().expect("validated key should exist");
        let load = self.load.as_ref().expect("validated load should exist");
        let tags_expr = self.tags_expr.as_ref().map(|tags| quote!(.tags(#tags)));
        let repeated_tags = self.repeated_tags.iter().map(|tag| quote!(.tag(#tag)));
        let ttl = self.ttl.as_ref().map(|ttl| quote!(.ttl(#ttl)));
        let ttl_secs = self
            .ttl_secs
            .as_ref()
            .map(|ttl_secs| quote!(.ttl(::std::time::Duration::from_secs(#ttl_secs))));
        let load_call = match mode {
            CacheableMode::Fallible => quote!(get_or_load),
            CacheableMode::Infallible => quote!(get_or_insert_with),
        };

        quote! {{
            let __hydracache_options = #options_path::new()
                #tags_expr
                #(#repeated_tags)*
                #ttl
                #ttl_secs;
            (#cache).#load_call(#key, __hydracache_options, #load)
        }}
    }
}

struct SegmentList {
    segments: Vec<Expr>,
}

impl Parse for SegmentList {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);
        let segments = parse_segment_exprs(&content)?;

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
                "cacheable attribute tag_segments requires at least one segment group",
            ));
        }

        let mut groups = Vec::new();
        while !content.is_empty() {
            if !content.peek(syn::token::Bracket) {
                return Err(content.error(
                    "cacheable attribute tag_segments expects nested segment groups like [[...], [...]]",
                ));
            }

            groups.push(content.parse()?);

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else if !content.is_empty() {
                return Err(content.error(
                    "cacheable attribute tag_segments expects comma-separated segment groups",
                ));
            }
        }

        Ok(Self { groups })
    }
}

fn parse_segment_exprs(input: ParseStream<'_>) -> syn::Result<Vec<Expr>> {
    let mut segments = Vec::new();

    while !input.is_empty() {
        segments.push(input.parse()?);

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        } else if !input.is_empty() {
            return Err(
                input.error("cacheable attribute segment list expects comma-separated expressions")
            );
        }
    }

    if segments.is_empty() {
        Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "cacheable attribute segment list cannot be empty",
        ))
    } else {
        Ok(segments)
    }
}

fn segment_builder_tokens(builder_path: &TokenStream2, segments: &[Expr]) -> TokenStream2 {
    let segments = segments.iter().map(|segment| quote!(.segment(&(#segment))));

    quote!(#builder_path::new()#(#segments)*)
}

fn cacheable_value_type(function: &ItemFn) -> syn::Result<Type> {
    if function.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            function.sig.fn_token,
            "cacheable attribute requires async fn",
        ));
    }

    let ReturnType::Type(_, output) = &function.sig.output else {
        return Err(syn::Error::new_spanned(
            &function.sig.ident,
            "cacheable attribute requires Result<T, E> return type",
        ));
    };

    result_ok_type(output)
}

fn original_result_type(function: &ItemFn) -> syn::Result<Type> {
    let ReturnType::Type(_, output) = &function.sig.output else {
        return Err(syn::Error::new_spanned(
            &function.sig.ident,
            "cacheable attribute requires Result<T, E> return type",
        ));
    };

    Ok((**output).clone())
}

fn result_ok_type(output: &Type) -> syn::Result<Type> {
    let Type::Path(path) = output else {
        return Err(syn::Error::new_spanned(
            output,
            "cacheable attribute requires Result<T, E> return type",
        ));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            output,
            "cacheable attribute requires Result<T, E> return type",
        ));
    };
    if segment.ident != "Result" {
        return Err(syn::Error::new_spanned(
            output,
            "cacheable attribute requires Result<T, E> return type",
        ));
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(syn::Error::new_spanned(
            output,
            "cacheable attribute requires Result<T, E> return type",
        ));
    };
    let Some(GenericArgument::Type(value_type)) = arguments.args.first() else {
        return Err(syn::Error::new_spanned(
            output,
            "cacheable attribute requires Result<T, E> return type",
        ));
    };

    Ok(value_type.clone())
}

fn parse_unique_segment_list(
    input: ParseStream<'_>,
    current: &mut Option<SegmentList>,
    option: &Ident,
) -> syn::Result<()> {
    if current.is_some() {
        return Err(syn::Error::new(
            option.span(),
            format!("duplicate cacheable {} option", option),
        ));
    }

    *current = Some(input.parse()?);
    Ok(())
}

fn parse_unique_expr(
    input: ParseStream<'_>,
    current: &mut Option<Expr>,
    option: &Ident,
) -> syn::Result<()> {
    if current.is_some() {
        return Err(syn::Error::new(
            option.span(),
            format!("duplicate cacheable {} option", option),
        ));
    }

    *current = Some(input.parse()?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand_to_string(input: TokenStream2) -> String {
        expand_loader(input).unwrap().to_string()
    }

    fn expand_infallible_to_string(input: TokenStream2) -> String {
        expand_infallible(input).unwrap().to_string()
    }

    fn expand_attribute_to_string(args: TokenStream2, item: TokenStream2) -> String {
        expand_attribute(args, item).unwrap().to_string()
    }

    #[test]
    fn expands_minimal_cacheable_loader() {
        let output = expand_to_string(quote! {
            cache = cache,
            key = "expensive:1",
            load = || async { Ok::<_, LoaderError>(1_u64) },
        });

        assert!(output.contains("CacheOptions :: new"));
        assert!(output.contains(". get_or_load (\"expensive:1\""));
        assert!(output.contains("Ok :: < _ , LoaderError > (1_u64)"));
    }

    #[test]
    fn expands_repeated_tags_and_ttl_secs() {
        let output = expand_to_string(quote! {
            cache = cache,
            key = key,
            tag = "expensive",
            tag = format!("user:{user_id}"),
            ttl_secs = 60,
            load = loader,
        });

        assert!(output.contains(". tag (\"expensive\")"));
        assert!(output.contains(". tag (format ! (\"user:{user_id}\"))"));
        assert!(output.contains("Duration :: from_secs (60)"));
    }

    #[test]
    fn expands_tags_expression_before_repeated_tags() {
        let output = expand_to_string(quote! {
            cache = cache,
            key = key,
            tags = ["expensive", "reports"],
            tag = format!("user:{user_id}"),
            load = loader,
        });

        assert!(output.contains(". tags ([\"expensive\" , \"reports\"])"));
        assert!(output.contains(". tag (format ! (\"user:{user_id}\"))"));
    }

    #[test]
    fn expands_infallible_loader() {
        let output = expand_infallible_to_string(quote! {
            cache = cache,
            key = "expensive:1",
            tags = tags,
            ttl_secs = 60,
            load = || async { 1_u64 },
        });

        assert!(output.contains(". get_or_insert_with (\"expensive:1\""));
        assert!(output.contains(". tags (tags)"));
        assert!(output.contains("Duration :: from_secs (60)"));
        assert!(!output.contains(". get_or_load"));
    }

    #[test]
    fn expands_ttl_expr() {
        let output = expand_to_string(quote! {
            cache = cache,
            key = key,
            ttl = ttl,
            load = loader,
        });

        assert!(output.contains(". ttl (ttl)"));
        assert!(!output.contains("Duration :: from_secs"));
    }

    #[test]
    fn expands_cacheable_attribute_with_segmented_key_and_tags() {
        let output = expand_attribute_to_string(
            quote! {
                cache = cache,
                key_segments = ["profile", profile_id],
                tag_segments = [["profile", profile_id], ["profiles"]],
                ttl_secs = 60,
            },
            quote! {
                async fn load_profile(
                    cache: &HydraCache,
                    profile_id: u64,
                ) -> Result<Profile, LoadError> {
                    repo_load_profile(profile_id).await
                }
            },
        );

        assert!(output.contains("CacheResult < Profile >"));
        assert!(output.contains("CacheKeyBuilder :: new"));
        assert!(output.contains(". segment (& (\"profile\"))"));
        assert!(output.contains(". get_or_load"));
        assert!(output.contains("Duration :: from_secs (60)"));
        assert!(output.contains("repo_load_profile"));
    }

    #[test]
    fn rejects_cacheable_attribute_missing_key_source() {
        let error = expand_attribute(
            quote!(cache = cache),
            quote! {
                async fn load_profile() -> Result<Profile, LoadError> {
                    Ok(Profile)
                }
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("requires one key source"));
    }

    #[test]
    fn rejects_cacheable_attribute_non_async_function() {
        let error = expand_attribute(
            quote!(cache = cache, key = "profile:1"),
            quote! {
                fn load_profile() -> Result<Profile, LoadError> {
                    Ok(Profile)
                }
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("requires async fn"));
    }

    #[test]
    fn rejects_missing_cache() {
        let error = expand_loader(quote!(key = "one", load = loader)).unwrap_err();

        assert!(error.to_string().contains("requires cache"));
    }

    #[test]
    fn rejects_missing_cache_for_infallible_macro() {
        let error = expand_infallible(quote!(key = "one", load = loader)).unwrap_err();

        assert!(error
            .to_string()
            .contains("cacheable_infallible requires cache"));
    }

    #[test]
    fn rejects_missing_key() {
        let error = expand_loader(quote!(cache = cache, load = loader)).unwrap_err();

        assert!(error.to_string().contains("requires key"));
    }

    #[test]
    fn rejects_missing_load() {
        let error = expand_loader(quote!(cache = cache, key = "one")).unwrap_err();

        assert!(error.to_string().contains("requires load"));
    }

    #[test]
    fn rejects_duplicate_options() {
        let error = expand_loader(quote! {
            cache = first,
            cache = second,
            key = "one",
            load = loader,
        })
        .unwrap_err();

        assert!(error.to_string().contains("duplicate cacheable cache"));
    }

    #[test]
    fn rejects_duplicate_tags_expression() {
        let error = expand_loader(quote! {
            cache = cache,
            key = "one",
            tags = ["one"],
            tags = ["two"],
            load = loader,
        })
        .unwrap_err();

        assert!(error.to_string().contains("duplicate cacheable tags"));
    }

    #[test]
    fn rejects_unknown_options() {
        let error = expand_loader(quote! {
            cache = cache,
            key = "one",
            loader = loader,
        })
        .unwrap_err();

        assert!(error.to_string().contains("unsupported cacheable option"));
    }

    #[test]
    fn rejects_conflicting_ttl_options() {
        let error = expand_loader(quote! {
            cache = cache,
            key = "one",
            ttl = ttl,
            ttl_secs = 60,
            load = loader,
        })
        .unwrap_err();

        assert!(error.to_string().contains("only one TTL option"));
    }
}
