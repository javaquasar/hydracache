//! Protected job producer only. Never starts or contacts a supervisor/worker.
fn main() {
    #[cfg(target_os = "linux")]
    {
        use hydracache_long_run_supervisor_074::diagnostic_controller_producer::linux;
        use std::path::Path;
        let args: Vec<String> = std::env::args().skip(1).collect();
        if let [command, request, policy, receipt] = args.as_slice() {
            if command == "audience" {
                if let Ok(prepared) =
                    linux::prepare_files(Path::new(request), Path::new(policy), Path::new(receipt))
                {
                    println!("{}", prepared.audience());
                    return;
                }
            }
        }
        if let [command, request, policy, receipt, key, output] = args.as_slice() {
            if command == "sign"
                && linux::sign_files(
                    Path::new(request),
                    Path::new(policy),
                    Path::new(receipt),
                    Path::new(key),
                    Path::new(output),
                )
                .is_ok()
            {
                println!(
                    "Diagnostic packet created; origin/replay/host/start admission not authorized."
                );
                return;
            }
        }
    }
    eprintln!("Diagnostic controller producer refused; use separate Linux audience/sign commands, no override or host route.");
    std::process::exit(2);
}
