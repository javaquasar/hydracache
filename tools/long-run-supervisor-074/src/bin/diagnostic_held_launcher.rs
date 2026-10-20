//! Private inherited-FD mechanics; not an enrolled original-start CLI.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "../diagnostic_held_launcher_kernel.rs"]
mod kernel;

fn main() {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    if std::env::args_os().len() == 1 {
        if let Err(error) = kernel::run() {
            eprintln!("Held launcher refused: {error}; outcome uncertain, never resend.");
        }
        std::process::exit(2);
    }
    eprintln!("Held launcher requires the fixed inherited-FD Linux x86-64 protocol; no command/path fallback.");
    std::process::exit(2);
}
