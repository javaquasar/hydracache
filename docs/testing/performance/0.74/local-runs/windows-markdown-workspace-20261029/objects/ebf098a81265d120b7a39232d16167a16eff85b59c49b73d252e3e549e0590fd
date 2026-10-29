//! Separate protected job CLI inputs; never the installed supervisor's command route.
use super::*;
use crate::diagnostic_builder::linux::{read, write_new};
use std::path::Path;

pub fn prepare_files(
    request: &Path,
    policy: &Path,
    receipt: &Path,
) -> Result<PreparedDiagnosticRequest, ProducerError> {
    let request = read(request, MAX_PACKET_BYTES as u64, false).map_err(|_| ProducerError::File)?;
    let policy = read(
        policy,
        crate::diagnostic_builder::MAX_POLICY_BYTES as u64,
        false,
    )
    .map_err(|_| ProducerError::File)?;
    let receipt = read(
        receipt,
        crate::diagnostic_artifacts::MAX_RECEIPT_BYTES,
        false,
    )
    .map_err(|_| ProducerError::File)?;
    prepare_request(&request, &policy, &receipt)
}
pub fn sign_files(
    request: &Path,
    policy: &Path,
    receipt: &Path,
    key: &Path,
    output: &Path,
) -> Result<(), ProducerError> {
    let expected =
        crate::diagnostic_reviewed_trust::controller_key().map_err(|_| ProducerError::Key)?;
    sign_files_selected(request, policy, receipt, key, output, &expected)
}
fn sign_files_selected(
    request: &Path,
    policy: &Path,
    receipt: &Path,
    key: &Path,
    output: &Path,
    expected: &VerifyingKey,
) -> Result<(), ProducerError> {
    let prepared = prepare_files(request, policy, receipt)?;
    let audience = prepared.audience().to_owned();
    sign_prepared_files_selected(prepared, &audience, key, output, expected)
}

/// Signs the retained body without reopening public inputs. Audience is consistency, not OIDC.
/// Consumes preparation even on error; this is not durable nonce consumption.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_controller_producer::{PreparedDiagnosticRequest, linux::sign_prepared_files};
/// use std::path::Path;
/// fn reuse(p: PreparedDiagnosticRequest, a: &str, key: &Path, out: &Path) {
///     sign_prepared_files(p, a, key, out).unwrap();
///     sign_prepared_files(p, a, key, out).unwrap();
/// }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_controller_producer::{PreparedDiagnosticRequest, linux::sign_prepared_files};
/// use std::path::Path;
/// fn replace(p: PreparedDiagnosticRequest, request: &Path, key: &Path, out: &Path) {
///     sign_prepared_files(p, request, key, out).unwrap();
/// }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_controller_producer::PreparedDiagnosticRequest;
/// fn duplicate(p: PreparedDiagnosticRequest) { let _ = p.clone(); }
/// ```
pub fn sign_prepared_files(
    prepared: PreparedDiagnosticRequest,
    observed_audience: &str,
    key: &Path,
    output: &Path,
) -> Result<(), ProducerError> {
    let expected =
        crate::diagnostic_reviewed_trust::controller_key().map_err(|_| ProducerError::Key)?;
    sign_prepared_files_selected(prepared, observed_audience, key, output, &expected)
}
fn sign_prepared_files_selected(
    prepared: PreparedDiagnosticRequest,
    observed_audience: &str,
    key: &Path,
    output: &Path,
    expected: &VerifyingKey,
) -> Result<(), ProducerError> {
    prepared.check_audience(observed_audience)?;
    prepared.check_time(job_unix_seconds()?)?;
    let mut secret = read(key, 65, true).map_err(|_| ProducerError::File)?;
    let raw = secret.strip_suffix(b"\n").unwrap_or(&secret);
    let decoded = std::str::from_utf8(raw)
        .ok()
        .filter(|v| crate::is_hash(v))
        .map(|v| {
            let mut out = [0u8; 32];
            for (i, pair) in v.as_bytes().chunks_exact(2).enumerate() {
                let n = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
                out[i] = (n(pair[0]) << 4) | n(pair[1]);
            }
            out
        });
    secret.fill(0);
    let mut decoded = decoded.ok_or(ProducerError::Key)?;
    let key = SigningKey::from_bytes(&decoded);
    decoded.fill(0);
    let bytes = prepared.sign_job(&key, expected)?;
    write_new(output, &bytes).map_err(|_| ProducerError::File)
}
#[cfg(test)]
#[path = "diagnostic_controller_producer_linux_tests.rs"]
mod tests;
