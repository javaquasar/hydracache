#![cfg(target_os = "linux")]
use hydracache_long_run_supervisor_074::diagnostic_signer_runtime_input::SignerRuntimeCompanions;

#[test]
fn supplied_companions_do_not_include_binary_or_runtime_manifest() {
    let companions = SignerRuntimeCompanions {
        root_lock: b"lock",
        build_log: b"log",
        bootstrap_context: b"context",
    };
    assert_eq!(companions.root_lock, b"lock");
    assert_eq!(companions.build_log, b"log");
    assert_eq!(companions.bootstrap_context, b"context");
}
