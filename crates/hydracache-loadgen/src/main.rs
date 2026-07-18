fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None | Some("--help" | "-h") => print_help(),
        Some("suite") => {
            let suite = args.next().unwrap_or_else(|| "<missing>".to_owned());
            eprintln!(
                "hydracache-loadgen: suite {suite:?} is not available until its release-0.67 W-item lands"
            );
            std::process::exit(2);
        }
        Some(other) => {
            eprintln!("hydracache-loadgen: unknown command {other:?}");
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!(
        "HydraCache release-0.67 development load generator\n\nUSAGE:\n    hydracache-loadgen suite <core|resp|control-plane> [OPTIONS]"
    );
}
