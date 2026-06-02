use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Attribute, DeriveInput, LitStr, Type};

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

    pub(crate) fn required_id(&self, input: &DeriveInput) -> syn::Result<&Type> {
        self.id
            .as_ref()
            .ok_or_else(|| syn::Error::new(input.ident.span(), "missing #[hydracache(id = Type)]"))
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
}
