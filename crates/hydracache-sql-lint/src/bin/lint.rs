use std::error::Error;
use std::fs;
use std::path::PathBuf;

use hydracache_sql_lint::{Baseline, LintDiagnostic};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut update_baseline = false;
    let mut check_baseline = false;
    let mut baseline_path = PathBuf::from("lint-baseline.json");
    let mut diagnostics_path = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--update-baseline" => update_baseline = true,
            "--check-baseline" => check_baseline = true,
            "--baseline" => {
                let Some(path) = args.next() else {
                    return Err("--baseline requires a path".into());
                };
                baseline_path = PathBuf::from(path);
            }
            "--diagnostics" => {
                let Some(path) = args.next() else {
                    return Err("--diagnostics requires a path".into());
                };
                diagnostics_path = Some(PathBuf::from(path));
            }
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run -p hydracache-sql-lint --bin lint -- [--update-baseline | --check-baseline] [--baseline PATH] [--diagnostics PATH]"
                );
                return Ok(());
            }
            other => return Err(format!("unsupported lint argument: {other}").into()),
        }
    }

    let diagnostics = load_diagnostics(diagnostics_path.as_ref())?;

    if update_baseline {
        Baseline::from_diagnostics(diagnostics.iter()).save(&baseline_path)?;
        println!(
            "wrote HydraCache SQL lint baseline to {}",
            baseline_path.display()
        );
    } else if check_baseline {
        let baseline = Baseline::load(&baseline_path)?;
        let diff = baseline.diff(diagnostics);
        if !diff.new_findings.is_empty() || !diff.stale_entries.is_empty() {
            if !diff.new_findings.is_empty() {
                eprintln!("new SQL lint finding(s): {}", diff.new_findings.len());
                for diagnostic in &diff.new_findings {
                    eprintln!("  {} {}", diagnostic.policy, diagnostic.fingerprint);
                }
            }
            if !diff.stale_entries.is_empty() {
                eprintln!(
                    "stale SQL lint baseline entries: {}",
                    diff.stale_entries.len()
                );
                for fingerprint in &diff.stale_entries {
                    eprintln!("  {fingerprint}");
                }
            }
            return Err("SQL lint baseline drift detected".into());
        }
        println!(
            "HydraCache SQL lint baseline passed: {} accepted finding(s)",
            diff.accepted_findings.len()
        );
    } else {
        println!(
            "HydraCache SQL lint CLI is ready; pass --check-baseline to enforce baseline drift."
        );
    }

    Ok(())
}

fn load_diagnostics(path: Option<&PathBuf>) -> Result<Vec<LintDiagnostic>, Box<dyn Error>> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}
