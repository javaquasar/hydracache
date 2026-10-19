//! Frozen old leaf references; no clock, admission or observation cache.
use super::*;

fn reference_namespace(name: &str) -> Result<(File, (u64, u64)), KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Namespace,
    );
    // These fixed kernel magic links must be followed, then checked as nsfs.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(format!("/proc/thread-self/ns/{name}"))?;
    filesystem(&file, 0x6e736673)?;
    let m = file.metadata()?;
    Ok((file, (m.dev(), m.ino())))
}
fn reference_boot(root: &File) -> Result<String, KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Boot,
    );
    let sys = child(root, "sys", true)?;
    let kernel = child(&sys, "kernel", true)?;
    let random = child(&kernel, "random", true)?;
    let file = child(&random, "boot_id", false)?;
    let raw = boot_read(|bytes| file.read_at(bytes, 0))?;
    // As before, close the proc document before decoding the owned bytes.
    drop(file);
    boot_value(raw)
}
fn errno<T>(value: Result<T, KernelError>) -> String {
    match value {
        Ok(_) => "ok".into(),
        Err(KernelError::Io(e)) => format!("io:{:?}:{:?}", e.kind(), e.raw_os_error()),
        Err(KernelError::Invalid) => "invalid".into(),
        Err(e) => panic!("unexpected: {e:?}"),
    }
}
#[test]
fn fixed_leaf_path_differential_identity_flags_and_unknown_names() {
    let origin = Origin::open().unwrap();
    assert_eq!(
        boot(&origin.root).unwrap(),
        reference_boot(&origin.root).unwrap()
    );
    for name in NAMES {
        let (a, aid) = namespace(name).unwrap();
        let (b, bid) = reference_namespace(name).unwrap();
        assert_eq!(aid, bid);
        for op in [libc::F_GETFL, libc::F_GETFD] {
            assert_eq!(unsafe { libc::fcntl(a.as_raw_fd(), op) }, unsafe {
                libc::fcntl(b.as_raw_fd(), op)
            });
        }
    }
    for name in ["", "no-such-namespace", "user\0suffix", "../stat"] {
        assert_eq!(
            errno(namespace(name)),
            errno(reference_namespace(name)),
            "{name:?}"
        );
    }
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(
        errno(boot(&File::open(directory.path()).unwrap())),
        errno(reference_boot(&File::open(directory.path()).unwrap()))
    );
    assert_eq!(
        errno(boot(&tempfile::tempfile().unwrap())),
        errno(reference_boot(&tempfile::tempfile().unwrap()))
    );
}
#[test]
fn fixed_leaf_path_preserves_original_trace_counts_and_sticky_drift() {
    use crate::diagnostic_ipc::validation_trace::kernel_walk::{capture, Kind};
    let mut origin = Origin::open().unwrap();
    let (result, report) = capture(|| origin.revalidate());
    result.unwrap();
    assert_eq!(report.nodes[Kind::Origin as usize].calls, 1);
    assert_eq!(report.nodes[Kind::Boot as usize].calls, 2);
    assert_eq!(report.nodes[Kind::Namespace as usize].calls, 4);
    assert_eq!(report.nodes[Kind::Document as usize].calls, 2);
    origin.namespaces[0].1 .1 += 1;
    assert!(origin.revalidate().is_err());
    assert!(origin.sample().is_err());
}

fn measure_boot(call: fn(&File) -> Result<String, KernelError>, root: &File) -> u128 {
    let call = std::hint::black_box(call);
    let start = std::time::Instant::now();
    for _ in 0..16384 {
        std::hint::black_box(call(root)).unwrap();
    }
    start.elapsed().as_nanos()
}
type NamespaceCall = fn(&str) -> Result<(File, (u64, u64)), KernelError>;
fn measure_namespace(call: NamespaceCall) -> u128 {
    let call = std::hint::black_box(call);
    let start = std::time::Instant::now();
    for i in 0..16384 {
        std::hint::black_box(call(NAMES[i % 4])).unwrap();
    }
    start.elapsed().as_nanos()
}
#[test]
#[ignore = "manual local paired fixed path leaf costs; no admission or product workload"]
fn fixed_leaf_path_cost_abba() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let origin = Origin::open().unwrap();
    for _ in 0..32 {
        assert_eq!(
            boot(&origin.root).unwrap(),
            reference_boot(&origin.root).unwrap()
        );
        for name in NAMES {
            assert_eq!(
                namespace(name).unwrap().1,
                reference_namespace(name).unwrap().1
            );
        }
    }
    for block in 0..15 {
        for boot_leaf in if block % 2 == 0 {
            [true, false]
        } else {
            [false, true]
        } {
            let values = [false, true, true, false, true, false, false, true].map(|candidate| {
                if boot_leaf {
                    measure_boot(if candidate { boot } else { reference_boot }, &origin.root)
                } else {
                    measure_namespace(if candidate {
                        namespace
                    } else {
                        reference_namespace
                    })
                }
            });
            println!(
                "HC074-LEAF-PATH {}",
                serde_json::json!({"block":block,"leaf":if boot_leaf {"boot"} else {"namespace"},"iterations":16384,"order":"ABBA-BAAB","a_ns":(values[0]+values[3]+values[5]+values[6])/4,"b_ns":(values[1]+values[2]+values[4]+values[7])/4,"all_ns":values})
            );
        }
    }
    origin.revalidate().unwrap();
    println!("HC074-LEAF-PATH-CLOSED observations_unchanged=true cached=false product=false");
}
