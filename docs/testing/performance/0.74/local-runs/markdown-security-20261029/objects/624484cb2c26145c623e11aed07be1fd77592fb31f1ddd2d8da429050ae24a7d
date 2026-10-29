//! Deliberately separate from the installed supervisor and its IPC routes.
fn main() {
    #[cfg(target_os = "linux")]
    {
        let arguments: Vec<String> = std::env::args().skip(1).collect();
        if let [command, policy, pin, controller, key, observation, bundle, output] =
            arguments.as_slice()
        {
            if command == "sign" {
                if hydracache_long_run_supervisor_074::diagnostic_builder::linux::sign_files(
                    std::path::Path::new(policy),
                    pin,
                    controller,
                    std::path::Path::new(key),
                    std::path::Path::new(observation),
                    std::path::Path::new(bundle),
                    std::path::Path::new(output),
                )
                .is_ok()
                {
                    println!("Diagnostic build receipt created; installation/execution/admission not authorized.");
                    return;
                }
                eprintln!("Diagnostic builder signing refused; no authority fallback.");
                std::process::exit(2);
            }
        }
        eprintln!("usage: diagnostic-builder sign POLICY PIN_SHA256 CONTROLLER_PUBLIC_HEX KEY_FILE OBSERVATION BUNDLE OUTPUT");
    }
    #[cfg(not(target_os = "linux"))]
    eprintln!("Diagnostic builder signing requires the protected Linux signing lane.");
    std::process::exit(2);
}
