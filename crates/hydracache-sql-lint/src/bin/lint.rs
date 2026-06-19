use std::error::Error;
use std::path::PathBuf;

use hydracache_sql_lint::Baseline;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut update_baseline = false;
    let mut baseline_path = PathBuf::from("lint-baseline.json");

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--update-baseline" => update_baseline = true,
            "--baseline" => {
                let Some(path) = args.next() else {
                    return Err("--baseline requires a path".into());
                };
                baseline_path = PathBuf::from(path);
            }
            "--help" | "-h" => {
                println!("Usage: cargo run -p hydracache-sql-lint --bin lint -- [--update-baseline] [--baseline PATH]");
                return Ok(());
            }
            other => return Err(format!("unsupported lint argument: {other}").into()),
        }
    }

    if update_baseline {
        Baseline::default().save(&baseline_path)?;
        println!(
            "wrote empty HydraCache SQL lint baseline to {}",
            baseline_path.display()
        );
    } else {
        println!(
            "HydraCache SQL lint CLI is ready; policy collection is provided by CI harnesses."
        );
    }

    Ok(())
}
