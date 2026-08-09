//! Transport-neutral HC/2 security rejection taxonomy and certificate bounds.

use crate::policy::TransportFailure;

/// Stable, low-cardinality reason emitted for a pre-dispatch security rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityRejection {
    UnknownCa,
    MissingClientCertificate,
    HostnameMismatch,
    CertificateExpired,
    CertificateNotYetValid,
    WrongClientCa,
    InvalidExtendedKeyUsage,
    CertificateChainDepth,
    CertificateChainSize,
    TlsProtocolPolicy,
    AuthorizationDenied,
}

impl SecurityRejection {
    /// Stable metric/log label; certificate subjects and tenant names are never labels.
    pub const fn label(self) -> &'static str {
        match self {
            Self::UnknownCa => "unknown_ca",
            Self::MissingClientCertificate => "missing_client_certificate",
            Self::HostnameMismatch => "hostname_mismatch",
            Self::CertificateExpired => "certificate_expired",
            Self::CertificateNotYetValid => "certificate_not_yet_valid",
            Self::WrongClientCa => "wrong_client_ca",
            Self::InvalidExtendedKeyUsage => "invalid_extended_key_usage",
            Self::CertificateChainDepth => "certificate_chain_depth",
            Self::CertificateChainSize => "certificate_chain_size",
            Self::TlsProtocolPolicy => "tls_protocol_policy",
            Self::AuthorizationDenied => "authorization_denied",
        }
    }

    /// Security failures are terminal for the selected authenticated endpoint.
    pub const fn transport_failure(self) -> TransportFailure {
        match self {
            Self::MissingClientCertificate => TransportFailure::Authentication,
            Self::AuthorizationDenied => TransportFailure::Authorization,
            _ => TransportFailure::TlsVerification,
        }
    }
}

/// Explicit certificate-chain limits applied before HC/2 protocol dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlsPeerPolicy {
    max_chain_depth: usize,
    max_chain_bytes: usize,
}

impl TlsPeerPolicy {
    /// Construct nonzero bounds. Configuration is deliberately capped to keep
    /// the security policy itself from becoming an allocation escape hatch.
    pub fn new(max_chain_depth: usize, max_chain_bytes: usize) -> Option<Self> {
        if max_chain_depth == 0
            || max_chain_depth > 16
            || max_chain_bytes == 0
            || max_chain_bytes > 256 * 1024
        {
            return None;
        }
        Some(Self {
            max_chain_depth,
            max_chain_bytes,
        })
    }

    /// Validate already-parsed DER certificate lengths without retaining DER.
    pub fn validate_chain<I>(self, certificate_lengths: I) -> Result<(), SecurityRejection>
    where
        I: IntoIterator<Item = usize>,
    {
        let mut depth = 0_usize;
        let mut bytes = 0_usize;
        for length in certificate_lengths {
            depth = depth.saturating_add(1);
            if depth > self.max_chain_depth {
                return Err(SecurityRejection::CertificateChainDepth);
            }
            bytes = bytes
                .checked_add(length)
                .ok_or(SecurityRejection::CertificateChainSize)?;
            if bytes > self.max_chain_bytes {
                return Err(SecurityRejection::CertificateChainSize);
            }
        }
        if depth == 0 {
            return Err(SecurityRejection::MissingClientCertificate);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_depth_and_size_are_exact_and_fail_closed() {
        let policy = TlsPeerPolicy::new(2, 1_024).unwrap();
        assert_eq!(policy.validate_chain([512, 512]), Ok(()));
        assert_eq!(
            policy.validate_chain([341, 341, 341]),
            Err(SecurityRejection::CertificateChainDepth)
        );
        assert_eq!(
            policy.validate_chain([513, 512]),
            Err(SecurityRejection::CertificateChainSize)
        );
        assert_eq!(
            policy.validate_chain([]),
            Err(SecurityRejection::MissingClientCertificate)
        );
    }

    #[test]
    fn security_taxonomy_is_bounded_and_terminal() {
        let reasons = [
            SecurityRejection::UnknownCa,
            SecurityRejection::MissingClientCertificate,
            SecurityRejection::HostnameMismatch,
            SecurityRejection::CertificateExpired,
            SecurityRejection::CertificateNotYetValid,
            SecurityRejection::WrongClientCa,
            SecurityRejection::InvalidExtendedKeyUsage,
            SecurityRejection::CertificateChainDepth,
            SecurityRejection::CertificateChainSize,
            SecurityRejection::TlsProtocolPolicy,
            SecurityRejection::AuthorizationDenied,
        ];
        let labels = reasons
            .iter()
            .map(|reason| reason.label())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(labels.len(), reasons.len());
        assert!(reasons.iter().all(|reason| !matches!(
            reason.transport_failure(),
            TransportFailure::Availability | TransportFailure::AuthenticatedUnsupported
        )));
    }
}
