use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, Data, DeriveInput, Field, Fields, LitStr, Type};

#[derive(Default)]
pub(crate) struct EntityConfig {
    entity: Option<LitStr>,
    collection: Option<LitStr>,
    id: Option<Type>,
}

impl EntityConfig {
    pub(crate) fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut config = Self::default();

        for attr in attrs
            .iter()
            .filter(|attr| attr.path().is_ident("hydracache"))
        {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("entity") {
                    reject_duplicate(&config.entity, &meta, "entity")?;
                    config.entity = Some(meta.value()?.parse()?);
                    Ok(())
                } else if meta.path.is_ident("collection") {
                    reject_duplicate(&config.collection, &meta, "collection")?;
                    config.collection = Some(meta.value()?.parse()?);
                    Ok(())
                } else if meta.path.is_ident("id") {
                    reject_duplicate(&config.id, &meta, "id")?;
                    config.id = Some(meta.value()?.parse()?);
                    Ok(())
                } else {
                    Err(meta
                        .error("unsupported hydracache option; expected entity, collection, or id"))
                }
            })?;
        }

        Ok(config)
    }

    pub(crate) fn required_entity(&self, input: &DeriveInput) -> syn::Result<&LitStr> {
        self.entity.as_ref().ok_or_else(|| {
            syn::Error::new(
                input.ident.span(),
                "missing #[hydracache(entity = \"...\")]",
            )
        })
    }

    pub(crate) fn required_id(&self, input: &DeriveInput) -> syn::Result<Type> {
        let inferred_id = inferred_id_type(input)?;

        match (&self.id, inferred_id) {
            (Some(explicit_id), None) => Ok(explicit_id.clone()),
            (None, Some(inferred_id)) => Ok(inferred_id),
            (Some(_), Some(_)) => Err(syn::Error::new(
                input.ident.span(),
                "conflicting hydracache id metadata; use either #[hydracache(id = Type)] on the struct or one #[hydracache(id)] field",
            )),
            (None, None) => Err(syn::Error::new(
                input.ident.span(),
                "missing #[hydracache(id = Type)] or #[hydracache(id)] field",
            )),
        }
    }

    pub(crate) fn collection_tokens(&self) -> TokenStream2 {
        match &self.collection {
            Some(collection) => quote!(Some(#collection)),
            None => quote!(None),
        }
    }

    #[cfg(test)]
    fn entity_value(&self) -> Option<String> {
        self.entity.as_ref().map(LitStr::value)
    }

    #[cfg(test)]
    fn collection_value(&self) -> Option<String> {
        self.collection.as_ref().map(LitStr::value)
    }

    #[cfg(test)]
    fn id_tokens(&self) -> TokenStream2 {
        let id = self.id.as_ref().expect("id should be present");
        quote!(#id)
    }
}

fn inferred_id_type(input: &DeriveInput) -> syn::Result<Option<Type>> {
    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => return Ok(None),
    };

    match fields {
        Fields::Named(fields) => {
            let mut id_type = None;
            for field in &fields.named {
                if field_id_marker_span(field)?.is_some() {
                    if id_type.is_some() {
                        return Err(syn::Error::new(
                            field.span(),
                            "duplicate hydracache id field marker",
                        ));
                    }
                    id_type = Some(field.ty.clone());
                }
            }
            Ok(id_type)
        }
        Fields::Unnamed(fields) => {
            for field in &fields.unnamed {
                if field_id_marker_span(field)?.is_some() {
                    return Err(syn::Error::new(
                        field.span(),
                        "field #[hydracache(id)] requires a named struct field",
                    ));
                }
            }
            Ok(None)
        }
        Fields::Unit => Ok(None),
    }
}

fn field_id_marker_span(field: &Field) -> syn::Result<Option<proc_macro2::Span>> {
    let mut id_marker = None;

    for attr in field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("hydracache"))
    {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("id") {
                if !meta.input.is_empty() {
                    return Err(meta.error(
                        "field #[hydracache(id)] does not accept a value; put id = Type on the struct or use #[hydracache(id)] on one named field",
                    ));
                }

                if id_marker.is_some() {
                    return Err(meta.error("duplicate hydracache id field marker"));
                }

                id_marker = Some(meta.path.span());
                Ok(())
            } else {
                Err(meta.error("unsupported hydracache field option; expected id"))
            }
        })?;
    }

    Ok(id_marker)
}

fn reject_duplicate<T>(
    current: &Option<T>,
    meta: &syn::meta::ParseNestedMeta<'_>,
    name: &str,
) -> syn::Result<()> {
    if current.is_some() {
        Err(meta.error(format!("duplicate hydracache {name} option")))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use syn::{parse_quote, DeriveInput};

    use super::*;

    fn parse_config(input: DeriveInput) -> syn::Result<EntityConfig> {
        EntityConfig::from_attrs(&input.attrs)
    }

    #[test]
    fn parses_entity_collection_and_id() {
        let input: DeriveInput = parse_quote! {
            #[hydracache(entity = "user", collection = "users", id = i64)]
            struct User;
        };

        let config = parse_config(input).unwrap();

        assert_eq!(config.entity_value().unwrap(), "user");
        assert_eq!(config.collection_value().unwrap(), "users");
        assert_eq!(config.id_tokens().to_string(), "i64");
    }

    #[test]
    fn infers_id_from_named_field_marker() {
        let input: DeriveInput = parse_quote! {
            #[hydracache(entity = "user", collection = "users")]
            struct User {
                #[hydracache(id)]
                id: i64,
                name: String,
            }
        };

        let config = parse_config(input.clone()).unwrap();
        let id = config.required_id(&input).unwrap();

        assert_eq!(quote!(#id).to_string(), "i64");
    }

    #[test]
    fn collection_is_optional() {
        let input: DeriveInput = parse_quote! {
            #[hydracache(entity = "profile", id = u64)]
            struct Profile;
        };

        let config = parse_config(input).unwrap();

        assert_eq!(config.collection_tokens().to_string(), "None");
    }

    #[test]
    fn accepts_split_hydracache_attributes() {
        let input: DeriveInput = parse_quote! {
            #[hydracache(entity = "account:user")]
            #[hydracache(collection = "users:active", id = &'static str)]
            struct AccountUser;
        };

        let config = parse_config(input).unwrap();

        assert_eq!(config.entity_value().unwrap(), "account:user");
        assert_eq!(config.collection_value().unwrap(), "users:active");
        assert_eq!(config.id_tokens().to_string(), "& 'static str");
    }

    #[test]
    fn rejects_duplicate_options() {
        let input: DeriveInput = parse_quote! {
            #[hydracache(entity = "user", entity = "profile", id = i64)]
            struct User;
        };

        let result = parse_config(input);
        assert!(result.is_err());
        let error = result.err().unwrap();

        assert!(error
            .to_string()
            .contains("duplicate hydracache entity option"));
    }

    #[test]
    fn rejects_unknown_options() {
        let input: DeriveInput = parse_quote! {
            #[hydracache(entity = "user", id = i64, table = "users")]
            struct User;
        };

        let result = parse_config(input);
        assert!(result.is_err());
        let error = result.err().unwrap();

        assert!(error.to_string().contains("unsupported hydracache option"));
    }

    #[test]
    fn rejects_conflicting_explicit_and_field_id_metadata() {
        let input: DeriveInput = parse_quote! {
            #[hydracache(entity = "user", id = i64)]
            struct User {
                #[hydracache(id)]
                id: i64,
            }
        };

        let config = parse_config(input.clone()).unwrap();
        let error = match config.required_id(&input) {
            Ok(_) => panic!("conflicting id metadata should fail"),
            Err(error) => error,
        };

        assert!(error
            .to_string()
            .contains("conflicting hydracache id metadata"));
    }

    #[test]
    fn rejects_duplicate_field_id_markers() {
        let input: DeriveInput = parse_quote! {
            #[hydracache(entity = "user")]
            struct User {
                #[hydracache(id)]
                id: i64,
                #[hydracache(id)]
                legacy_id: i64,
            }
        };

        let config = parse_config(input.clone()).unwrap();
        let error = match config.required_id(&input) {
            Ok(_) => panic!("duplicate field id markers should fail"),
            Err(error) => error,
        };

        assert!(error
            .to_string()
            .contains("duplicate hydracache id field marker"));
    }

    #[test]
    fn rejects_field_id_marker_with_value() {
        let input: DeriveInput = parse_quote! {
            #[hydracache(entity = "user")]
            struct User {
                #[hydracache(id = i64)]
                id: i64,
            }
        };

        let config = parse_config(input.clone()).unwrap();
        let error = match config.required_id(&input) {
            Ok(_) => panic!("field id marker with value should fail"),
            Err(error) => error,
        };

        assert!(error
            .to_string()
            .contains("field #[hydracache(id)] does not accept a value"));
    }
}
