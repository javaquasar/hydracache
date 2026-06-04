use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Token};

use crate::paths::cache_options_path;

pub(crate) fn expand(input: TokenStream2) -> syn::Result<TokenStream2> {
    let config: CacheableConfig = syn::parse2(input)?;
    config.validate()?;
    Ok(config.expand())
}

#[derive(Default)]
struct CacheableConfig {
    cache: Option<Expr>,
    key: Option<Expr>,
    load: Option<Expr>,
    ttl: Option<Expr>,
    ttl_secs: Option<Expr>,
    tags: Vec<Expr>,
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
                "tag" => config.tags.push(input.parse()?),
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

impl CacheableConfig {
    fn validate(&self) -> syn::Result<()> {
        if self.cache.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "cacheable requires cache",
            ));
        }

        if self.key.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "cacheable requires key",
            ));
        }

        if self.load.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "cacheable requires load",
            ));
        }

        if self.ttl.is_some() && self.ttl_secs.is_some() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "cacheable accepts only one TTL option: ttl or ttl_secs",
            ));
        }

        Ok(())
    }

    fn expand(&self) -> TokenStream2 {
        let options_path = cache_options_path();
        let cache = self.cache.as_ref().expect("validated cache should exist");
        let key = self.key.as_ref().expect("validated key should exist");
        let load = self.load.as_ref().expect("validated load should exist");
        let tags = self.tags.iter().map(|tag| quote!(.tag(#tag)));
        let ttl = self.ttl.as_ref().map(|ttl| quote!(.ttl(#ttl)));
        let ttl_secs = self
            .ttl_secs
            .as_ref()
            .map(|ttl_secs| quote!(.ttl(::std::time::Duration::from_secs(#ttl_secs))));

        quote! {{
            let __hydracache_options = #options_path::new()
                #(#tags)*
                #ttl
                #ttl_secs;
            (#cache).get_or_load(#key, __hydracache_options, #load)
        }}
    }
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
        expand(input).unwrap().to_string()
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
    fn expands_tags_and_ttl_secs() {
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
    fn rejects_missing_cache() {
        let error = expand(quote!(key = "one", load = loader)).unwrap_err();

        assert!(error.to_string().contains("requires cache"));
    }

    #[test]
    fn rejects_missing_key() {
        let error = expand(quote!(cache = cache, load = loader)).unwrap_err();

        assert!(error.to_string().contains("requires key"));
    }

    #[test]
    fn rejects_missing_load() {
        let error = expand(quote!(cache = cache, key = "one")).unwrap_err();

        assert!(error.to_string().contains("requires load"));
    }

    #[test]
    fn rejects_duplicate_options() {
        let error = expand(quote! {
            cache = first,
            cache = second,
            key = "one",
            load = loader,
        })
        .unwrap_err();

        assert!(error.to_string().contains("duplicate cacheable cache"));
    }

    #[test]
    fn rejects_unknown_options() {
        let error = expand(quote! {
            cache = cache,
            key = "one",
            loader = loader,
        })
        .unwrap_err();

        assert!(error.to_string().contains("unsupported cacheable option"));
    }

    #[test]
    fn rejects_conflicting_ttl_options() {
        let error = expand(quote! {
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
