//! Separate job-side request preparation/signing, never receiver admission or replay.
use crate::diagnostic_artifacts::VerifiedBuild;
use crate::diagnostic_ipc::{self, DiagnosticRequest, SignedDiagnosticRequest, MAX_PACKET_BYTES};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "linux")]
#[path = "diagnostic_controller_producer_linux.rs"]
pub mod linux;

#[derive(Debug, thiserror::Error)]
pub enum ProducerError {
    #[error("diagnostic producer request or builder binding refused")]
    Invalid,
    #[error("diagnostic producer selected controller key refused")]
    Key,
    #[error("diagnostic producer job clock/window refused")]
    Time,
    #[error("diagnostic producer file boundary refused")]
    File,
}

/// Job-side bytes only; cannot become any original receiver/admission/start guard.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_controller_producer::PreparedDiagnosticRequest;
/// fn start(mut p: PreparedDiagnosticRequest) { p.start_worker(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_controller_producer::PreparedDiagnosticRequest;
/// fn extract(p: PreparedDiagnosticRequest) { p.into_verified_build(); }
/// ```
pub struct PreparedDiagnosticRequest {
    request: DiagnosticRequest,
    build: VerifiedBuild,
    request_sha256: String,
    audience: String,
}

pub fn prepare_request(
    request_bytes: &[u8],
    builder_policy: &[u8],
    receipt: &[u8],
) -> Result<PreparedDiagnosticRequest, ProducerError> {
    if request_bytes.is_empty() || request_bytes.len() > MAX_PACKET_BYTES {
        return Err(ProducerError::Invalid);
    }
    let request: DiagnosticRequest =
        serde_json::from_slice(request_bytes).map_err(|_| ProducerError::Invalid)?;
    let mut canonical = crate::canonical_json(&request).map_err(|_| ProducerError::Invalid)?;
    let request_sha256 = crate::sha256_hex(&canonical);
    canonical.push(b'\n');
    if canonical != request_bytes
        || request.controller.repository_id != crate::diagnostic_builder::REPOSITORY_ID
    {
        return Err(ProducerError::Invalid);
    }
    diagnostic_ipc::signing_message(&request).map_err(|_| ProducerError::Invalid)?;
    let policy = crate::diagnostic_reviewed_trust::load_builder_policy(builder_policy)
        .map_err(|_| ProducerError::Invalid)?;
    let build = policy
        .verify_receipt(receipt, &request.identity)
        .map_err(|_| ProducerError::Invalid)?;
    let audience = format!("urn:hydracache:diagnostic:074:request:{request_sha256}");
    Ok(PreparedDiagnosticRequest {
        request,
        build,
        request_sha256,
        audience,
    })
}
fn job_unix_seconds() -> Result<u64, ProducerError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|t| t.as_secs())
        .map_err(|_| ProducerError::Time)
}
impl PreparedDiagnosticRequest {
    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }
    pub fn audience(&self) -> &str {
        &self.audience
    }
    // This compares retained job bytes only, never token or bootstrap authority.
    fn check_audience(&self, observed_audience: &str) -> Result<(), ProducerError> {
        let hash = crate::sha256_hex(
            &crate::canonical_json(&self.request).map_err(|_| ProducerError::Invalid)?,
        );
        if hash != self.request_sha256
            || self.build.identity() != &self.request.identity
            || self.audience != format!("urn:hydracache:diagnostic:074:request:{hash}")
            || observed_audience != self.audience
        {
            return Err(ProducerError::Invalid);
        }
        Ok(())
    }
    fn check_time(&self, now: u64) -> Result<(), ProducerError> {
        if now < self.request.issued_at_unix_seconds || now >= self.request.expires_at_unix_seconds
        {
            return Err(ProducerError::Time);
        }
        Ok(())
    }
    /// Consumes local preparation, not a nonce or host authority. No caller trust/time override.
    pub fn sign_with_selected_controller(self, key: &SigningKey) -> Result<Vec<u8>, ProducerError> {
        let expected =
            crate::diagnostic_reviewed_trust::controller_key().map_err(|_| ProducerError::Key)?;
        self.sign_job(key, &expected)
    }
    fn sign_job(self, key: &SigningKey, expected: &VerifyingKey) -> Result<Vec<u8>, ProducerError> {
        let now = job_unix_seconds()?;
        let bytes = self.sign_checked(key, expected, now)?;
        let signed: SignedDiagnosticRequest =
            serde_json::from_slice(&bytes).map_err(|_| ProducerError::Invalid)?;
        let after = job_unix_seconds()?;
        if after < now || after >= signed.request.expires_at_unix_seconds {
            return Err(ProducerError::Time);
        }
        Ok(bytes)
    }
    // Private synthetic model seam. Production selects only the reviewed controller.
    fn sign_checked(
        self,
        key: &SigningKey,
        expected: &VerifyingKey,
        now: u64,
    ) -> Result<Vec<u8>, ProducerError> {
        self.check_time(now)?;
        let message =
            diagnostic_ipc::signing_message(&self.request).map_err(|_| ProducerError::Invalid)?;
        self.check_audience(&self.audience)?;
        if expected.is_weak() || key.verifying_key() != *expected {
            return Err(ProducerError::Key);
        }
        let signature = key.sign(&message);
        expected
            .verify_strict(&message, &signature)
            .map_err(|_| ProducerError::Key)?;
        let mut bytes = crate::canonical_json(&SignedDiagnosticRequest {
            request: self.request,
            signature_hex: crate::hex(&signature.to_bytes()),
        })
        .map_err(|_| ProducerError::Invalid)?;
        bytes.push(b'\n');
        if bytes.len() > MAX_PACKET_BYTES {
            return Err(ProducerError::Invalid);
        }
        Ok(bytes)
    }
}
#[cfg(test)]
#[path = "diagnostic_controller_producer_tests.rs"]
mod tests;
