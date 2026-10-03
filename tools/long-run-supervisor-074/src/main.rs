use hydracache_long_run_supervisor_074::verify_journal;
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next(), args.next()) {
        (Some("verify"), Some(path), None) => {
            let report = verify_journal(&PathBuf::from(path))?;
            println!("{}", serde_json::to_string(&report)?);
            Ok(())
        }
        _ => Err("usage: hydracache-long-run-supervisor-074 verify <checkpoints.jsonl> (live supervisor operations are not implemented in this local foundation)".into()),
    }
}
