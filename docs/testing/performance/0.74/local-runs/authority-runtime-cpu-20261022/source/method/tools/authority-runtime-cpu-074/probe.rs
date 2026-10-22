//! Identical timing probe in isolated actual-engine overlays, never a live entry.
use super::*;
#[path = "runtime_cpu_storage.rs"]
mod storage;

const ITERATIONS: usize = 32;
#[derive(Clone, Copy)]
struct Stamp {
    wall: u64,
    cpu: u64,
    user: u64,
    system: u64,
    voluntary: u64,
    involuntary: u64,
    inblock: u64,
    oublock: u64,
}
fn ns(value: libc::timespec) -> u64 {
    assert!(value.tv_sec >= 0 && value.tv_nsec >= 0 && value.tv_nsec < 1_000_000_000);
    (value.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .unwrap()
        .checked_add(value.tv_nsec as u64)
        .unwrap()
}
fn clock(id: libc::clockid_t, resolution: bool) -> u64 {
    let mut value = std::mem::MaybeUninit::<libc::timespec>::uninit();
    // SAFETY: the syscall initializes the properly aligned output on success.
    let result = unsafe {
        if resolution {
            libc::clock_getres(id, value.as_mut_ptr())
        } else {
            libc::clock_gettime(id, value.as_mut_ptr())
        }
    };
    assert_eq!(result, 0);
    ns(unsafe { value.assume_init() })
}
fn timeval(value: libc::timeval) -> u64 {
    assert!(value.tv_sec >= 0 && value.tv_usec >= 0 && value.tv_usec < 1_000_000);
    (value.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .unwrap()
        .checked_add(value.tv_usec as u64 * 1_000)
        .unwrap()
}
impl Stamp {
    fn now() -> Self {
        let wall = clock(libc::CLOCK_MONOTONIC_RAW, false);
        let cpu = clock(libc::CLOCK_THREAD_CPUTIME_ID, false);
        let mut value = std::mem::MaybeUninit::<libc::rusage>::uninit();
        assert_eq!(
            unsafe { libc::getrusage(libc::RUSAGE_THREAD, value.as_mut_ptr()) },
            0
        );
        let value = unsafe { value.assume_init() };
        Self {
            wall,
            cpu,
            user: timeval(value.ru_utime),
            system: timeval(value.ru_stime),
            voluntary: value.ru_nvcsw.try_into().unwrap(),
            involuntary: value.ru_nivcsw.try_into().unwrap(),
            inblock: value.ru_inblock.try_into().unwrap(),
            oublock: value.ru_oublock.try_into().unwrap(),
        }
    }
    fn since(self, before: Self) -> serde_json::Value {
        serde_json::json!({"wall_ns":self.wall.checked_sub(before.wall).unwrap(),
            "cpu_ns":self.cpu.checked_sub(before.cpu).unwrap(),
            "user_ns":self.user.checked_sub(before.user).unwrap(),
            "system_ns":self.system.checked_sub(before.system).unwrap(),
            "voluntary":self.voluntary.checked_sub(before.voluntary).unwrap(),
            "involuntary":self.involuntary.checked_sub(before.involuntary).unwrap(),
            "inblock":self.inblock.checked_sub(before.inblock).unwrap(),
            "oublock":self.oublock.checked_sub(before.oublock).unwrap()})
    }
}
fn measure<T>(call: impl FnOnce() -> T) -> (T, serde_json::Value) {
    let before = Stamp::now();
    let value = call();
    let after = Stamp::now();
    (value, after.since(before))
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
fn prepare(depth: usize) -> (TempDir, u32, u32, CheckedWorkerPolicy, String) {
    assert!(matches!(depth, 1 | 7 | 31 | 127));
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
    (root, uid, gid, policy, corpus)
}
fn inspect(registry: &mut FixtureAuthorityRegistry, policy: &CheckedWorkerPolicy, depth: usize) {
    for _ in 0..ITERATIONS {
        let view = registry.inspect(&policy.trust, &policy.context).unwrap();
        assert_eq!(view.revision, depth as u64);
        assert!(!view.refused);
        std::hint::black_box(view);
    }
}
fn phased(depth: usize, fault: &str) -> serde_json::Value {
    let (root, uid, gid, policy, corpus) = prepare(depth);
    let (mut registry, opened) = measure(|| {
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
    let (_, warm) = measure(|| inspect(&mut registry, &policy, depth));
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
    let scratch_warm = storage::scratch(&registry);
    let publication = if depth == 127 {
        let (_, cost) = measure(|| registry.refuse(RefusalReason::Operator).unwrap());
        assert_eq!(registry.entries.len(), MAX_GENERATIONS);
        assert!(registry.view().unwrap().refused);
        Some(cost)
    } else {
        None
    };
    let wrong = checked(policy.trust.epoch + 1);
    if fault == "validation" {
        fs::remove_file(root.path().join(name(depth, "commit"))).unwrap();
    }
    let (_, refusal) = measure(|| match fault {
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
        _ => panic!("unregistered CPU case"),
    });
    assert!(registry.is_failed());
    assert_eq!(storage::scratch(&registry), (0, 0));
    let (_, sticky) = measure(|| {
        assert!(matches!(
            registry.inspect(&policy.trust, &policy.context),
            Err(AuthorityError::Refused)
        ))
    });
    let (_, dropped) = measure(|| drop(registry));
    serde_json::json!({"mode":"phases","depth":depth,"fault":fault,"seed":74,"iterations":ITERATIONS,
        "corpus":corpus,"root_name_bytes":root.path().file_name().unwrap().len(),
        "open":opened,"warm":warm,"publication":publication,"refusal":refusal,"sticky":sticky,"drop":dropped,
        "scratch_warm":scratch_warm,"scratch_cleared":true,"original_objects_preserved":true,"outcome_checked":true})
}
fn cycle(depth: usize) -> serde_json::Value {
    let (root, uid, gid, policy, corpus) = prepare(depth);
    let (_, cost) = measure(|| {
        let mut registry =
            open_fixture_registry(root.path(), uid, gid, &policy.trust, &policy.context).unwrap();
        inspect(&mut registry, &policy, depth);
        if depth == 127 {
            registry.refuse(RefusalReason::Operator).unwrap();
            assert_eq!(registry.entries.len(), MAX_GENERATIONS);
            assert!(registry.view().unwrap().refused);
        }
        registry.latch_refusal();
        assert!(registry.is_failed());
        assert!(matches!(
            registry.inspect(&policy.trust, &policy.context),
            Err(AuthorityError::Refused)
        ));
        drop(registry);
    });
    serde_json::json!({"mode":"cycle","depth":depth,"fault":"external","seed":74,"iterations":ITERATIONS,
        "corpus":corpus,"root_name_bytes":root.path().file_name().unwrap().len(),"cycle":cost,"outcome_checked":true})
}
#[test]
fn runtime_cpu_clocks() {
    for id in [libc::CLOCK_MONOTONIC_RAW, libc::CLOCK_THREAD_CPUTIME_ID] {
        assert!(clock(id, true) > 0);
        assert!(clock(id, false) <= clock(id, false));
    }
    let (sum, cost) = measure(|| {
        let mut sum = 0_u64;
        for i in 0..32_768 {
            sum = std::hint::black_box(sum.wrapping_add(i));
        }
        sum
    });
    assert_eq!(sum, (32_767_u64 * 32_768) / 2);
    assert!(cost["cpu_ns"].as_u64().unwrap() > 0);
    assert!(cost["wall_ns"].as_u64().unwrap() > 0);
}
#[test]
fn runtime_cpu_semantics() {
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
    for fault in [
        "external",
        "validation",
        "binding",
        "transition",
        "publication",
    ] {
        assert_eq!(
            phased(1 + usize::from(fault != "external") * 6, fault)["outcome_checked"],
            true
        );
    }
    assert_eq!(cycle(1)["outcome_checked"], true);
}
#[test]
fn runtime_cpu_measurement() {
    const { assert!(!cfg!(feature = "fixed-leaf-alloc-probe")) };
    let empty: Vec<_> = (0..8).map(|_| measure(|| ()).1).collect();
    println!(
        "HC074-RUNTIME-CPU-CLOCKS {}",
        serde_json::json!({"wall_resolution_ns":clock(libc::CLOCK_MONOTONIC_RAW,true),
        "cpu_resolution_ns":clock(libc::CLOCK_THREAD_CPUTIME_ID,true),"empty":empty})
    );
    for depth in [1, 7, 31, 127] {
        println!("HC074-RUNTIME-CPU {}", phased(depth, "external"));
    }
    for fault in ["validation", "binding", "transition", "publication"] {
        println!("HC074-RUNTIME-CPU {}", phased(7, fault));
    }
    for depth in [1, 7, 31, 127] {
        println!("HC074-RUNTIME-CPU {}", cycle(depth));
    }
}
