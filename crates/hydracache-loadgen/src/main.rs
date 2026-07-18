use hydracache_loadgen::cli;
use hydracache_loadgen::tiers::local::write_local_report;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("hydracache-loadgen: {error}");
        std::process::exit(2);
    }
}

async fn run() -> Result<(), String> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.is_empty()
        || arguments
            .iter()
            .any(|argument| argument == "--help" || argument == "-h")
    {
        print_help();
        return Ok(());
    }
    let command = cli::parse(arguments)?;
    let path = command.local_report_path();
    write_local_report(command.profile(), &path)
        .await
        .map_err(|error| error.to_string())?;
    eprintln!(
        "hydracache-loadgen: wrote plumbing-only local smoke report to {}",
        path.display()
    );
    Ok(())
}

fn print_help() {
    println!(
        "HydraCache release-0.67 development load generator\n\nUSAGE:\n    hydracache-loadgen tier local --profile <PROFILE> --report <PATH>\n    hydracache-loadgen suite core --profile <PROFILE> --output-dir <DIR>\n\nUse smoke-v1 for explicitly plumbing-only output. reference-v1 fails closed until the W7 profile and receipt-bound prebuild context are present."
    );
}
