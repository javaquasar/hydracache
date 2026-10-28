#![cfg(target_os = "linux")]
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::OriginalCausalProgress;
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::preconsent::FramedConsentPreparation;
use hydracache_long_run_supervisor_074::diagnostic_kernel_admission::KernelPacket;
#[test]
fn preconsent_frame_requires_consumed_original_causal_owner() {
    fn probe(owner: OriginalCausalProgress<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        let _ = owner.prepare_framed_consent("1".repeat(64));
    }
    let _ = probe;
    fn named_owner(
        mut owner: FramedConsentPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        packet: KernelPacket,
    ) {
        let _ = owner.consent_to_sign();
        let _ = owner.revalidate();
        let _ = owner.accept_consent(packet);
    }
    let _ = named_owner;
}
