//! Retained installed contents under the original output owner, never start authority.
use super::*;
use crate::diagnostic_artifacts::linux::{inspect_fixed_install, BundleSnapshot};
use crate::diagnostic_artifacts::{ArtifactError, VerifiedBuild};
use crate::diagnostic_builder::CheckedBuilderPolicy;
use crate::diagnostic_lease::{DiagnosticIdentity, DiagnosticState};
use crate::diagnostic_unit::diagnostic_start_intent;
use crate::diagnostic_worker_policy::intent_projection::collector::collector_unit_projection;

#[derive(Debug, thiserror::Error)]
pub enum OriginalStartMaterialError {
    #[error("original outputs refused: {0}")]
    Outputs(#[from] OriginalSinkError),
    #[error("installed original build refused: {0}")]
    Build(#[from] ArtifactError),
    #[error("original collector requirements or build binding differs")]
    Binding,
    #[error("fixture origin cannot enter fixed installed preparation")]
    Fixture,
    #[error("original start material preparation already refused")]
    Refused,
}

fn same_build(
    expected: &VerifiedBuild,
    actual: &VerifiedBuild,
) -> Result<(), OriginalStartMaterialError> {
    equal_build(
        expected.identity(),
        expected.statement(),
        actual.identity(),
        actual.statement(),
    )
}
fn equal_build(
    expected_identity: &DiagnosticIdentity,
    expected: &crate::diagnostic_artifacts::BuildStatement,
    actual_identity: &DiagnosticIdentity,
    actual: &crate::diagnostic_artifacts::BuildStatement,
) -> Result<(), OriginalStartMaterialError> {
    if expected_identity != actual_identity || expected != actual {
        return Err(OriginalStartMaterialError::Binding);
    }
    Ok(())
}
fn requirements(
    state: &DiagnosticState,
    digest: &str,
    state_digest: &str,
) -> Result<Vec<u8>, OriginalStartMaterialError> {
    let raw = collector_unit_projection(state).map_err(|_| OriginalStartMaterialError::Binding)?;
    if crate::sha256_hex(&raw) != digest
        || crate::sha256_hex(
            &crate::canonical_json(state).map_err(|_| OriginalStartMaterialError::Binding)?,
        ) != state_digest
    {
        return Err(OriginalStartMaterialError::Binding);
    }
    Ok(raw)
}

/// Retains the same output/fence owner and installed bundle. No descriptors,
/// executable, unit transport, process, completion or recovery are exported.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::OriginalCollectorStartMaterial;
/// fn start(p: OriginalCollectorStartMaterial<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::OriginalCollectorStartMaterial;
/// fn clone(p: OriginalCollectorStartMaterial<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::OriginalCollectorStartMaterial;
/// fn extract(p: OriginalCollectorStartMaterial<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_bundle(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::OriginalCollectorStartMaterial;
/// fn stdio(p: OriginalCollectorStartMaterial<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_stdio(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::OriginalCollectorStartMaterial;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<OriginalCollectorStartMaterial<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
pub struct OriginalCollectorStartMaterial<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Bundle descriptors drop before outputs, which in turn close before the fence.
    bundle: Option<BundleSnapshot>,
    projection: Vec<u8>,
    starting: Option<DiagnosticState>,
    ready: bool,
    outputs: OriginalCollectorSinks<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
}
impl fmt::Debug for OriginalCollectorStartMaterial<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalCollectorStartMaterial")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl OriginalCollectorStartMaterial<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.outputs.is_refused()
    }
    fn bind(&mut self, allow_fixture: bool) -> Result<(), OriginalStartMaterialError> {
        let prepared = &self.outputs.pipes.starting.original.original.prepared;
        let r = &prepared.original.original.reservation;
        let body = prepared.consent_to_sign();
        if requirements(
            &r.state,
            &body.collector_unit_sha256,
            &body.reserved_state_sha256,
        )? != self.projection
            || self.starting.as_ref() != Some(&body.starting_state)
        {
            return Err(OriginalStartMaterialError::Binding);
        }
        let bundle = self
            .bundle
            .as_mut()
            .ok_or(OriginalStartMaterialError::Binding)?;
        if bundle.is_fixture() && !allow_fixture {
            return Err(OriginalStartMaterialError::Fixture);
        }
        same_build(&r.original.original.build, bundle.build())?;
        let intent = diagnostic_start_intent(&body.starting_state)
            .map_err(|_| OriginalStartMaterialError::Binding)?;
        bundle.build().bind_intent(&intent)?;
        bundle.revalidate()?;
        Ok(())
    }
    fn validate_with(&mut self, allow_fixture: bool) -> Result<(), OriginalStartMaterialError> {
        if !self.ready || self.is_refused() {
            self.outputs.refuse();
            return Err(OriginalStartMaterialError::Refused);
        }
        let result = (|| {
            self.outputs.material_round()?;
            self.bind(allow_fixture)?;
            self.outputs.material_round()?;
            Ok(())
        })();
        if result.is_err() {
            self.outputs.refuse();
        }
        result
    }
    pub fn revalidate(&mut self) -> Result<(), OriginalStartMaterialError> {
        self.validate_with(false)
    }
    #[cfg(test)]
    pub(crate) fn revalidate_fixture_for_test(&mut self) -> Result<(), OriginalStartMaterialError> {
        self.validate_with(true)
    }
    #[cfg(test)]
    pub(crate) fn corrupt_for_test(&mut self, kind: usize) {
        match kind {
            0 => self.projection.push(b' '),
            1 => self.starting.as_mut().unwrap().revision += 1,
            2 => self.outputs.corrupt_descriptor_for_test(0),
            _ => panic!("unknown start material mutation"),
        }
    }
}

pub struct OriginalStartMaterialFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: OriginalStartMaterialError,
    held: Box<OriginalCollectorStartMaterial<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl OriginalStartMaterialFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &OriginalStartMaterialError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.held.is_refused()
    }
}
impl fmt::Debug for OriginalStartMaterialFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for OriginalStartMaterialFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for OriginalStartMaterialFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {}

impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCollectorSinks<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    fn material_round(&mut self) -> Result<(), OriginalSinkError> {
        if self.is_refused() || !self.acknowledged {
            self.refuse();
            return Err(OriginalSinkError::Refused);
        }
        // A fresh original complete frame, not a cached closing result. The
        // ordinary public output validator remains the unchanged reference.
        let result = (|| {
            self.inspect(true)?;
            self.pipes.join_revalidate()?;
            self.inspect(true)
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
    /// Inspect only the fixed installed bundle under the original builder policy.
    /// No supplied path, FD, policy, time, outcome or execution transport is accepted.
    pub fn prepare_original_start_material(
        self,
    ) -> Result<
        OriginalCollectorStartMaterial<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalStartMaterialFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.prepare_material_with(false, |policy, identity| {
            inspect_fixed_install(identity, policy)
        })
    }
    #[cfg(test)]
    pub(crate) fn prepare_original_start_material_for_test(
        self,
        path: &std::path::Path,
        after_load: impl FnOnce(),
    ) -> Result<
        OriginalCollectorStartMaterial<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalStartMaterialFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        assert_ne!(unsafe { libc::geteuid() }, 0);
        assert_ne!(unsafe { libc::getegid() }, 0);
        self.prepare_material_with(true, |_, identity| {
            let bytes = include_bytes!(
                "../../../docs/testing/performance/0.74/diagnostic-builder-public-policy.json"
            );
            let policy: crate::diagnostic_builder::BuilderPolicy =
                serde_json::from_slice(bytes).unwrap();
            let trust = crate::diagnostic_artifacts::BuildTrust {
                key: crate::diagnostic_builder::decode_key(&policy.builder_key_hex).unwrap(),
                repository_id: policy.repository_id,
                builder_id: policy.builder_id,
            };
            let bundle = crate::diagnostic_artifacts::linux::inspect_fixture(
                path,
                unsafe { libc::geteuid() },
                unsafe { libc::getegid() },
                identity,
                &trust,
            )?;
            after_load();
            Ok(bundle)
        })
    }
    fn prepare_material_with(
        self,
        allow_fixture: bool,
        load: impl FnOnce(
            &CheckedBuilderPolicy,
            &DiagnosticIdentity,
        ) -> Result<BundleSnapshot, ArtifactError>,
    ) -> Result<
        OriginalCollectorStartMaterial<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalStartMaterialFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = OriginalCollectorStartMaterial {
            bundle: None,
            projection: Vec::new(),
            starting: None,
            ready: false,
            outputs: self,
        };
        let result = (|| {
            held.outputs.material_round()?;
            let prepared = &held.outputs.pipes.starting.original.original.prepared;
            let r = &prepared.original.original.reservation;
            let body = prepared.consent_to_sign();
            held.projection = requirements(
                &r.state,
                &body.collector_unit_sha256,
                &body.reserved_state_sha256,
            )?;
            held.starting = Some(body.starting_state.clone());
            let build = &r.original.original;
            held.bundle = Some(load(&build.policy, build.build.identity())?);
            held.bind(allow_fixture)?;
            held.outputs.material_round()?;
            held.ready = true;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(held),
            Err(error) => {
                held.outputs.refuse();
                Err(OriginalStartMaterialFailure {
                    error,
                    held: Box::new(held),
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_original_start_material_tests.rs"]
mod tests;
