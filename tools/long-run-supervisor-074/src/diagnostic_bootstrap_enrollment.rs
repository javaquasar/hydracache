//! Protected bootstrap selectors only, never original admission or execution.
use super::*;
pub const SCHEMA: &str = "diagnostic-bootstrap-enrollment-074-v1";
pub const FIXED_PATH: &str = "/etc/hydracache-perf/held-bootstrap-enrollment-074.json";
const FILE: &str = "held-bootstrap-enrollment-074.json";

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: String,
    repository_id: u64,
    bootstrap_builder_key_hex: String,
    bootstrap_policy_sha256: String,
    bootstrap_sha256: String,
    held_policy_sha256: String,
    policy_epoch: u64,
}
fn decode(raw: &[u8], held: &HeldSource) -> Result<Document, TrustInputError> {
    if raw.is_empty() || raw.len() > MAX_TRUST_BYTES {
        return Err(TrustInputError::Invalid);
    }
    let doc: Document = serde_json::from_slice(raw).map_err(|_| TrustInputError::Invalid)?;
    let mut canonical = canonical_json(&doc).map_err(|_| TrustInputError::Invalid)?;
    canonical.push(b'\n');
    let key = crate::diagnostic_builder::decode_key(&doc.bootstrap_builder_key_hex)
        .map_err(|_| TrustInputError::Invalid)?;
    let held_key = crate::diagnostic_builder::decode_key(&held.document.held_builder_key_hex)
        .map_err(|_| TrustInputError::Invalid)?;
    if canonical != raw
        || doc.schema_version != SCHEMA
        || doc.repository_id != crate::diagnostic_builder::REPOSITORY_ID
        || key.is_weak()
        || [
            held_key,
            held.selection.controller,
            held.selection.worker_builder,
            held.selection.worker_issuer,
        ]
        .contains(&key)
        || doc.held_policy_sha256 != held.document.held_policy_sha256
        || doc.policy_epoch == 0
        || doc.policy_epoch != held.document.policy_epoch
        || [
            &doc.bootstrap_policy_sha256,
            &doc.bootstrap_sha256,
            &doc.held_policy_sha256,
        ]
        .iter()
        .any(|digest| !is_hash(digest) || digest.as_bytes().iter().all(|b| *b == b'0'))
    {
        return Err(TrustInputError::Invalid);
    }
    Ok(doc)
}
fn snapshot(held: &HeldSource) -> Result<Snapshot, TrustInputError> {
    let mut named = root_file(&held.root_name)?;
    held.root.same(&named)?;
    if held.ancestors.len() != ANCESTORS.len() {
        return Err(TrustInputError::Invalid);
    }
    for (name, retained) in ANCESTORS.iter().zip(&held.ancestors) {
        named = child(&named, name, true)?;
        retained.same(&named)?;
    }
    Snapshot::new(child(&named, FILE, false)?, held.uid)
}
pub struct BootstrapEnrollment<'held> {
    held: &'held mut HeldTrustInput,
    retained: Snapshot,
    document: Document,
    refused: bool,
}
impl HeldTrustInput {
    /// Fixed independently protected selectors; no constructor from caller pins.
    pub fn bind_bootstrap_enrollment(&mut self) -> Result<BootstrapEnrollment<'_>, HeldTrustError> {
        let result = (|| {
            self.revalidate()?;
            let retained = snapshot(&self.source)?;
            let document = decode(&retained.raw, &self.source)?;
            self.revalidate()?;
            Ok((retained, document))
        })();
        let (retained, document) = match result {
            Ok(value) => value,
            Err(error) => {
                self.refused = true;
                return Err(error);
            }
        };
        let mut guard = BootstrapEnrollment {
            held: self,
            retained,
            document,
            refused: false,
        };
        guard.revalidate()?;
        Ok(guard)
    }
}
impl BootstrapEnrollment<'_> {
    pub fn is_refused(&self) -> bool {
        self.refused || self.held.is_refused()
    }
    pub fn revalidate(&mut self) -> Result<(), HeldTrustError> {
        let result = (|| {
            if self.is_refused() {
                return Err(HeldTrustError::Refused);
            }
            self.held.revalidate()?;
            self.retained.same(&snapshot(&self.held.source)?)?;
            if decode(&self.retained.raw, &self.held.source)? != self.document {
                return Err(TrustInputError::Invalid.into());
            }
            self.held.revalidate()?;
            Ok(())
        })();
        if result.is_err() {
            self.refused = true;
            self.held.refused = true;
        }
        result
    }
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_enrollment_tests.rs"]
mod tests;

#[path = "diagnostic_bootstrap_selected_build.rs"]
pub(crate) mod selected_build;
