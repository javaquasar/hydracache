use std::path::PathBuf;

use hydracache_loadgen::notification_observer::{
    default_prototype_output, run_and_write_prototype,
};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("hydracache-notification-observer-prototype: {error}");
        std::process::exit(2);
    }
}

async fn run() -> Result<(), String> {
    let mut output = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => {
                output = Some(PathBuf::from(
                    args.next().ok_or("--output requires a path")?,
                ));
            }
            "--help" | "-h" => {
                println!("hydracache-notification-observer-prototype [--output <new-json-path>]");
                return Ok(());
            }
            _ => return Err(format!("unsupported argument: {arg}")),
        }
    }
    let output = output.unwrap_or_else(default_prototype_output);
    let receipt = run_and_write_prototype(&output).await?;
    println!(
        "hydracache-notification-observer-prototype: OK ({} cases, diagnostic only, {})",
        receipt.cases.len(),
        output.display()
    );
    Ok(())
}
