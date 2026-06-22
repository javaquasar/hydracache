use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("bench-budget") => xtask::bench_budget::run(args.collect())?,
        Some("doc-check") => xtask::doc_check::run(args.collect())?,
        Some("verify") => xtask::verify::run(args.collect())?,
        Some("--help") | Some("-h") | None => print_usage(),
        Some(command) => return Err(format!("unsupported xtask command: {command}").into()),
    }
    Ok(())
}

fn print_usage() {
    println!(
        "Usage:\n  \
         cargo xtask verify        # run the fast release gates (see docs/GATES.md)\n  \
         cargo xtask doc-check     # validate docs/plans/releases.toml (RULES R-11)\n  \
         cargo xtask bench-budget [--budget benches/budget.toml] [--baseline benches/baseline/0_37.json] [--current target/criterion]\n\n\
         (The `cargo xtask` alias is defined in .cargo/config.toml; `cargo run -p xtask -- <cmd>` also works.)"
    );
}
