//! Protected build selection retained across rounds on the same trust input.
use super::*;
use crate::diagnostic_builder::bootstrap_build as build;

pub(crate) struct SelectionReceipt {
    retained: Snapshot,
    document: Document,
    documents: [String; 2],
    pub(crate) build: build::VerifiedBuild,
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_selected_build_tests.rs"]
mod tests;
impl HeldTrustInput {
    pub(crate) fn inspect_bootstrap_build(
        &mut self,
        policy: &[u8],
        receipt: &[u8],
        original: [&str; 4],
        retained: Option<&SelectionReceipt>,
    ) -> Result<SelectionReceipt, HeldTrustError> {
        let mut guard = self.bind_bootstrap_enrollment()?;
        let result = (|| {
            guard.revalidate()?;
            if policy.is_empty()
                || policy.len() > crate::diagnostic_builder::MAX_POLICY_BYTES
                || receipt.is_empty()
                || receipt.len() as u64 > crate::diagnostic_artifacts::MAX_RECEIPT_BYTES
            {
                return Err(HeldTrustError::ContextMismatch);
            }
            let documents = [crate::sha256_hex(policy), crate::sha256_hex(receipt)];
            if let Some(old) = retained {
                old.retained.same(&guard.retained)?;
                if old.document != guard.document || old.documents != documents {
                    return Err(HeldTrustError::ContextMismatch);
                }
            }
            let doc = &guard.document;
            let key = crate::diagnostic_builder::decode_key(&doc.bootstrap_builder_key_hex)?;
            let policy = build::load_policy(
                policy,
                &doc.bootstrap_policy_sha256,
                &build::ExpectedTrust {
                    builder: &key,
                    controller: &guard.held.source.selection.controller,
                    epoch: doc.policy_epoch,
                    held_policy_sha256: &doc.held_policy_sha256,
                    bootstrap_sha256: &doc.bootstrap_sha256,
                },
            )?;
            let build = build::verify_receipt(receipt, &policy)?;
            build.matches_original(original)?;
            guard.revalidate()?;
            Ok((documents, build))
        })();
        match result {
            Ok((documents, build)) => Ok(SelectionReceipt {
                retained: guard.retained,
                document: guard.document,
                documents,
                build,
            }),
            Err(error) => {
                guard.refused = true;
                guard.held.refused = true;
                Err(error)
            }
        }
    }
}
