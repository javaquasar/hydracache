#![cfg(target_os = "linux")]
#[test]
fn controller_trust_input_public_entry_is_fixed_and_readonly() {
    use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::{open_fixed_controller_trust, SignedControllerInput};
    let _ = open_fixed_controller_trust;
    let _ = SignedControllerInput::revalidate;
}
#[test]
fn controller_build_input_public_entry_has_no_identity_or_trust_override() {
    use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::SignedControllerInput;
    let _ = SignedControllerInput::bind_reviewed_build;
}
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::{
    KernelWorkerPolicyInput, WorkerTrustInput,
};

#[test]
fn oidc_public_entry_is_fixed_original_binding_not_execution() {
    use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::{ControllerBuildInput, oidc_input::open_fixed_oidc_trust};
    let _ = open_fixed_oidc_trust;
    let _ = ControllerBuildInput::bind_oidc;
}

#[test]
fn worker_kernel_binding_public_entry_has_no_host_or_revalidation_override() {
    fn bind<'s, 'a, 'p>(
        source: &'s mut WorkerTrustInput,
        journal: &'a mut hydracache_long_run_supervisor_074::diagnostic_worker_policy::authority_registry::fixed::FixedAuthorityRegistry,
        packet: &'p mut hydracache_long_run_supervisor_074::diagnostic_kernel_admission::KernelPacket,
        bytes: &[u8],
    ) -> Result<KernelWorkerPolicyInput<'s, 'a, 'p>, hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::TrustInputError>{
        source.verify_kernel_policy(bytes, packet, journal)
    }
    let _ = bind;
    let _ = KernelWorkerPolicyInput::revalidate;
}
