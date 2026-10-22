#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "support/held_launcher_process.rs"]
mod process;

#[path = "support/held_activation_process.rs"]
mod activation;

#[test]
fn held_activation_environment_ingress_reaches_only_unchanged_core_refusal() {
    activation::controls();
}

#[test]
#[ignore = "owned nonroot synthetic ELF process/FD fault matrix; no systemd or product"]
fn owned_launcher_process_fault_matrix() {
    process::matrix();
}
