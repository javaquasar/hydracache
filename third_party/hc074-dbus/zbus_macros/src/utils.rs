#[cfg(any(feature = "proxy", feature = "service"))]
use std::fmt::Display;

#[cfg(any(feature = "proxy", feature = "service"))]
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
#[cfg(feature = "service")]
use syn::Attribute;
#[cfg(any(feature = "proxy", feature = "service"))]
use syn::{FnArg, Ident, Pat, PatIdent, PatType};

/// Parses the `crate` attribute value into a path.
#[cfg(feature = "comms")]
pub fn parse_crate_path(crate_attr: Option<&str>) -> Result<Option<syn::Path>, syn::Error> {
    crate_attr.map(syn::parse_str).transpose()
}

/// Returns the path to the zbus crate.
///
/// If a custom crate path is provided via the `crate` attribute, it will be used.
/// Otherwise, defaults to `::zbus`.
pub fn zbus_path(crate_path: Option<&syn::Path>) -> TokenStream {
    if let Some(path) = crate_path {
        quote! { ::#path }
    } else {
        quote! { ::zbus }
    }
}

/// Shared-codegen configuration for the wire derives.
///
/// `#[zvariant(...)]` stays accepted next to `#[zbus(...)]`: code written against zvariant 5
/// keeps compiling unchanged.
pub fn derive_config() -> zbus_utils::derive::Config {
    zbus_utils::derive::Config {
        attr_lists: &["zbus", "zvariant"],
        default_path: wire_path(),
    }
}

/// Path of the wire-format module for generated code.
pub fn wire_path() -> TokenStream {
    let zbus = zbus_path(None);
    quote! { #zbus::wire }
}

#[cfg(any(feature = "proxy", feature = "service"))]
pub fn typed_arg(arg: &FnArg) -> Option<&PatType> {
    match arg {
        FnArg::Typed(t) => Some(t),
        _ => None,
    }
}

#[cfg(any(feature = "proxy", feature = "service"))]
pub fn pat_ident(pat: &PatType) -> Option<&Ident> {
    match &*pat.pat {
        Pat::Ident(PatIdent { ident, .. }) => Some(ident),
        _ => None,
    }
}

#[cfg(feature = "service")]
pub fn get_doc_attrs(attrs: &[Attribute]) -> Vec<&Attribute> {
    attrs.iter().filter(|x| x.path().is_ident("doc")).collect()
}

// Convert to pascal case, assuming snake case.
// If `s` is already in pascal case, should yield the same result.
#[cfg(feature = "service")]
pub fn pascal_case(s: &str) -> String {
    let mut pascal = String::new();
    let mut capitalize = true;
    for ch in s.chars() {
        if ch == '_' {
            capitalize = true;
        } else if capitalize {
            pascal.push(ch.to_ascii_uppercase());
            capitalize = false;
        } else {
            pascal.push(ch);
        }
    }
    pascal
}

#[cfg(feature = "service")]
pub fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// Standard annotation `org.freedesktop.DBus.Property.EmitsChangedSignal`.
///
/// See <https://dbus.freedesktop.org/doc/dbus-specification.html#introspection-format>.
#[cfg(any(feature = "proxy", feature = "service"))]
#[derive(Debug, Default, Clone, PartialEq)]
pub enum PropertyEmitsChangedSignal {
    #[default]
    True,
    Invalidates,
    Const,
    False,
}

#[cfg(any(feature = "proxy", feature = "service"))]
impl Display for PropertyEmitsChangedSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let emits_changed_signal = match self {
            PropertyEmitsChangedSignal::True => "true",
            PropertyEmitsChangedSignal::Const => "const",
            PropertyEmitsChangedSignal::False => "false",
            PropertyEmitsChangedSignal::Invalidates => "invalidates",
        };
        write!(f, "{emits_changed_signal}")
    }
}

#[cfg(any(feature = "proxy", feature = "service"))]
impl PropertyEmitsChangedSignal {
    pub fn parse(s: &str, span: Span) -> syn::Result<Self> {
        use PropertyEmitsChangedSignal::*;

        match s {
            "true" => Ok(True),
            "invalidates" => Ok(Invalidates),
            "const" => Ok(Const),
            "false" => Ok(False),
            other => Err(syn::Error::new(
                span,
                format!("invalid value \"{other}\" for attribute `property(emits_changed_signal)`"),
            )),
        }
    }
}
