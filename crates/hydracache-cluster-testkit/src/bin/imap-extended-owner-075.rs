//! Development-only actual Rust owner peer for the shared v2 oracle gate.
use hydracache_cluster_testkit::value_plane_differential_075::hex;
use hydracache_cluster_testkit::value_plane_extended_075::{
    execute_extended, execute_extended_drop_event, ExtendedCorpus, MAX_BYTES,
};
use std::io::{Read, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let fault = match args.as_slice() {
        [] => false,
        [flag] if flag == "--canary-drop-event" => true,
        _ => return Err("unknown actual-owner peer argument".into()),
    };
    let mut input = String::new();
    std::io::stdin()
        .take(MAX_BYTES as u64 + 1)
        .read_to_string(&mut input)?;
    let corpus = ExtendedCorpus::parse(&input)?;
    let observations = if fault {
        execute_extended_drop_event(&corpus)?
    } else {
        execute_extended(&corpus)?
    };
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    writeln!(out, "imap-extended-observations-v2\tactual")?;
    for (step, observation) in corpus.steps.iter().zip(observations) {
        writeln!(out, "{}\t{}", hex(step.id.as_bytes()), observation)?;
    }
    out.flush()?;
    Ok(())
}
