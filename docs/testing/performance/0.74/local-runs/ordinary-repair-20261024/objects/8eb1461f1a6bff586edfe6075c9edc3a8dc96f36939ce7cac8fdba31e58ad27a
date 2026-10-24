//! Fixed nonproduct child-entry check, never a signer/worker or general launcher.
fn main() {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        use hydracache_long_run_supervisor_074::diagnostic_signer_isolation::{
            run_fixed_probe, FixedProbe,
        };
        let args: Vec<_> = std::env::args_os().skip(1).take(2).collect();
        if let [arg] = args.as_slice() {
            let probe = match arg.to_str() {
                Some("native") => Some(FixedProbe::Native),
                Some("x32") => Some(FixedProbe::X32),
                Some("i386") => Some(FixedProbe::I386),
                _ => None,
            };
            if let Some(probe) = probe {
                run_fixed_probe(probe);
            }
        }
    }
    eprintln!("Fixed signer isolation probe refused.");
    std::process::exit(2);
}
