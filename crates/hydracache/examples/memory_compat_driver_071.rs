use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

use hydracache::{
    ClusterEpoch, DurableValueStore, PartitionId, ReplicatedSlot, ReplicatedValueRecord,
    ReplicatedValueStore, DURABLE_VALUE_FORMAT_VERSION,
};

const STORE_BUDGET: u64 = 4 * 1024 * 1024;
const LARGE_VALUE_BYTES: usize = 64 * 1024;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mode = args.next().ok_or("missing mode")?;
    let path = args.next().ok_or("missing store path")?;
    if args.next().is_some() {
        return Err("unexpected trailing argument".into());
    }
    match mode.as_str() {
        "create-baseline" => create_baseline(Path::new(&path))?,
        "verify-and-mutate-candidate" => verify_and_mutate_candidate(Path::new(&path))?,
        "verify-complete" => verify_complete(Path::new(&path))?,
        "hold-after-flush" => hold_after_flush(Path::new(&path))?,
        "write-future-and-refuse" => write_future_and_refuse(Path::new(&path))?,
        _ => return Err(format!("unsupported mode {mode}").into()),
    }
    Ok(())
}

fn create_baseline(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut store = DurableValueStore::open_with_budget(path, STORE_BUDGET)?;
    store.upsert("empty", value_record(1, Vec::new()))?;
    store.upsert(
        "maximum-record",
        value_record(2, vec![0x5a; LARGE_VALUE_BYTES]),
    )?;
    store.tombstone("deleted", PartitionId::new(7), 3, ClusterEpoch::new(11))?;
    store.flush()?;
    Ok(())
}

fn verify_and_mutate_candidate(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut store = open_after_lock_release(path)?;
    verify_baseline(&store)?;
    store.upsert("candidate-record", value_record(4, b"candidate".to_vec()))?;
    store.tombstone(
        "candidate-tombstone",
        PartitionId::new(7),
        5,
        ClusterEpoch::new(11),
    )?;
    store.flush()?;
    Ok(())
}

fn verify_complete(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let store = open_after_lock_release(path)?;
    verify_baseline(&store)?;
    let candidate = store
        .get("candidate-record")?
        .ok_or("candidate record is missing")?;
    if value_bytes(&candidate) != Some(b"candidate".as_slice()) {
        return Err("candidate record changed".into());
    }
    if !store
        .get("candidate-tombstone")?
        .ok_or("candidate tombstone is missing")?
        .is_tombstone()
    {
        return Err("candidate tombstone was resurrected".into());
    }
    Ok(())
}

fn hold_after_flush(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let store = open_after_lock_release(path)?;
    verify_baseline(&store)?;
    store.flush()?;
    println!("READY");
    io::stdout().flush()?;
    thread::sleep(Duration::from_secs(600));
    Ok(())
}

fn write_future_and_refuse(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    DurableValueStore::write_format_marker_for_test(path, DURABLE_VALUE_FORMAT_VERSION + 1)?;
    match open_after_lock_release(path) {
        Ok(_) => Err("unknown future durable format was accepted".into()),
        Err(error)
            if error
                .to_string()
                .contains("unsupported durable value-store format") =>
        {
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn verify_baseline(store: &DurableValueStore) -> Result<(), Box<dyn std::error::Error>> {
    let empty = store.get("empty")?.ok_or("empty record is missing")?;
    if value_bytes(&empty) != Some(&[]) {
        return Err("empty record changed".into());
    }
    let maximum = store
        .get("maximum-record")?
        .ok_or("maximum record is missing")?;
    if value_bytes(&maximum) != Some(vec![0x5a; LARGE_VALUE_BYTES].as_slice()) {
        return Err("maximum record changed".into());
    }
    if !store
        .get("deleted")?
        .ok_or("baseline tombstone is missing")?
        .is_tombstone()
    {
        return Err("baseline tombstone was resurrected".into());
    }
    Ok(())
}

fn value_record(version: u64, value: Vec<u8>) -> ReplicatedValueRecord {
    ReplicatedValueRecord::value(PartitionId::new(7), version, ClusterEpoch::new(11), value)
}

fn value_bytes(record: &ReplicatedValueRecord) -> Option<&[u8]> {
    match &record.state {
        ReplicatedSlot::Value { value, .. } => Some(value.as_slice()),
        ReplicatedSlot::Tombstone { .. } => None,
    }
}

fn open_after_lock_release(path: &Path) -> Result<DurableValueStore, Box<dyn std::error::Error>> {
    for attempt in 0..100 {
        match DurableValueStore::open_with_budget(path, STORE_BUDGET) {
            Ok(store) => return Ok(store),
            Err(error) if error.to_string().contains("could not acquire lock") && attempt < 99 => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error.into()),
        }
    }
    Err("durable value-store lock was not released".into())
}
