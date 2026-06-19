use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Lit, LitStr, Token, Type};

use crate::paths::{
    cache_key_builder_path, declared_lint_mode_path, query_cache_policy_path, refresh_policy_path,
};

pub(crate) fn expand(input: TokenStream2) -> syn::Result<TokenStream2> {
    let config: PolicyConfig = syn::parse2(input)?;
    config.validate()?;
    Ok(config.expand())
}

#[derive(Default)]
struct PolicyConfig {
    preset: Option<Ident>,
    name: Option<Expr>,
    key: Option<Expr>,
    key_segments: Option<SegmentList>,
    collection: Option<Expr>,
    entity: Option<Type>,
    id: Option<Expr>,
    ttl: Option<Expr>,
    ttl_secs: Option<Expr>,
    refresh_ahead_secs: Option<Expr>,
    stale_while_revalidate_secs: Option<Expr>,
    stale_on_loader_error_secs: Option<Expr>,
    tags: Vec<Expr>,
    tag_segments: Vec<SegmentList>,
    collection_tags: Vec<Expr>,
    required_dimensions: Option<DimensionList>,
    sql: Option<Expr>,
    depends_on: Vec<Expr>,
    dependency_lint: Option<Ident>,
    lint_allow: Vec<LintAllow>,
}

impl Parse for PolicyConfig {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut config = Self::default();

        while !input.is_empty() {
            let option: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match option.to_string().as_str() {
                "preset" => parse_unique_ident(input, &mut config.preset, &option)?,
                "name" => parse_unique_expr(input, &mut config.name, &option)?,
                "key" => parse_unique_expr(input, &mut config.key, &option)?,
                "key_segments" => {
                    parse_unique_segment_list(input, &mut config.key_segments, &option)?
                }
                "collection" => parse_unique_expr(input, &mut config.collection, &option)?,
                "entity" => parse_unique_type(input, &mut config.entity, &option)?,
                "id" => parse_unique_expr(input, &mut config.id, &option)?,
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
                "required_dimensions" => {
                    parse_unique_dimension_list(input, &mut config.required_dimensions, &option)?
                }
                "sql" => parse_unique_expr(input, &mut config.sql, &option)?,
                "depends_on" => parse_unique_expr_list(input, &mut config.depends_on, &option)?,
                "dependency_lint" => {
                    parse_unique_ident(input, &mut config.dependency_lint, &option)?
                }
                "lint_allow" => {
                    parse_unique_lint_allow_list(input, &mut config.lint_allow, &option)?
                }
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
        if let Some(preset) = &self.preset {
            validate_preset(preset)?;
        }

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
            self.key_segments.is_some(),
            self.collection.is_some(),
            self.entity.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count();

        if key_sources == 0 {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "query_cache_policy requires one key source: key, key_segments, collection, or entity + id",
            ));
        }

        if key_sources > 1 {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "query_cache_policy accepts only one key source: key, key_segments, collection, or entity + id",
            ));
        }

        if self.ttl.is_some() && self.ttl_secs.is_some() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "query_cache_policy accepts only one TTL option: ttl or ttl_secs",
            ));
        }

        if self.preset.is_some() && (self.ttl.is_some() || self.ttl_secs.is_some()) {
            return Err(syn::Error::new(
                self.preset
                    .as_ref()
                    .map_or(proc_macro2::Span::call_site(), Ident::span),
                "query_cache_policy preset cannot be combined with ttl or ttl_secs",
            ));
        }

        self.validate_required_dimensions()?;
        self.validate_dependency_lint()?;

        Ok(())
    }

    fn expand(&self) -> TokenStream2 {
        let policy_path = query_cache_policy_path();
        let refresh_path = refresh_policy_path();
        let lint_mode_path = declared_lint_mode_path();
        let key_builder_path = cache_key_builder_path();
        let base = match &self.preset {
            Some(preset) => {
                let preset_call = preset_call(preset);
                quote!(#policy_path::#preset_call())
            }
            None => match &self.name {
                Some(name) => quote!(#policy_path::named(#name)),
                None => quote!(#policy_path::new()),
            },
        };
        let preset_name = if self.preset.is_some() {
            self.name.as_ref().map(|name| quote!(.with_name(#name)))
        } else {
            None
        };

        let key_source = if let Some(key) = &self.key {
            quote!(.key(#key))
        } else if let Some(key_segments) = &self.key_segments {
            let key_builder = segment_builder_tokens(&key_builder_path, &key_segments.segments);
            quote!(.key_builder(#key_builder))
        } else if let Some(collection) = &self.collection {
            quote!(.collection(#collection))
        } else {
            let entity = self.entity.as_ref().expect("validated entity should exist");
            let id = self.id.as_ref().expect("validated id should exist");
            quote!(.for_cache_entity::<#entity>(#id))
        };

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
        let required_dimensions = self.required_dimensions.as_ref().map(|dimensions| {
            let dimensions = dimensions.labels.iter();
            quote!(.required_dimensions([#(#dimensions),*]))
        });
        let sql = self.sql.as_ref().map(|sql| quote!(.lint_sql(#sql)));
        let depends_on = self
            .depends_on
            .iter()
            .map(|relation| quote!(.declared_dependency(#relation)));
        let dependency_lint = self.dependency_lint.as_ref().map(|mode| {
            let mode = dependency_lint_variant(mode);
            quote!(.dependency_lint_mode(#lint_mode_path::#mode))
        });
        let lint_allow = self.lint_allow.iter().map(|allow| {
            let finding = &allow.finding;
            let reason = &allow.reason;
            quote!(.lint_allow(#finding, #reason))
        });
        let refresh_policy = self.refresh_policy_tokens(&refresh_path);

        quote! {
            #base
                #preset_name
                #key_source
                #(#tags)*
                #(#segment_tags)*
                #(#collection_tags)*
                #ttl
                #ttl_secs
                #required_dimensions
                #sql
                #(#depends_on)*
                #dependency_lint
                #(#lint_allow)*
                #refresh_policy
        }
    }

    fn validate_required_dimensions(&self) -> syn::Result<()> {
        let Some(required_dimensions) = &self.required_dimensions else {
            return Ok(());
        };

        let Some(key_segments) = &self.key_segments else {
            let span = required_dimensions
                .labels
                .first()
                .map_or(proc_macro2::Span::call_site(), LitStr::span);
            return Err(syn::Error::new(
                span,
                "query_cache_policy required_dimensions requires key_segments",
            ));
        };

        let labels = key_segments.dimension_labels();
        for required in &required_dimensions.labels {
            if !labels.contains(&required.value()) {
                return Err(syn::Error::new(
                    required.span(),
                    format!(
                        "query_cache_policy required dimension `{}` is missing from key_segments",
                        required.value()
                    ),
                ));
            }
        }

        Ok(())
    }

    fn validate_dependency_lint(&self) -> syn::Result<()> {
        if let Some(mode) = &self.dependency_lint {
            match mode.to_string().as_str() {
                "warn" | "deny_missing_dependencies" => {}
                _ => {
                    return Err(syn::Error::new(
                        mode.span(),
                        "unsupported query_cache_policy dependency_lint mode",
                    ));
                }
            }
        }

        if self.dependency_lint.is_some() && self.sql.is_none() {
            let span = self
                .dependency_lint
                .as_ref()
                .map_or(proc_macro2::Span::call_site(), Ident::span);
            return Err(syn::Error::new(
                span,
                "query_cache_policy dependency_lint requires sql",
            ));
        }

        Ok(())
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

impl SegmentList {
    fn dimension_labels(&self) -> BTreeSet<String> {
        self.segments
            .iter()
            .step_by(2)
            .filter_map(string_literal_expr)
            .collect()
    }
}

impl Parse for SegmentList {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);
        let segments =
            parse_segment_exprs(&content, "query_cache_policy segment list cannot be empty")?;

        Ok(Self { segments })
    }
}

struct SegmentGroups {
    groups: Vec<SegmentList>,
}

struct DimensionList {
    labels: Vec<LitStr>,
}

struct LintAllow {
    finding: Expr,
    reason: LitStr,
}

impl Parse for DimensionList {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);
        let mut labels = Vec::new();
        let mut seen = BTreeSet::new();

        while !content.is_empty() {
            let label: LitStr = content.parse()?;
            if label.value().is_empty() {
                return Err(syn::Error::new(
                    label.span(),
                    "query_cache_policy required_dimensions labels cannot be empty",
                ));
            }
            if !seen.insert(label.value()) {
                return Err(syn::Error::new(
                    label.span(),
                    "duplicate query_cache_policy required_dimensions label",
                ));
            }
            labels.push(label);

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else if !content.is_empty() {
                return Err(content.error(
                    "query_cache_policy required_dimensions expects comma-separated string labels",
                ));
            }
        }

        if labels.is_empty() {
            Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "query_cache_policy required_dimensions cannot be empty",
            ))
        } else {
            Ok(Self { labels })
        }
    }
}

impl Parse for SegmentGroups {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);

        if content.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "query_cache_policy tag_segments requires at least one segment group",
            ));
        }

        let mut groups = Vec::new();
        while !content.is_empty() {
            if !content.peek(syn::token::Bracket) {
                return Err(content.error(
                    "query_cache_policy tag_segments expects nested segment groups like [[...], [...]]",
                ));
            }

            groups.push(content.parse()?);

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else if !content.is_empty() {
                return Err(content.error(
                    "query_cache_policy tag_segments expects comma-separated segment groups",
                ));
            }
        }

        Ok(Self { groups })
    }
}

impl Parse for LintAllow {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);
        let finding: Expr = content.parse()?;
        content.parse::<Token![,]>()?;
        let reason: LitStr = content.parse()?;
        if reason.value().trim().is_empty() {
            return Err(syn::Error::new(
                reason.span(),
                "query_cache_policy lint_allow reason cannot be empty",
            ));
        }
        Ok(Self { finding, reason })
    }
}

fn parse_segment_exprs(input: ParseStream<'_>, empty_message: &str) -> syn::Result<Vec<Expr>> {
    let mut segments = Vec::new();

    while !input.is_empty() {
        segments.push(input.parse()?);

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        } else if !input.is_empty() {
            return Err(
                input.error("query_cache_policy segment list expects comma-separated expressions")
            );
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

fn parse_unique_ident(
    input: ParseStream<'_>,
    current: &mut Option<Ident>,
    option: &Ident,
) -> syn::Result<()> {
    reject_duplicate(current, option)?;
    *current = Some(input.parse()?);
    Ok(())
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

fn parse_unique_dimension_list(
    input: ParseStream<'_>,
    current: &mut Option<DimensionList>,
    option: &Ident,
) -> syn::Result<()> {
    reject_duplicate(current, option)?;
    *current = Some(input.parse()?);
    Ok(())
}

fn parse_unique_expr_list(
    input: ParseStream<'_>,
    current: &mut Vec<Expr>,
    option: &Ident,
) -> syn::Result<()> {
    if !current.is_empty() {
        return Err(syn::Error::new(
            option.span(),
            format!("duplicate query_cache_policy {} option", option),
        ));
    }

    let content;
    syn::bracketed!(content in input);
    while !content.is_empty() {
        current.push(content.parse()?);
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        } else if !content.is_empty() {
            return Err(content.error(format!(
                "query_cache_policy {} expects comma-separated expressions",
                option
            )));
        }
    }

    if current.is_empty() {
        Err(syn::Error::new(
            option.span(),
            format!("query_cache_policy {} cannot be empty", option),
        ))
    } else {
        Ok(())
    }
}

fn parse_unique_lint_allow_list(
    input: ParseStream<'_>,
    current: &mut Vec<LintAllow>,
    option: &Ident,
) -> syn::Result<()> {
    if !current.is_empty() {
        return Err(syn::Error::new(
            option.span(),
            "duplicate query_cache_policy lint_allow option",
        ));
    }

    let content;
    syn::bracketed!(content in input);
    while !content.is_empty() {
        current.push(content.parse()?);
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        } else if !content.is_empty() {
            return Err(
                content.error("query_cache_policy lint_allow expects comma-separated pairs")
            );
        }
    }

    if current.is_empty() {
        Err(syn::Error::new(
            option.span(),
            "query_cache_policy lint_allow cannot be empty",
        ))
    } else {
        Ok(())
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

fn validate_preset(preset: &Ident) -> syn::Result<()> {
    match preset.to_string().as_str() {
        "short_lived"
        | "read_mostly"
        | "per_entity"
        | "no_ttl_explicit_invalidation"
        | "negative_cache" => Ok(()),
        _ => Err(syn::Error::new(
            preset.span(),
            "unsupported query_cache_policy preset",
        )),
    }
}

fn preset_call(preset: &Ident) -> Ident {
    let name = preset.to_string();
    Ident::new(&name, preset.span())
}

fn dependency_lint_variant(mode: &Ident) -> Ident {
    let variant = match mode.to_string().as_str() {
        "warn" => "Warn",
        "deny_missing_dependencies" => "DenyMissingDependencies",
        _ => unreachable!("dependency lint mode validated"),
    };
    Ident::new(variant, mode.span())
}

fn string_literal_expr(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Str(value) => Some(value.value()),
            _ => None,
        },
        _ => None,
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
    fn expands_preset_policy_with_name_and_refresh_options() {
        let output = expand_to_string(quote! {
            preset = read_mostly,
            name = "load-user",
            key = "user:42",
            refresh_ahead_secs = 10,
            stale_while_revalidate_secs = 300,
            stale_on_loader_error_secs = 600,
        });

        assert!(output.contains("QueryCachePolicy :: read_mostly"));
        assert!(output.contains(". with_name (\"load-user\")"));
        assert!(output.contains(". key (\"user:42\")"));
        assert!(output.contains("RefreshPolicy :: new"));
        assert!(output.contains(". refresh_ahead"));
        assert!(output.contains(". stale_while_revalidate"));
        assert!(output.contains(". stale_on_loader_error"));
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
    fn expands_segmented_key_and_tags() {
        let output = expand_to_string(quote! {
            name = "search-users",
            key_segments = ["tenant", tenant_id, "q", query, "page", page],
            tag_segments = [["tenant", tenant_id], ["users"]],
            ttl_secs = 30,
        });

        assert!(output.contains("CacheKeyBuilder :: new"));
        assert!(output.contains(". key_builder"));
        assert!(output.contains(". segment (\"tenant\")"));
        assert!(output.contains(". segment (tenant_id)"));
        assert!(output.contains(". tag"));
        assert!(output.contains(". build_string"));
        assert!(output.contains("Duration :: from_secs (30)"));
    }

    #[test]
    fn expands_required_dimensions_for_segmented_key() {
        let output = expand_to_string(quote! {
            name = "search-users",
            key_segments = ["tenant", tenant_id, "q", query, "page", page],
            required_dimensions = ["tenant", "q", "page"],
            ttl_secs = 30,
        });

        assert!(output.contains(". required_dimensions"));
        assert!(output.contains("\"tenant\""));
        assert!(output.contains("\"q\""));
        assert!(output.contains("\"page\""));
    }

    #[test]
    fn expands_sql_dependency_lint_metadata() {
        let output = expand_to_string(quote! {
            name = "load-user-permissions",
            key_segments = ["tenant", tenant_id, "user", user_id],
            sql = "select * from users join user_roles on true",
            depends_on = [table("users"), table("user_roles")],
            dependency_lint = deny_missing_dependencies,
            lint_allow = [(LintFinding::Inconclusive, "dynamic fragment audited")],
            ttl_secs = 30,
        });

        assert!(output.contains(". lint_sql"));
        assert!(output.contains(". declared_dependency"));
        assert!(output.contains("DeclaredLintMode :: DenyMissingDependencies"));
        assert!(output.contains(". lint_allow"));
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
        let error = expand(quote!(key = "user:1", key_segments = ["user", 1])).unwrap_err();

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

    #[test]
    fn rejects_unknown_preset() {
        let error = expand(quote! {
            preset = catalog,
            key = "one",
        })
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("unsupported query_cache_policy preset"));
    }

    #[test]
    fn rejects_unknown_dependency_lint_mode() {
        let error = expand(quote! {
            key = "one",
            sql = "select * from users",
            dependency_lint = strict,
        })
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("unsupported query_cache_policy dependency_lint mode"));
    }

    #[test]
    fn rejects_dependency_lint_without_sql() {
        let error = expand(quote! {
            key = "one",
            dependency_lint = warn,
        })
        .unwrap_err();

        assert!(error.to_string().contains("dependency_lint requires sql"));
    }

    #[test]
    fn rejects_duplicate_sql() {
        let error = expand(quote! {
            key = "one",
            sql = "select 1",
            sql = "select 2",
        })
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("duplicate query_cache_policy sql option"));
    }

    #[test]
    fn rejects_empty_lint_allow_reason() {
        let error = expand(quote! {
            key = "one",
            lint_allow = [(LintFinding::Inconclusive, "")],
        })
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("lint_allow reason cannot be empty"));
    }

    #[test]
    fn rejects_preset_with_ttl_override() {
        let error = expand(quote! {
            preset = read_mostly,
            key = "one",
            ttl_secs = 60,
        })
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("preset cannot be combined with ttl or ttl_secs"));
    }

    #[test]
    fn rejects_empty_key_segments() {
        let error = expand(quote!(key_segments = [])).unwrap_err();

        assert!(error.to_string().contains("segment list cannot be empty"));
    }

    #[test]
    fn rejects_required_dimensions_without_key_segments() {
        let error = expand(quote! {
            key = "users",
            required_dimensions = ["tenant"],
        })
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("required_dimensions requires key_segments"));
    }

    #[test]
    fn rejects_missing_required_dimension_label() {
        let error = expand(quote! {
            key_segments = ["tenant", tenant_id, "page", page],
            required_dimensions = ["tenant", "permission"],
        })
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("required dimension `permission` is missing from key_segments"));
    }

    #[test]
    fn rejects_duplicate_required_dimension_labels() {
        let error = expand(quote! {
            key_segments = ["tenant", tenant_id],
            required_dimensions = ["tenant", "tenant"],
        })
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("duplicate query_cache_policy required_dimensions label"));
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
