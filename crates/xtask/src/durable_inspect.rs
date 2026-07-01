use std::error::Error;
use std::path::PathBuf;

use hydracache::{inspect_replicated_store, DurableValueStore};

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.len() != 1 || matches!(args.first().map(String::as_str), Some("--help" | "-h")) {
        print_usage();
        return if args.len() == 1 {
            Ok(())
        } else {
            Err("missing durable store directory".into())
        };
    }

    let path = PathBuf::from(&args[0]);
    let store = DurableValueStore::open(path)?;
    let records = inspect_replicated_store(&store)?;
    println!("{}", serde_json::to_string_pretty(&records)?);
    Ok(())
}

fn print_usage() {
    println!("Usage: cargo xtask durable-inspect <store-dir>");
}
