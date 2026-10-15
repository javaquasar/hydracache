//! Read-only original context join; no principal, replay, start or completion authority.
use super::*;
use crate::diagnostic_kernel_admission::KernelPacket;
use crate::diagnostic_worker_policy::{
    local_context::Probe, SignedWorkerPolicy, MAX_SIGNED_WORKER_POLICY_BYTES,
};

fn candidate_context(bytes: &[u8]) -> Result<AssertedWorkerHost, TrustInputError> {
    if bytes.is_empty() || bytes.len() > MAX_SIGNED_WORKER_POLICY_BYTES {
        return Err(TrustInputError::Invalid);
    }
    let signed: SignedWorkerPolicy =
        serde_json::from_slice(bytes).map_err(|_| TrustInputError::Invalid)?;
    Ok(AssertedWorkerHost {
        machine_id: signed.policy.machine_id,
        boot_id: signed.policy.boot_id,
        user_namespace: signed.policy.user_namespace,
        mount_namespace: signed.policy.mount_namespace,
    })
}
impl WorkerTrustInput {
    /// No caller host input. Candidate fields become accepted only under original trust.
    pub fn verify_kernel_policy<'s, 'a, 'p>(
        &'s mut self,
        bytes: &[u8],
        packet: &'p mut KernelPacket,
        authority: &'a mut FixedAuthorityRegistry,
    ) -> Result<KernelWorkerPolicyInput<'s, 'a, 'p>, TrustInputError> {
        let context = match (|| {
            self.check()?;
            if packet.is_refused() || authority.is_failed() {
                return Err(TrustInputError::Refused);
            }
            candidate_context(bytes)
        })() {
            Ok(context) => context,
            Err(error) => {
                self.refused = true;
                authority.latch_refusal();
                packet.latch_refusal();
                return Err(error);
            }
        };
        let mut policy = match self.verify_policy(bytes, &context, authority) {
            Ok(policy) => policy,
            Err(error) => {
                packet.latch_refusal();
                return Err(error);
            }
        };
        let probe = match (|| {
            packet.require_worker_context(&policy.host)?;
            let probe = Probe::open(&policy.host)?;
            packet.require_worker_context(&policy.host)?;
            policy.revalidate()?;
            Ok::<_, TrustInputError>(probe)
        })() {
            Ok(probe) => probe,
            Err(error) => {
                policy.latch_refusal();
                packet.latch_refusal();
                return Err(error);
            }
        };
        Ok(KernelWorkerPolicyInput {
            policy,
            packet,
            probe,
        })
    }
}

/// Keeps the same source, journal, packet and fixed Linux context; no extract/start.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::KernelWorkerPolicyInput;
/// fn launch(mut guard: KernelWorkerPolicyInput<'_, '_, '_>) { guard.start_worker(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::KernelWorkerPolicyInput;
/// fn extract(guard: KernelWorkerPolicyInput<'_, '_, '_>) { guard.into_worker_policy(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::KernelWorkerPolicyInput;
/// fn replace(mut guard: KernelWorkerPolicyInput<'_, '_, '_>) { guard.revalidate(b"replacement"); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::KernelWorkerPolicyInput;
/// fn send(guard: KernelWorkerPolicyInput<'static, 'static, 'static>) { std::thread::spawn(move || drop(guard)); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::WorkerTrustInput;
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::authority_registry::fixed::FixedAuthorityRegistry;
/// use hydracache_long_run_supervisor_074::diagnostic_kernel_admission::KernelPacket;
/// fn replace(source: &mut WorkerTrustInput, journal: &mut FixedAuthorityRegistry, packet: &mut KernelPacket, bytes: &[u8]) {
///     let mut original = source.verify_kernel_policy(bytes, packet, journal).unwrap();
///     packet.revalidate_sender().unwrap();
///     original.revalidate().unwrap();
/// }
/// ```
pub struct KernelWorkerPolicyInput<'s, 'a, 'p> {
    policy: WorkerPolicyInput<'s, 'a>,
    packet: &'p mut KernelPacket,
    probe: Probe,
}
impl KernelWorkerPolicyInput<'_, '_, '_> {
    // Private capture for the original controller attempt. No second journal owner.
    pub(super) fn controller_authority_binding(
        &mut self,
    ) -> Result<(String, u64, u64, String), TrustInputError> {
        self.revalidate()?;
        let view = active(
            self.policy.authority,
            &self.policy.input.source.trust,
            &self.policy.host,
        )?;
        let result = (
            self.policy.policy_sha256().to_owned(),
            view.epoch,
            view.revision,
            crate::sha256_hex(&self.policy.input.source.snapshot.raw),
        );
        self.revalidate()?;
        Ok(result)
    }

    pub(super) fn original_unix_seconds(&mut self) -> Result<u64, TrustInputError> {
        let result = (|| {
            self.revalidate()?;
            let now = self.packet.sample_original_unix_seconds()?;
            self.revalidate()?;
            Ok(now)
        })();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
    pub(super) fn latch_refusal(&mut self) {
        self.policy.latch_refusal();
        self.packet.latch_refusal();
    }
    pub(super) fn authenticate_configured_controller(
        &mut self,
        policy: &crate::diagnostic_ipc::DiagnosticPolicy,
    ) -> Result<crate::diagnostic_ipc::DiagnosticRequest, TrustInputError> {
        let result = (|| {
            self.revalidate()?;
            let request = self.packet.authenticate(policy)?;
            self.revalidate()?;
            Ok(request)
        })();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
    #[cfg(test)]
    pub(super) fn simulate_owned_fault_for_test(&mut self, case: u8) {
        match case {
            0 => self.packet.latch_refusal(),
            1 => self.probe.simulate_thread_drift_for_test(),
            _ => self.policy.policy.refused = true,
        }
    }
    pub fn is_refused(&self) -> bool {
        self.policy.is_refused() || self.packet.is_refused()
    }
    pub fn revalidate(&mut self) -> Result<(), TrustInputError> {
        let result = (|| {
            if self.is_refused() {
                return Err(TrustInputError::Refused);
            }
            self.policy.revalidate()?;
            self.packet.require_worker_context(&self.policy.host)?;
            self.probe.observe(&self.policy.host)?;
            self.packet.require_worker_context(&self.policy.host)?;
            self.policy.revalidate()
        })();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
}
