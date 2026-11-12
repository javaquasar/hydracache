//! Separate actual-owner peer; this development schedule has no production protocol identity.
use hydracache_cluster_testkit::value_plane_extended_concurrent_075::{
    encode_extended_histories, execute_extended_concurrent, ExtendedSchedule, MAX_SCHEDULE_BYTES,
};
use std::io::{Read, Write};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let defect = match args.as_slice() {
        [] => false,
        [flag] if flag == "--canary-unconditional-version" => true,
        _ => return Err("concurrent peer arguments".into()),
    };
    let mut input = String::new();
    std::io::stdin()
        .take(MAX_SCHEDULE_BYTES as u64 + 1)
        .read_to_string(&mut input)?;
    let schedule = ExtendedSchedule::parse(&input)?;
    let histories = execute_extended_concurrent(&schedule, defect)?;
    std::io::stdout()
        .write_all(encode_extended_histories(schedule.seed, &histories)?.as_bytes())?;
    Ok(())
}
