//! Same actual-engine probe in both isolated overlays; never a live entry point.
use super::*;
#[path = "runtime_allocation_counter.rs"]
mod meter;
#[path = "runtime_allocation_storage.rs"]
mod storage;
use std::os::unix::fs::FileExt as _;

const ITERATIONS: usize = 32;
fn counts(value: meter::Counts) -> serde_json::Value {
    serde_json::json!({"allocations":value.allocations,"zeroed":value.zeroed,
        "reallocations":value.reallocations,"requested_bytes":value.requested_bytes,
        "deallocations":value.deallocations,"released_bytes":value.released_bytes,
        "live_delta":value.live(),"peak_delta":value.peak_delta})
}
fn rss(file: &File) -> u64 {
    let mut raw = [0; 256];
    let count = file.read_at(&mut raw, 0).unwrap();
    assert!(count > 0 && count < raw.len());
    let resident: u64 = std::str::from_utf8(&raw[..count])
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    assert!(page > 0);
    resident.checked_mul(page as u64).unwrap()
}
fn digest(registry: &FixtureAuthorityRegistry) -> String {
    let mut raw = Vec::new();
    for entry in &registry.entries {
        for leaf in [&entry.record, &entry.marker] {
            raw.extend_from_slice(&(leaf.name.len() as u64).to_le_bytes());
            raw.extend_from_slice(leaf.name.as_bytes());
            raw.extend_from_slice(&(leaf.raw.len() as u64).to_le_bytes());
            raw.extend_from_slice(&leaf.raw);
        }
    }
    sha256_hex(&raw)
}
fn owned(registry: &FixtureAuthorityRegistry) -> serde_json::Value {
    let raw_capacity: usize = registry
        .entries
        .iter()
        .map(|entry| entry.record.raw.capacity() + entry.marker.raw.capacity())
        .sum();
    let (slots, scratch_capacity) = storage::scratch(registry);
    serde_json::json!({"generations":registry.entries.len(),"entry_capacity":registry.entries.capacity(),
        "entry_size":std::mem::size_of::<Entry>(),"leaf_size":std::mem::size_of::<Leaf>(),
        "raw_capacity":raw_capacity,"scratch_slots":slots,"scratch_capacity":scratch_capacity})
}
fn case(depth: usize, fault: &str) -> serde_json::Value {
    let (root, uid, gid) = fixture();
    let mut policy = checked(7);
    let mut producer = initialize_fixture_registry(root.path(), uid, gid, &policy).unwrap();
    for epoch in 8..8 + (depth / 2) as u64 {
        producer.refuse(RefusalReason::Operator).unwrap();
        policy = checked(epoch);
        producer.advance(&policy).unwrap();
    }
    assert_eq!(producer.entries.len(), depth);
    let corpus = digest(&producer);
    drop(producer);
    let statm = File::open("/proc/self/statm").unwrap();
    let rss_before = rss(&statm);
    let (mut registry, opened) = meter::capture(|| {
        open_fixture_registry(root.path(), uid, gid, &policy.trust, &policy.context).unwrap()
    });
    assert_eq!(digest(&registry), corpus);
    let original: Vec<_> = registry
        .entries
        .iter()
        .flat_map(|entry| [&entry.record, &entry.marker])
        .map(|leaf| {
            (
                leaf.file.as_raw_fd(),
                leaf.stamp.clone(),
                sha256_hex(&leaf.raw),
            )
        })
        .collect();
    let after_open = owned(&registry);
    let rss_open = rss(&statm);
    let (_, warm) = meter::capture(|| {
        for _ in 0..ITERATIONS {
            let view = registry.inspect(&policy.trust, &policy.context).unwrap();
            assert_eq!(view.revision, depth as u64);
            assert!(!view.refused);
            std::hint::black_box(view);
        }
    });
    assert_eq!(digest(&registry), corpus);
    for (leaf, (fd, stamp, raw)) in registry
        .entries
        .iter()
        .flat_map(|entry| [&entry.record, &entry.marker])
        .zip(&original)
    {
        assert_eq!(leaf.file.as_raw_fd(), *fd);
        assert!(leaf.stamp == *stamp);
        assert_eq!(sha256_hex(&leaf.raw), *raw);
    }
    let after_warm = owned(&registry);
    let rss_warm = rss(&statm);
    let max_publication = if depth == 127 {
        let (_, count) = meter::capture(|| registry.refuse(RefusalReason::Operator).unwrap());
        assert_eq!(registry.entries.len(), MAX_GENERATIONS);
        assert!(registry.view().unwrap().refused);
        Some((count, owned(&registry)))
    } else {
        None
    };
    let wrong = checked(policy.trust.epoch + 1);
    if fault == "validation" {
        fs::remove_file(root.path().join(name(depth, "commit"))).unwrap();
    }
    let (_, refused) = meter::capture(|| match fault {
        "external" => registry.latch_refusal(),
        "validation" => assert!(registry.inspect(&policy.trust, &policy.context).is_err()),
        "binding" => assert!(matches!(
            registry.inspect(&wrong.trust, &wrong.context),
            Err(AuthorityError::Binding)
        )),
        "transition" => assert!(matches!(
            registry.advance(&wrong),
            Err(AuthorityError::Transition)
        )),
        "publication" => assert!(matches!(
            registry.change(
                |record| record.refuse(RefusalReason::Storage),
                Some(PublicationStage::RecordCreated)
            ),
            Err(AuthorityError::Injected(PublicationStage::RecordCreated))
        )),
        _ => panic!("unregistered allocation case"),
    });
    assert!(registry.is_failed());
    assert_eq!(storage::scratch(&registry), (0, 0));
    let after_refusal = owned(&registry);
    let rss_refused = rss(&statm);
    let (_, sticky) = meter::capture(|| {
        assert!(matches!(
            registry.inspect(&policy.trust, &policy.context),
            Err(AuthorityError::Refused)
        ))
    });
    let (_, dropped) = meter::capture(|| drop(registry));
    let extra = max_publication.as_ref().map_or(0, |value| value.0.live());
    assert_eq!(
        opened.live() + warm.live() + extra + refused.live() + sticky.live() + dropped.live(),
        0,
        "owner layout lifetime must reconcile"
    );
    serde_json::json!({"depth":depth,"fault":fault,"seed":74,"iterations":ITERATIONS,
        "corpus":corpus,"root_name_bytes":root.path().file_name().unwrap().len(),
        "open":counts(opened),"warm":counts(warm),"refusal":counts(refused),
        "sticky":counts(sticky),"drop":counts(dropped),"after_open":after_open,
        "after_warm":after_warm,"after_refusal":after_refusal,
        "max_publication":max_publication.map(|(count,state)|serde_json::json!({"counts":counts(count),"state":state})),
        "rss":{"before":rss_before,"open":rss_open,"warm":rss_warm,"refused":rss_refused,"drop":rss(&statm)},
        "owner_reconciled":true,"original_objects_preserved":true,"server":false,"product":false})
}
#[test]
fn runtime_allocation_semantic_control() {
    const { assert!(!cfg!(feature = "fixed-leaf-alloc-probe")) };
    use crate::diagnostic_worker_policy::authority_registry::fixture::name_capacity_controls::{
        render, Reading,
    };
    for depth in [1, 7, 127] {
        for suffix in ["json", "commit"] {
            assert_eq!(
                render(Reading::Reference, depth, suffix),
                name(depth, suffix)
            );
        }
    }
    let row = case(1, "external");
    assert_eq!(row["owner_reconciled"], true);
    assert_eq!(row["iterations"], 32);
}
#[test]
fn runtime_allocation_measurement() {
    const { assert!(!cfg!(feature = "fixed-leaf-alloc-probe")) };
    for depth in [1, 7, 31, 127] {
        println!("HC074-RUNTIME-ALLOCATION {}", case(depth, "external"));
    }
    for fault in ["validation", "binding", "transition", "publication"] {
        println!("HC074-RUNTIME-ALLOCATION {}", case(7, fault));
    }
}
