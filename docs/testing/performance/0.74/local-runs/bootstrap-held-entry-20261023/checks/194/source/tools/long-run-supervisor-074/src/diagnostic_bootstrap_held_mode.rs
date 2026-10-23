//! Separate readiness entry; old modes and execution gates remain closed.
#[path = "diagnostic_held_activation.rs"]
mod activation;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "diagnostic_bootstrap_held_transport.rs"]
pub(crate) mod transport;
pub(crate) fn requested() -> bool {
    std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new(activation::ARG))
}
pub(crate) fn run() -> std::io::Result<()> {
    let checked = activation::inspect(&std::env::args_os().skip(1).take(2).collect::<Vec<_>>(),
        &std::env::vars_os().take(6).collect::<Vec<_>>(),std::process::id())?;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    { transport::run(checked.invocation) }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    { let _ = checked.invocation; Err(std::io::Error::new(std::io::ErrorKind::Unsupported,"Linux x86-64 required")) }
}
