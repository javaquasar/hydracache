//! Deliberately faulty local proof workers. Never launched by a production path.
use std::io::{Read, Write};
use std::process::{Command, Stdio};
// LLVM's supported manual-buffer mode: this binary may be deliberately killed.
// Suppress the default initializer/atexit writer; checkpoints are published atomically.
#[cfg(feature = "imap-075-coverage")]
#[used]
#[unsafe(no_mangle)]
static __llvm_profile_runtime: std::ffi::c_int = 0;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("echo") => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .take(2 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            std::io::stdout().write_all(&bytes)?;
        }
        Some("environment") => {
            for variable in [
                "JAVA_TOOL_OPTIONS",
                "JDK_JAVA_OPTIONS",
                "_JAVA_OPTIONS",
                "CLASSPATH",
                "LD_PRELOAD",
                "LD_AUDIT",
                "LD_LIBRARY_PATH",
            ] {
                writeln!(
                    std::io::stdout(),
                    "{variable}={}",
                    std::env::var_os(variable).is_some()
                )?;
            }
            writeln!(
                std::io::stdout(),
                "sentinel={}",
                std::env::var("IMAP_PROOF_TEST_SENTINEL")?
            )?;
        }
        Some("sleep" | "ignore_stdin") => loop {
            std::thread::park_timeout(std::time::Duration::from_secs(60));
        },
        Some("nested-controller" | "nested-controller-complete") => {
            let mut child = Command::new(std::env::current_exe()?);
            child
                .arg("announce-sleep")
                .arg(args.get(1).ok_or("nested PID file required")?);
            if args[0] == "nested-controller-complete" {
                child.arg("--exit-after-announce");
            }
            xtask::imap_proof_process_075::execute(
                child,
                &[],
                xtask::imap_proof_process_075::ProcessBounds {
                    timeout: std::time::Duration::from_secs(30),
                    ..Default::default()
                },
            )?;
        }
        Some("announce-sleep") => {
            let file = args.get(1).ok_or("PID file required")?;
            let exit_after = args.get(2).is_some_and(|s| s == "--exit-after-announce");
            let mut ready = false;
            loop {
                std::thread::park_timeout(std::time::Duration::from_millis(2));
                if !ready {
                    xtask::imap_coverage_075::flush_profile()?;
                    std::fs::write(file, std::process::id().to_string())?;
                    ready = true;
                }
                if exit_after {
                    xtask::imap_coverage_075::flush_profile()?;
                    return Ok(());
                }
            }
        }
        Some("stdout" | "stderr") => {
            let mut out: Box<dyn Write> = if args[0] == "stdout" {
                Box::new(std::io::stdout())
            } else {
                Box::new(std::io::stderr())
            };
            loop {
                out.write_all(&[b'x'; 8192])?;
            }
        }
        Some("hold_pipe") => {
            let child = Command::new(std::env::current_exe()?)
                .arg("sleep")
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()?;
            let pid = child.id();
            if let Some(file) = args.get(1) {
                xtask::imap_coverage_075::flush_profile()?;
                std::fs::write(file, pid.to_string())?;
            }
            writeln!(std::io::stdout(), "descendant:{pid}")?;
            // Negative fixture: only the managed parent owns cleanup after root exit.
            std::mem::forget(child);
        }
        Some("memory") => {
            let mut bytes = Vec::<u8>::new();
            if bytes
                .try_reserve_exact(3usize * 1024 * 1024 * 1024)
                .is_err()
            {
                writeln!(std::io::stderr(), "allocation_refused")?;
                std::process::exit(42);
            }
            bytes.resize(3usize * 1024 * 1024 * 1024, 1);
            std::hint::black_box(bytes);
            std::process::exit(43);
        }
        Some("error") => {
            writeln!(std::io::stderr(), "fixture_exit")?;
            std::process::exit(23);
        }
        _ => return Err("unknown proof worker mode".into()),
    }
    xtask::imap_coverage_075::flush_profile()?;
    Ok(())
}
