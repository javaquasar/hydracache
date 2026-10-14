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
