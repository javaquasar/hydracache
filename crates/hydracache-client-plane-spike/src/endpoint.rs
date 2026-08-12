//! Strict, canonical HC/2 endpoint identities.

use std::net::{Ipv4Addr, Ipv6Addr};
use std::num::NonZeroU16;

use thiserror::Error;

use crate::TransportCandidate;

const MAX_ENDPOINT_URI_BYTES: usize = 512;
const MAX_DNS_NAME_BYTES: usize = 253;

/// Whether an address came from explicit operator bootstrap configuration or
/// from an authenticated cluster advertisement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointOrigin {
    ConfiguredBootstrap,
    AuthenticatedDiscovery,
}

/// Canonical host identity. Raw input spelling is deliberately not retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointHost {
    Dns(String),
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
}

/// Parsed endpoint identity used by policy and equality checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportAuthority {
    candidate: TransportCandidate,
    host: EndpointHost,
    port: NonZeroU16,
    server_name: String,
    origin: EndpointOrigin,
}

impl TransportAuthority {
    /// Parse an HC/2 endpoint and bind its scheme to the selected adapter.
    pub fn parse(
        candidate: TransportCandidate,
        uri: &str,
        server_name: &str,
        origin: EndpointOrigin,
    ) -> Result<Self, EndpointParseError> {
        if uri.is_empty() || uri.len() > MAX_ENDPOINT_URI_BYTES || !uri.is_ascii() {
            return Err(EndpointParseError::InvalidUri);
        }
        if uri.contains('@') || uri.contains('/') || uri.contains('?') || uri.contains('#') {
            // The `://` separator is the only slash sequence permitted.
            let separator = uri.find("://").ok_or(EndpointParseError::InvalidUri)?;
            if uri[separator + 3..].contains(['@', '/', '?', '#']) {
                return Err(EndpointParseError::ForbiddenComponent);
            }
        }

        let (scheme, authority) = uri
            .split_once("://")
            .ok_or(EndpointParseError::InvalidUri)?;
        if scheme != expected_scheme(candidate) {
            return Err(EndpointParseError::WrongScheme);
        }
        let (host, port) = parse_authority(authority)?;
        let server_name =
            canonical_dns(server_name).ok_or(EndpointParseError::InvalidServerName)?;
        if let EndpointHost::Dns(dns) = &host {
            if dns != &server_name {
                return Err(EndpointParseError::ServerNameMismatch);
            }
        }
        if origin == EndpointOrigin::AuthenticatedDiscovery && !advertisable(&host) {
            return Err(EndpointParseError::UnadvertisableAddress);
        }

        Ok(Self {
            candidate,
            host,
            port,
            server_name,
            origin,
        })
    }

    pub fn candidate(&self) -> TransportCandidate {
        self.candidate
    }

    pub fn host(&self) -> &EndpointHost {
        &self.host
    }

    pub fn port(&self) -> NonZeroU16 {
        self.port
    }

    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    pub fn origin(&self) -> EndpointOrigin {
        self.origin
    }

    /// Stable canonical identity; it is generated from parsed fields and is
    /// never used as an input to security decisions.
    pub fn canonical_uri(&self) -> String {
        let host = match &self.host {
            EndpointHost::Dns(name) => name.clone(),
            EndpointHost::Ipv4(address) => address.to_string(),
            EndpointHost::Ipv6(address) => format!("[{address}]"),
        };
        format!(
            "{}://{}:{}",
            expected_scheme(self.candidate),
            host,
            self.port
        )
    }
}

fn expected_scheme(candidate: TransportCandidate) -> &'static str {
    match candidate {
        TransportCandidate::GrpcBidirectional => "hc2+grpc",
        TransportCandidate::Http2Bidirectional => "hc2+h2",
        TransportCandidate::DedicatedTcpTls => "hc2+tls",
    }
}

fn parse_authority(authority: &str) -> Result<(EndpointHost, NonZeroU16), EndpointParseError> {
    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        let (host, suffix) = rest
            .split_once(']')
            .ok_or(EndpointParseError::InvalidHost)?;
        let port = suffix
            .strip_prefix(':')
            .ok_or(EndpointParseError::MissingPort)?;
        let address = host
            .parse::<Ipv6Addr>()
            .map_err(|_| EndpointParseError::InvalidHost)?;
        (EndpointHost::Ipv6(address), port)
    } else {
        let (host, port) = authority
            .rsplit_once(':')
            .ok_or(EndpointParseError::MissingPort)?;
        if host.contains(':') {
            return Err(EndpointParseError::InvalidHost);
        }
        let host = if let Ok(address) = host.parse::<Ipv4Addr>() {
            EndpointHost::Ipv4(address)
        } else {
            EndpointHost::Dns(canonical_dns(host).ok_or(EndpointParseError::InvalidHost)?)
        };
        (host, port)
    };
    let port = port
        .parse::<u16>()
        .ok()
        .and_then(NonZeroU16::new)
        .ok_or(EndpointParseError::InvalidPort)?;
    Ok((host, port))
}

fn canonical_dns(value: &str) -> Option<String> {
    if value.is_empty()
        || value.len() > MAX_DNS_NAME_BYTES
        || !value.is_ascii()
        || value.ends_with('.')
    {
        return None;
    }
    for label in value.split('.') {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return None;
        }
    }
    Some(value.to_ascii_lowercase())
}

fn advertisable(host: &EndpointHost) -> bool {
    match host {
        EndpointHost::Dns(name) => name != "localhost",
        EndpointHost::Ipv4(address) => !address.is_unspecified() && !address.is_multicast(),
        EndpointHost::Ipv6(address) => !address.is_unspecified() && !address.is_multicast(),
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EndpointParseError {
    #[error("endpoint URI is empty, non-ASCII, oversized, or malformed")]
    InvalidUri,
    #[error("endpoint URI contains userinfo, path, query, or fragment")]
    ForbiddenComponent,
    #[error("endpoint scheme does not match the selected transport")]
    WrongScheme,
    #[error("endpoint host is invalid or ambiguous")]
    InvalidHost,
    #[error("endpoint requires an explicit port")]
    MissingPort,
    #[error("endpoint port must be in 1..=65535")]
    InvalidPort,
    #[error("TLS server name must be an explicit canonical ASCII DNS name")]
    InvalidServerName,
    #[error("DNS endpoint host and TLS server name differ")]
    ServerNameMismatch,
    #[error("authenticated discovery cannot advertise this host address")]
    UnadvertisableAddress,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(
        candidate: TransportCandidate,
        uri: &str,
        sni: &str,
    ) -> Result<TransportAuthority, EndpointParseError> {
        TransportAuthority::parse(candidate, uri, sni, EndpointOrigin::AuthenticatedDiscovery)
    }

    #[test]
    fn canonicalizes_dns_ipv4_and_bracketed_ipv6() {
        let dns = parse(
            TransportCandidate::GrpcBidirectional,
            "hc2+grpc://CACHE.Example:7443",
            "cache.example",
        )
        .unwrap();
        assert_eq!(dns.canonical_uri(), "hc2+grpc://cache.example:7443");

        let ipv4 = parse(
            TransportCandidate::DedicatedTcpTls,
            "hc2+tls://192.0.2.10:7445",
            "cache.example",
        )
        .unwrap();
        assert_eq!(ipv4.canonical_uri(), "hc2+tls://192.0.2.10:7445");

        let ipv6 = parse(
            TransportCandidate::Http2Bidirectional,
            "hc2+h2://[2001:db8::1]:7444",
            "cache.example",
        )
        .unwrap();
        assert_eq!(ipv6.canonical_uri(), "hc2+h2://[2001:db8::1]:7444");
    }

    #[test]
    fn rejects_ambiguous_or_unsafe_uri_components() {
        let cases = [
            "hc2+grpc://user@cache.example:7443",
            "hc2+grpc://cache.example:7443/path",
            "hc2+grpc://cache.example:7443?x=1",
            "hc2+grpc://cache.example:7443#fragment",
            "hc2+grpc://2001:db8::1:7443",
        ];
        for uri in cases {
            assert!(
                parse(TransportCandidate::GrpcBidirectional, uri, "cache.example").is_err(),
                "{uri}"
            );
        }
    }

    #[test]
    fn rejects_wrong_scheme_port_idn_and_sni_mismatch() {
        assert_eq!(
            parse(
                TransportCandidate::GrpcBidirectional,
                "hc2+h2://cache.example:7443",
                "cache.example"
            ),
            Err(EndpointParseError::WrongScheme)
        );
        for uri in [
            "hc2+grpc://cache.example",
            "hc2+grpc://cache.example:0",
            "hc2+grpc://cache.example:65536",
        ] {
            assert!(
                parse(TransportCandidate::GrpcBidirectional, uri, "cache.example").is_err(),
                "{uri}"
            );
        }
        assert_eq!(
            parse(
                TransportCandidate::GrpcBidirectional,
                "hc2+grpc://caché.example:7443",
                "caché.example"
            ),
            Err(EndpointParseError::InvalidUri)
        );
        assert_eq!(
            parse(
                TransportCandidate::GrpcBidirectional,
                "hc2+grpc://cache.example:7443",
                "other.example"
            ),
            Err(EndpointParseError::ServerNameMismatch)
        );
    }

    #[test]
    fn discovery_rejects_non_routable_advertisements_but_bootstrap_keeps_local_test_access() {
        assert_eq!(
            parse(
                TransportCandidate::GrpcBidirectional,
                "hc2+grpc://0.0.0.0:7443",
                "cache.example"
            ),
            Err(EndpointParseError::UnadvertisableAddress)
        );
        assert!(TransportAuthority::parse(
            TransportCandidate::GrpcBidirectional,
            "hc2+grpc://localhost:7443",
            "localhost",
            EndpointOrigin::ConfiguredBootstrap,
        )
        .is_ok());
    }
}
