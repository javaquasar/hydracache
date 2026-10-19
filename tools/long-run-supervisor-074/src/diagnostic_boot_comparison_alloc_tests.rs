//! Paired original live validation; Rust allocation requests, never admission.
use super::*;

fn reference_revalidate(origin: &Origin) -> Result<(), KernelError> {
    let named = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/proc")?;
    for file in [&named, &origin.root] {
        filesystem(file, 0x9fa0)?;
        let m = file.metadata()?;
        if (m.dev(), m.ino()) != origin.id || boot(file)? != origin.boot {
            return Err(kernel_invalid!("proc-origin-or-boot"));
        }
    }
    for (index, (file, id)) in origin.namespaces.iter().enumerate() {
        filesystem(file, 0x6e736673)?;
        let m = file.metadata()?;
        if (m.dev(), m.ino()) != *id || namespace(NAMES[index])?.1 != *id {
            return Err(kernel_invalid!("namespace-origin"));
        }
    }
    Ok(())
}
fn reference_sample(origin: &Origin) -> Result<KernelTimePoint, KernelError> {
    reference_revalidate(origin)?;
    let boottime = clock(libc::CLOCK_BOOTTIME)?;
    let realtime_ns = clock(libc::CLOCK_REALTIME)?;
    reference_revalidate(origin)?;
    Ok(KernelTimePoint {
        boot: origin.boot.clone(),
        boottime,
        realtime_ns,
    })
}
#[test]
fn boot_comparison_allocation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let origin = Origin::open().unwrap();
    for _ in 0..32 {
        reference_sample(&origin).unwrap();
        origin.sample().unwrap();
    }
    for sample in 0..3 {
        for operation in ["revalidate", "sample"] {
            let run = |candidate| {
                for _ in 0..1024 {
                    if operation == "revalidate" {
                        if candidate {
                            origin.revalidate().unwrap();
                        } else {
                            reference_revalidate(&origin).unwrap();
                        }
                    } else {
                        let point = if candidate {
                            origin.sample().unwrap()
                        } else {
                            reference_sample(&origin).unwrap()
                        };
                        assert_eq!(point.boot_id(), origin.boot);
                        std::hint::black_box(point);
                    }
                }
            };
            let reference = capture(|| run(false)).1;
            let candidate = capture(|| run(true)).1;
            assert_eq!(reference.reallocations, 0);
            assert_eq!(candidate.reallocations, 0);
            assert!(candidate.allocations <= reference.allocations);
            println!(
                "HC074-BOOT-COMPARE-ALLOC {}",
                serde_json::json!({
                    "sample":sample,"operation":operation,"iterations":1024,
                    "reference":reference,"candidate":candidate,
                    "rust_requests_only":true,"product":false
                })
            );
        }
    }
    origin.revalidate().unwrap();
}
