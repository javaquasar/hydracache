//! Versioned process-fixture receipts shared by HC/2 interoperability tests.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Versioned discriminator for the production-daemon readiness receipt.
pub const PRODUCTION_DAEMON_READY_V1: &str = "READY_DAEMON_V1";

/// Parsed readiness contract emitted by the HC/2 production-daemon fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionDaemonReadyReceipt {
    pub hc2_port: u16,
    pub admin_port: u16,
    pub ca: PathBuf,
    pub first_client_certificate: PathBuf,
    pub first_client_key: PathBuf,
    pub second_client_certificate: PathBuf,
    pub second_client_key: PathBuf,
}

impl ProductionDaemonReadyReceipt {
    /// Construct a receipt after validating that every path is safe for the
    /// line-oriented, tab-delimited fixture protocol.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        hc2_port: u16,
        admin_port: u16,
        ca: impl Into<PathBuf>,
        first_client_certificate: impl Into<PathBuf>,
        first_client_key: impl Into<PathBuf>,
        second_client_certificate: impl Into<PathBuf>,
        second_client_key: impl Into<PathBuf>,
    ) -> Result<Self, FixtureReceiptError> {
        if hc2_port == 0 || admin_port == 0 || hc2_port == admin_port {
            return Err(FixtureReceiptError::InvalidPortSet);
        }
        let receipt = Self {
            hc2_port,
            admin_port,
            ca: ca.into(),
            first_client_certificate: first_client_certificate.into(),
            first_client_key: first_client_key.into(),
            second_client_certificate: second_client_certificate.into(),
            second_client_key: second_client_key.into(),
        };
        for path in receipt.paths() {
            validate_path(path)?;
        }
        Ok(receipt)
    }

    /// Encode the exact V1 wire shape.
    pub fn encode(&self) -> String {
        format!(
            "{PRODUCTION_DAEMON_READY_V1}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.hc2_port,
            self.admin_port,
            self.ca.display(),
            self.first_client_certificate.display(),
            self.first_client_key.display(),
            self.second_client_certificate.display(),
            self.second_client_key.display()
        )
    }

    /// Parse only the exact V1 shape. Unknown versions and extension fields
    /// fail loudly so every consumer is updated with the producer.
    pub fn parse(line: &str) -> Result<Self, FixtureReceiptError> {
        let fields = line
            .trim_end_matches(['\r', '\n'])
            .split('\t')
            .collect::<Vec<_>>();
        if fields.first().copied() != Some(PRODUCTION_DAEMON_READY_V1) {
            return Err(FixtureReceiptError::UnsupportedVersion(
                fields.first().copied().unwrap_or_default().to_owned(),
            ));
        }
        if fields.len() != 8 {
            return Err(FixtureReceiptError::InvalidFieldCount(fields.len()));
        }
        let hc2_port = fields[1]
            .parse::<u16>()
            .map_err(|_| FixtureReceiptError::InvalidPort("hc2"))?;
        let admin_port = fields[2]
            .parse::<u16>()
            .map_err(|_| FixtureReceiptError::InvalidPort("admin"))?;
        Self::new(
            hc2_port, admin_port, fields[3], fields[4], fields[5], fields[6], fields[7],
        )
    }

    fn paths(&self) -> [&Path; 5] {
        [
            &self.ca,
            &self.first_client_certificate,
            &self.first_client_key,
            &self.second_client_certificate,
            &self.second_client_key,
        ]
    }
}

fn validate_path(path: &Path) -> Result<(), FixtureReceiptError> {
    let value = path.to_string_lossy();
    if value.is_empty() || value.contains(['\t', '\r', '\n']) {
        Err(FixtureReceiptError::InvalidPath)
    } else {
        Ok(())
    }
}

/// Fail-loud receipt parsing errors used by process harnesses.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FixtureReceiptError {
    #[error("unsupported production daemon readiness version {0:?}")]
    UnsupportedVersion(String),
    #[error("production daemon readiness has {0} fields; expected 8")]
    InvalidFieldCount(usize),
    #[error("production daemon readiness contains an invalid {0} port")]
    InvalidPort(&'static str),
    #[error("production daemon readiness ports must be distinct and nonzero")]
    InvalidPortSet,
    #[error("production daemon readiness paths must be non-empty single-line values")]
    InvalidPath,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt() -> ProductionDaemonReadyReceipt {
        ProductionDaemonReadyReceipt::new(
            41001,
            41002,
            "/tmp/ca.pem",
            "/tmp/client.pem",
            "/tmp/client.key",
            "/tmp/client-second.pem",
            "/tmp/client-second.key",
        )
        .unwrap()
    }

    #[test]
    fn production_daemon_v1_receipt_round_trips_exactly() {
        let receipt = receipt();
        assert_eq!(
            ProductionDaemonReadyReceipt::parse(&receipt.encode()).unwrap(),
            receipt
        );
    }

    #[test]
    fn production_daemon_receipt_rejects_legacy_and_extended_shapes() {
        let encoded = receipt().encode();
        assert!(matches!(
            ProductionDaemonReadyReceipt::parse(&encoded.replacen(
                PRODUCTION_DAEMON_READY_V1,
                "READY_DAEMON",
                1
            )),
            Err(FixtureReceiptError::UnsupportedVersion(_))
        ));
        assert_eq!(
            ProductionDaemonReadyReceipt::parse(&format!("{encoded}\textra")),
            Err(FixtureReceiptError::InvalidFieldCount(9))
        );
    }
}
