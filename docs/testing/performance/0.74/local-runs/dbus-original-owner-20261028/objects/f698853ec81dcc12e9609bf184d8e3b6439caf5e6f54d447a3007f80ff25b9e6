//! Fixed nonproduct self-exec probe; never a signer, worker or general launcher.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "../diagnostic_signer_owned_probe.rs"]
mod owned;

fn main() {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    owned::entry();
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        eprintln!("Fixed owned probe refused.");
        std::process::exit(2);
    }
}
