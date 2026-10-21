//! Fixed independent held selection joined to the original controller/kernel authority.
//! Read-only early guard, not placement admission, enrollment, replay or start.
use super::*;
use crate::diagnostic_artifacts::ArtifactError;
use crate::diagnostic_builder::held_build::{self as held, role_binding};

#[path = "diagnostic_held_late_trust.rs"]
pub(crate) mod late_join;

pub const TRUST_SCHEMA: &str = "diagnostic-held-runtime-trust-074-v1";
pub const FIXED_TRUST_PATH: &str = "/etc/hydracache-perf/held-runtime-trust-074.json";
const TRUST_FILE: &str = "held-runtime-trust-074.json";

#[derive(Debug, thiserror::Error)]
pub enum HeldTrustError {
    #[error("held trust source or original context refused: {0}")]
    Input(#[from] TrustInputError),
    #[error("held policy refused: {0}")]
    Policy(#[from] ArtifactError),
    #[error("held binding refused: {0}")]
    Binding(#[from] role_binding::BindingError),
    #[error("held selection differs from the original context")]
    ContextMismatch,
    #[error("held source or original chain previously refused")]
    Refused,
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: String,
    repository_id: u64,
    controller_key_hex: String,
    held_builder_key_hex: String,
    held_policy_sha256: String,
    worker_builder_policy_sha256: String,
    worker_policy_sha256: String,
    policy_epoch: u64,
}
#[derive(Clone)]
struct Selection {
    controller: VerifyingKey,
    worker_builder: VerifyingKey,
    worker_issuer: VerifyingKey,
    worker_builder_policy_sha256: String,
}
impl Selection {
    fn reviewed() -> Result<Self, ArtifactError> {
        use crate::diagnostic_reviewed_trust as reviewed;
        Ok(Self {
            controller: reviewed::controller_key()?,
            worker_builder: crate::diagnostic_builder::decode_key(reviewed::BUILDER_KEY_HEX)?,
            worker_issuer: reviewed::worker_issuer_key()?,
            worker_builder_policy_sha256: reviewed::BUILDER_POLICY_SHA256.into(),
        })
    }
}
fn decode(raw: &[u8], selected: &Selection) -> Result<Document, TrustInputError> {
    if raw.is_empty() || raw.len() > MAX_TRUST_BYTES {
        return Err(TrustInputError::Invalid);
    }
    let doc: Document = serde_json::from_slice(raw).map_err(|_| TrustInputError::Invalid)?;
    let mut canonical = canonical_json(&doc).map_err(|_| TrustInputError::Invalid)?;
    canonical.push(b'\n');
    let key = crate::diagnostic_builder::decode_key(&doc.held_builder_key_hex)
        .map_err(|_| TrustInputError::Invalid)?;
    let controller = crate::diagnostic_builder::decode_key(&doc.controller_key_hex)
        .map_err(|_| TrustInputError::Invalid)?;
    if canonical != raw
        || doc.schema_version != TRUST_SCHEMA
        || doc.repository_id != crate::diagnostic_builder::REPOSITORY_ID
        || controller != selected.controller
        || [
            key,
            selected.controller,
            selected.worker_builder,
            selected.worker_issuer,
        ]
        .iter()
        .any(VerifyingKey::is_weak)
        || [
            selected.controller,
            selected.worker_builder,
            selected.worker_issuer,
        ]
        .contains(&key)
        || selected.controller == selected.worker_builder
        || selected.controller == selected.worker_issuer
        || selected.worker_builder == selected.worker_issuer
        || !is_hash(&doc.held_policy_sha256)
        || !is_hash(&doc.worker_policy_sha256)
        || !is_hash(&doc.worker_builder_policy_sha256)
        || doc.worker_builder_policy_sha256 != selected.worker_builder_policy_sha256
        || doc.policy_epoch == 0
    {
        return Err(TrustInputError::Invalid);
    }
    Ok(doc)
}
struct HeldSource {
    root_name: PathBuf,
    root: Directory,
    ancestors: Vec<Directory>,
    snapshot: Snapshot,
    document: Document,
    selection: Selection,
    uid: u32,
}
fn root_file(path: &Path) -> Result<File, TrustInputError> {
    if !path.is_absolute() {
        return Err(TrustInputError::Invalid);
    }
    Ok(OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?)
}
impl HeldSource {
    // Only the public opener selects / and uid 0. Modeled roots remain private.
    fn open(path: &Path, uid: u32, selection: Selection) -> Result<Self, TrustInputError> {
        let root = Directory::new(root_file(path)?, uid)?;
        let mut parent = &root.file;
        let mut ancestors = Vec::with_capacity(ANCESTORS.len());
        for name in ANCESTORS {
            ancestors.push(Directory::new(child(parent, name, true)?, uid)?);
            parent = &ancestors.last().ok_or(TrustInputError::Invalid)?.file;
        }
        let snapshot = Snapshot::new(child(parent, TRUST_FILE, false)?, uid)?;
        let document = decode(&snapshot.raw, &selection)?;
        let source = Self {
            root_name: path.into(),
            root,
            ancestors,
            snapshot,
            document,
            selection,
            uid,
        };
        source.revalidate()?;
        Ok(source)
    }
    fn revalidate(&self) -> Result<(), TrustInputError> {
        let mut named = root_file(&self.root_name)?;
        self.root.same(&named)?;
        if self.ancestors.len() != ANCESTORS.len() {
            return Err(TrustInputError::Invalid);
        }
        for (name, held) in ANCESTORS.iter().zip(&self.ancestors) {
            named = child(&named, name, true)?;
            held.same(&named)?;
        }
        self.snapshot
            .same(&Snapshot::new(child(&named, TRUST_FILE, false)?, self.uid)?)?;
        if decode(&self.snapshot.raw, &self.selection)? != self.document {
            return Err(TrustInputError::Invalid);
        }
        Ok(())
    }
}
/// Retained root-selected input. No path override, writer, key generation or start.
/// ```compile_fail,E0599
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::held_runtime::HeldTrustInput;
/// fn no_clone(value: HeldTrustInput) {
///     value.clone();
/// }
/// ```
/// ```compile_fail,E0277
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::held_runtime::HeldTrustInput;
/// fn no_send(value: HeldTrustInput) {
///     std::thread::spawn(move || drop(value));
/// }
/// ```
/// ```compile_fail,E0277
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::held_runtime::HeldTrustInput;
/// fn requires_decode<T: serde::de::DeserializeOwned>() {}
/// requires_decode::<HeldTrustInput>();
/// ```
pub struct HeldTrustInput {
    source: HeldSource,
    refused: bool,
    _original_thread: PhantomData<Rc<()>>,
}
/// Sole production opener. The fixed root configuration supplies held key/pin;
/// no incoming policy/binding or compiled held default may select them.
pub fn open_fixed_held_runtime_trust() -> Result<HeldTrustInput, HeldTrustError> {
    if unsafe { libc::geteuid() } != 0 || unsafe { libc::getegid() } != 0 {
        return Err(TrustInputError::Invalid.into());
    }
    Ok(HeldTrustInput {
        source: HeldSource::open(Path::new("/"), 0, Selection::reviewed()?)?,
        refused: false,
        _original_thread: PhantomData,
    })
}
impl HeldTrustInput {
    pub fn is_refused(&self) -> bool {
        self.refused
    }
    pub fn revalidate(&mut self) -> Result<(), HeldTrustError> {
        if self.refused {
            return Err(HeldTrustError::Refused);
        }
        self.source.revalidate().map_err(|error| {
            self.refused = true;
            error.into()
        })
    }
    pub fn bind_original<'h, 'q, 'c, 'b, 's, 'a, 'p>(
        &'h mut self,
        original: &'q mut SignedControllerInput<'c, 'b, 's, 'a, 'p>,
        held_policy: &[u8],
        worker_builder_policy: &[u8],
    ) -> Result<OriginalHeldTrust<'h, 'q, 'c, 'b, 's, 'a, 'p>, HeldTrustError> {
        let result = (|| {
            self.revalidate()?;
            original.revalidate()?;
            if held_policy.is_empty()
                || worker_builder_policy.is_empty()
                || held_policy.len() > crate::diagnostic_builder::MAX_POLICY_BYTES
                || worker_builder_policy.len() > crate::diagnostic_builder::MAX_POLICY_BYTES
            {
                return Err(HeldTrustError::ContextMismatch);
            }
            Observed::capture(original)
        })();
        let observed = match result {
            Ok(value) => value,
            Err(error) => {
                self.refused = true;
                original.latch_refusal();
                return Err(error);
            }
        };
        let mut guard = OriginalHeldTrust {
            source: self,
            original,
            observed,
            held_policy: held_policy.to_vec(),
            worker_policy: worker_builder_policy.to_vec(),
        };
        guard.revalidate()?;
        Ok(guard)
    }
}
#[derive(PartialEq, Eq)]
struct Observed {
    controller: VerifyingKey,
    request_sha256: String,
    authority: (String, u64, u64, String),
}
impl Observed {
    fn capture(
        original: &mut SignedControllerInput<'_, '_, '_, '_, '_>,
    ) -> Result<Self, HeldTrustError> {
        Ok(Self {
            controller: original.controller.source.policy.verifying_key,
            request_sha256: original.request_sha256.clone(),
            authority: original.worker.controller_authority_binding()?,
        })
    }
}
/// Early original-controller borrow, not the later consuming placement/full-frame join.
/// Every operation freshly validates both retained sources and the original chain.
/// ```compile_fail,E0599
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::held_runtime::OriginalHeldTrust;
/// fn no_start(value: OriginalHeldTrust<'_, '_, '_, '_, '_, '_, '_>) {
///     value.start();
/// }
/// ```
/// ```compile_fail,E0616
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::held_runtime::OriginalHeldTrust;
/// fn no_extract(value: OriginalHeldTrust<'_, '_, '_, '_, '_, '_, '_>) {
///     let _ = value.original;
/// }
/// ```
pub struct OriginalHeldTrust<'h, 'q, 'c, 'b, 's, 'a, 'p> {
    source: &'h mut HeldTrustInput,
    original: &'q mut SignedControllerInput<'c, 'b, 's, 'a, 'p>,
    observed: Observed,
    held_policy: Vec<u8>,
    worker_policy: Vec<u8>,
}
impl OriginalHeldTrust<'_, '_, '_, '_, '_, '_, '_> {
    fn poison(&mut self) {
        self.source.refused = true;
        self.original.latch_refusal();
    }
    pub fn is_refused(&self) -> bool {
        self.source.is_refused() || self.original.is_refused()
    }
    fn checked_policy(&mut self) -> Result<held::CheckedPolicy, HeldTrustError> {
        let result = (|| {
            self.source.revalidate()?;
            self.original.revalidate()?;
            if Observed::capture(self.original)? != self.observed {
                return Err(HeldTrustError::ContextMismatch);
            }
            let doc = &self.source.source.document;
            if doc.controller_key_hex != crate::hex(self.observed.controller.as_bytes())
                || doc.worker_policy_sha256 != self.observed.authority.0
                || doc.policy_epoch != self.observed.authority.1
            {
                return Err(HeldTrustError::ContextMismatch);
            }
            let policy = held::load_policy(
                &self.held_policy,
                &doc.held_policy_sha256,
                &held::ExpectedTrust {
                    controller_key: &self.observed.controller,
                    authority_epoch: self.observed.authority.1,
                    worker_policy: &self.worker_policy,
                    worker_policy_sha256: &doc.worker_builder_policy_sha256,
                },
            )?;
            let held_doc: held::Policy = serde_json::from_slice(&self.held_policy)
                .map_err(|_| HeldTrustError::ContextMismatch)?;
            let worker_doc: crate::diagnostic_builder::BuilderPolicy =
                serde_json::from_slice(&self.worker_policy)
                    .map_err(|_| HeldTrustError::ContextMismatch)?;
            if held_doc.builder_key_hex != doc.held_builder_key_hex
                || crate::diagnostic_builder::decode_key(&worker_doc.builder_key_hex)?
                    != self.source.source.selection.worker_builder
            {
                return Err(HeldTrustError::ContextMismatch);
            }
            self.original.revalidate()?;
            if Observed::capture(self.original)? != self.observed {
                return Err(HeldTrustError::ContextMismatch);
            }
            self.source.revalidate()?;
            Ok(policy)
        })();
        if result.is_err() {
            self.poison();
        }
        result
    }
    pub fn revalidate(&mut self) -> Result<(), HeldTrustError> {
        self.checked_policy().map(|_| ())
    }
    /// Static v4 inspection additionally matched to THIS original signed request.
    /// The returned static Inspection does not grant late placement/FD admission.
    pub fn inspect_request_binding(
        &mut self,
        raw: &[u8],
        receipt: &[u8],
    ) -> Result<role_binding::Inspection, HeldTrustError> {
        let result = (|| {
            let policy = self.checked_policy()?;
            let inspection = role_binding::inspect_signature(raw, &policy, receipt)?;
            let doc: role_binding::SignedBinding =
                serde_json::from_slice(raw).map_err(|_| HeldTrustError::ContextMismatch)?;
            let placement = &doc.binding.placement.placement;
            let consent = &placement.launch_claims.consent_claims;
            let request = &self.original.request;
            if consent.reservation_request_id != request.request_id
                || consent.worker_policy_sha256 != self.observed.authority.0
                || consent.worker_policy_epoch != self.observed.authority.1
                || consent.controller != request.controller
                || consent.source_commit != request.source_commit
                || consent.preset != request.preset
                || consent.starting_state.identity != request.identity
            {
                return Err(HeldTrustError::ContextMismatch);
            }
            self.checked_policy()?;
            Ok(inspection)
        })();
        if result.is_err() {
            self.poison();
        }
        result
    }
}

#[cfg(test)]
#[path = "../tests/support/held_runtime_trust.rs"]
mod tests;
