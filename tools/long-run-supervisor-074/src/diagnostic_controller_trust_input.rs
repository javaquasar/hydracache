//! Fixed controller principals plus original signed request observation, not replay/start.
use super::*;
use crate::diagnostic_ipc::{DiagnosticPolicy, DiagnosticRequest};

#[path = "diagnostic_controller_build_input.rs"]
pub mod build_input;

pub const CONTROLLER_INPUT_SCHEMA: &str = "diagnostic-controller-trust-input-074-v1";
pub const FIXED_CONTROLLER_PATH: &str = "/etc/hydracache-perf/controller-trust-074.json";
const CONTROLLER_FILE: &str = "controller-trust-074.json";
const PRINCIPAL_LIMIT: usize = 32;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ControllerDocument {
    schema_version: String,
    repository_id: u64,
    controller_key_hex: String,
    actor_ids: Vec<u64>,
    client_uids: Vec<u32>,
    client_gid: u32,
}
fn decode_controller(
    raw: &[u8],
    selected_key: &VerifyingKey,
) -> Result<DiagnosticPolicy, TrustInputError> {
    if raw.is_empty() || raw.len() > MAX_TRUST_BYTES {
        return Err(TrustInputError::Invalid);
    }
    let d: ControllerDocument =
        serde_json::from_slice(raw).map_err(|_| TrustInputError::Invalid)?;
    let mut canonical = canonical_json(&d).map_err(|_| TrustInputError::Invalid)?;
    canonical.push(b'\n');
    if canonical != raw
        || d.schema_version != CONTROLLER_INPUT_SCHEMA
        || d.repository_id != crate::diagnostic_builder::REPOSITORY_ID
        || selected_key.is_weak()
        || crate::diagnostic_builder::decode_key(&d.controller_key_hex)
            .map_err(|_| TrustInputError::Invalid)?
            != *selected_key
        || d.actor_ids.is_empty()
        || d.actor_ids.len() > PRINCIPAL_LIMIT
        || d.actor_ids.contains(&0)
        || d.actor_ids.windows(2).any(|p| p[0] >= p[1])
        || d.client_uids.is_empty()
        || d.client_uids.len() > PRINCIPAL_LIMIT
        || d.client_uids.iter().any(|id| matches!(*id, 0 | u32::MAX))
        || d.client_uids.windows(2).any(|p| p[0] >= p[1])
        || matches!(d.client_gid, 0 | u32::MAX)
    {
        return Err(TrustInputError::Invalid);
    }
    Ok(DiagnosticPolicy {
        repository_id: d.repository_id,
        actor_ids: d.actor_ids,
        client_uids: d.client_uids,
        client_gid: d.client_gid,
        verifying_key: *selected_key,
    })
}
struct ControllerSource {
    root_name: PathBuf,
    root: Directory,
    ancestors: Vec<Directory>,
    snapshot: Snapshot,
    policy: DiagnosticPolicy,
    uid: u32,
}
impl ControllerSource {
    // Private model seam only; the public constructor fixes root, owner and reviewed key.
    fn open(root_name: &Path, uid: u32, key: &VerifyingKey) -> Result<Self, TrustInputError> {
        let root = Directory::new(
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(root_name)?,
            uid,
        )?;
        let mut ancestors: Vec<Directory> = Vec::with_capacity(2);
        for name in ANCESTORS {
            let parent = ancestors.last().map_or(&root.file, |d| &d.file);
            ancestors.push(Directory::new(child(parent, name, true)?, uid)?);
        }
        let parent = &ancestors.last().ok_or(TrustInputError::Invalid)?.file;
        let snapshot = Snapshot::new(child(parent, CONTROLLER_FILE, false)?, uid)?;
        let policy = decode_controller(&snapshot.raw, key)?;
        let source = Self {
            root_name: root_name.into(),
            root,
            ancestors,
            snapshot,
            policy,
            uid,
        };
        source.revalidate()?;
        Ok(source)
    }
    fn revalidate(&self) -> Result<(), TrustInputError> {
        let root = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&self.root_name)?;
        self.root.same(&root)?;
        let mut parent = &self.root.file;
        for (name, directory) in ANCESTORS.iter().zip(&self.ancestors) {
            directory.same(&child(parent, name, true)?)?;
            parent = &directory.file;
        }
        self.snapshot.same(&Snapshot::new(
            child(parent, CONTROLLER_FILE, false)?,
            self.uid,
        )?)
    }
}
/// Independent fixed principal source; no public model, path or trust extractor.
pub struct ControllerTrustInput {
    source: ControllerSource,
    refused: bool,
    _original_thread: PhantomData<Rc<()>>,
}
pub fn open_fixed_controller_trust() -> Result<ControllerTrustInput, TrustInputError> {
    if unsafe { libc::geteuid() } != 0 || unsafe { libc::getegid() } != 0 {
        return Err(TrustInputError::Invalid);
    }
    let key =
        crate::diagnostic_reviewed_trust::controller_key().map_err(|_| TrustInputError::Invalid)?;
    Ok(ControllerTrustInput {
        source: ControllerSource::open(Path::new("/"), 0, &key)?,
        refused: false,
        _original_thread: PhantomData,
    })
}
impl ControllerTrustInput {
    pub fn is_refused(&self) -> bool {
        self.refused
    }
    fn check(&self) -> Result<(), TrustInputError> {
        if self.refused {
            return Err(TrustInputError::Refused);
        }
        self.source.revalidate()
    }
    pub fn authenticate_controller<'c, 'b, 's, 'a, 'p>(
        &'c mut self,
        worker: &'b mut KernelWorkerPolicyInput<'s, 'a, 'p>,
    ) -> Result<SignedControllerInput<'c, 'b, 's, 'a, 'p>, TrustInputError> {
        let result = (|| {
            worker.revalidate()?;
            self.check()?;
            let request = worker.authenticate_configured_controller(&self.source.policy)?;
            self.check()?;
            worker.revalidate()?;
            let hash =
                crate::sha256_hex(&canonical_json(&request).map_err(|_| TrustInputError::Invalid)?);
            Ok((request, hash))
        })();
        match result {
            Ok((request, request_sha256)) => Ok(SignedControllerInput {
                controller: self,
                worker,
                request,
                request_sha256,
            }),
            Err(error) => {
                self.refused = true;
                worker.latch_refusal();
                Err(error)
            }
        }
    }
}
/// Signature/peer/freshness observation, never GitHub provenance or durable nonce consumption.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::SignedControllerInput;
/// fn start(mut g: SignedControllerInput<'_, '_, '_, '_, '_>) { g.start_worker(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::SignedControllerInput;
/// fn extract(g: SignedControllerInput<'_, '_, '_, '_, '_>) { g.into_request(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::SignedControllerInput;
/// fn replace(mut g: SignedControllerInput<'_, '_, '_, '_, '_>) { g.revalidate(b"replacement"); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::SignedControllerInput;
/// fn send(g: SignedControllerInput<'static, 'static, 'static, 'static, 'static>) { std::thread::spawn(move || drop(g)); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::{KernelWorkerPolicyInput, controller_input::ControllerTrustInput};
/// fn refresh(source: &mut ControllerTrustInput, worker: &mut KernelWorkerPolicyInput<'_, '_, '_>) {
///     let mut g = source.authenticate_controller(worker).unwrap();
///     worker.revalidate().unwrap();
///     g.revalidate().unwrap();
/// }
/// ```
pub struct SignedControllerInput<'c, 'b, 's, 'a, 'p> {
    controller: &'c mut ControllerTrustInput,
    worker: &'b mut KernelWorkerPolicyInput<'s, 'a, 'p>,
    request: DiagnosticRequest,
    request_sha256: String,
}
impl SignedControllerInput<'_, '_, '_, '_, '_> {
    fn latch_refusal(&mut self) {
        self.controller.refused = true;
        self.worker.latch_refusal();
    }
    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }
    pub fn is_refused(&self) -> bool {
        self.controller.refused || self.worker.is_refused()
    }
    pub fn revalidate(&mut self) -> Result<(), TrustInputError> {
        let result = (|| {
            if self.is_refused() {
                return Err(TrustInputError::Refused);
            }
            self.worker.revalidate()?;
            self.controller.check()?;
            if self
                .worker
                .authenticate_configured_controller(&self.controller.source.policy)?
                != self.request
            {
                return Err(TrustInputError::Invalid);
            }
            self.controller.check()?;
            self.worker.revalidate()
        })();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
}

#[cfg(test)]
#[path = "diagnostic_controller_trust_input_tests.rs"]
mod tests;
