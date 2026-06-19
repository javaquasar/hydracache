use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("bench-budget") => xtask::bench_budget::run(args.collect())?,
        Some("--help") | Some("-h") | None => print_usage(),
        Some(command) => return Err(format!("unsupported xtask command: {command}").into()),
    }
    Ok(())
}

fn print_usage() {
    println!(
        "Usage:\n  cargo run -p xtask -- bench-budget [--budget benches/budget.toml] [--baseline benches/baseline/0_37.json] [--current target/criterion]"
    );
}
