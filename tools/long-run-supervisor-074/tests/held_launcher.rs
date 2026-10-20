#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "support/held_launcher_process.rs"]
mod process;

#[test]
#[ignore = "owned nonroot synthetic ELF process/FD fault matrix; no systemd or product"]
fn owned_launcher_process_fault_matrix() {
    process::matrix();
}
