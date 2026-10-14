//! Original configured OIDC signature observation; actual GitHub key enrollment remains separate.
use super::super::super::{child, Directory, Snapshot, TrustInputError, ANCESTORS};
use super::{ControllerBuildError, ControllerBuildInput};
use std::fs::OpenOptions;
use std::marker::PhantomData;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[path = "diagnostic_oidc_profile.rs"]
mod profile;
use profile::{verify, TrustProfile, MAX_ROSTER_BYTES, MAX_TOKEN};

pub const FIXED_OIDC_PATH: &str = "/etc/hydracache-perf/oidc-trust-074.json";
const FILE: &str = "oidc-trust-074.json";
#[derive(Debug, thiserror::Error)]
pub enum OidcError {
    #[error("OIDC profile, token, origin or freshness refused")]
    Invalid,
    #[error("OIDC signature refused")]
    Signature,
    #[error("original OIDC input previously refused")]
    Refused,
    #[error("original OIDC trust source refused: {0}")]
    Source(#[from] TrustInputError),
    #[error("original controller/build input refused: {0}")]
    Original(#[from] ControllerBuildError),
}
struct Source {
    root_name: PathBuf,
    root: Directory,
    ancestors: Vec<Directory>,
    snapshot: Snapshot,
    profile: TrustProfile,
    uid: u32,
}
impl Source {
    fn open(root_name: &Path, uid: u32) -> Result<Self, OidcError> {
        let root = Directory::new(
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(root_name)
                .map_err(TrustInputError::from)?,
            uid,
        )?;
        let mut ancestors: Vec<Directory> = Vec::new();
        for name in ANCESTORS {
            let parent = ancestors.last().map_or(&root.file, |d| &d.file);
            ancestors.push(Directory::new(child(parent, name, true)?, uid)?);
        }
        let snapshot = Snapshot::new_with_limit(
            child(
                &ancestors.last().ok_or(OidcError::Invalid)?.file,
                FILE,
                false,
            )?,
            uid,
            MAX_ROSTER_BYTES,
        )?;
        let profile = TrustProfile::decode(&snapshot.raw)?;
        let source = Self {
            root_name: root_name.into(),
            root,
            ancestors,
            snapshot,
            profile,
            uid,
        };
        source.check()?;
        Ok(source)
    }
    fn check(&self) -> Result<(), OidcError> {
        let named_root = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&self.root_name)
            .map_err(TrustInputError::from)?;
        self.root.same(&named_root)?;
        let mut parent = &self.root.file;
        for (name, directory) in ANCESTORS.iter().zip(&self.ancestors) {
            directory.same(&child(parent, name, true)?)?;
            parent = &directory.file;
        }
        self.snapshot.same(&Snapshot::new_with_limit(
            child(parent, FILE, false)?,
            self.uid,
            MAX_ROSTER_BYTES,
        )?)?;
        Ok(())
    }
}
/// Original fixed public configuration, not independently authenticated key acquisition.
pub struct OidcTrustInput {
    source: Source,
    refused: bool,
    _original_thread: PhantomData<Rc<()>>,
}
pub fn open_fixed_oidc_trust() -> Result<OidcTrustInput, OidcError> {
    if unsafe { libc::geteuid() } != 0 || unsafe { libc::getegid() } != 0 {
        return Err(OidcError::Invalid);
    }
    Ok(OidcTrustInput {
        source: Source::open(Path::new("/"), 0)?,
        refused: false,
        _original_thread: PhantomData,
    })
}
impl OidcTrustInput {
    pub fn is_refused(&self) -> bool {
        self.refused
    }
    fn check(&self) -> Result<(), OidcError> {
        if self.refused {
            return Err(OidcError::Refused);
        }
        self.source.check()
    }
}
impl<'r, 'c, 'b, 's, 'a, 'p> ControllerBuildInput<'r, 'c, 'b, 's, 'a, 'p> {
    pub fn bind_oidc<'o, 't>(
        &'o mut self,
        trust: &'t mut OidcTrustInput,
        token: &[u8],
    ) -> Result<OidcControllerInput<'o, 't, 'r, 'c, 'b, 's, 'a, 'p>, OidcError> {
        let result = (|| {
            self.revalidate()?;
            trust.check()?;
            let now = self.original.worker.original_unix_seconds()?;
            let digest = verify(
                &trust.source.profile,
                token,
                &self.original.request,
                &self.original.request_sha256,
                now,
            )?;
            trust.check()?;
            self.revalidate()?;
            Ok::<_, OidcError>(digest)
        })();
        match result {
            Ok(token_digest) => {
                // Token budget is checked by verification before copying.
                debug_assert!(token.len() <= MAX_TOKEN);
                let mut guard = OidcControllerInput {
                    original: self,
                    trust,
                    token: token.to_vec(),
                    token_digest,
                };
                guard.revalidate()?;
                Ok(guard)
            }
            Err(error) => {
                trust.refused = true;
                self.original.latch_refusal();
                Err(error)
            }
        }
    }
}
/// Configured origin evidence only. No actual issuer enrollment, nonce consumption or start.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::OidcControllerInput;
/// fn start(mut g: OidcControllerInput<'_, '_, '_, '_, '_, '_, '_, '_>) { g.start_worker(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::OidcControllerInput;
/// fn extract(g: OidcControllerInput<'_, '_, '_, '_, '_, '_, '_, '_>) { g.into_claims(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::OidcControllerInput;
/// fn replace(mut g: OidcControllerInput<'_, '_, '_, '_, '_, '_, '_, '_>) { g.revalidate(b"new"); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::OidcControllerInput;
/// fn send(g: OidcControllerInput<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>) { std::thread::spawn(move || drop(g)); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::{ControllerBuildInput, oidc_input::OidcTrustInput};
/// fn refresh(original: &mut ControllerBuildInput<'_, '_, '_, '_, '_, '_>, trust: &mut OidcTrustInput, token: &[u8]) {
///     let mut g = original.bind_oidc(trust, token).unwrap();
///     original.revalidate().unwrap();
///     g.revalidate().unwrap();
/// }
/// ```
pub struct OidcControllerInput<'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    original: &'o mut ControllerBuildInput<'r, 'c, 'b, 's, 'a, 'p>,
    trust: &'t mut OidcTrustInput,
    token: Vec<u8>,
    token_digest: String,
}
impl OidcControllerInput<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.trust.refused || self.original.is_refused()
    }
    pub fn revalidate(&mut self) -> Result<(), OidcError> {
        let result = (|| {
            if self.is_refused() {
                return Err(OidcError::Refused);
            }
            self.original.revalidate()?;
            self.trust.check()?;
            let request = &self.original.original.request;
            let hash = &self.original.original.request_sha256;
            let now = self.original.original.worker.original_unix_seconds()?;
            if verify(&self.trust.source.profile, &self.token, request, hash, now)?
                != self.token_digest
            {
                return Err(OidcError::Invalid);
            }
            self.trust.check()?;
            self.original.revalidate()?;
            Ok(())
        })();
        if result.is_err() {
            self.trust.refused = true;
            self.original.original.latch_refusal();
        }
        result
    }
}
#[cfg(test)]
#[path = "diagnostic_oidc_input_tests.rs"]
mod tests;
