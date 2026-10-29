//! Pure pathname ownership only. Never opens a descriptor or samples a clock.
use super::*;

const DESCRIPTORS: [i32; 9] = [i32::MIN, -1, 0, 1, 9, 10, 99, 100, i32::MAX];
const ITERATIONS: u64 = 256;

#[test]
fn pidfd_info_path_reference_requests() {
    let source = include_str!("../../src/diagnostic_kernel_origin.rs");
    assert_eq!(
        source
            .matches(".open(format!(\"/proc/thread-self/fdinfo/{}\", pidfd.as_raw_fd()))?;")
            .count()
            + source
                .matches(".open(pidfd_info_path::InfoPath::new(pidfd.as_raw_fd()).as_path())?;")
                .count(),
        1
    );
    for fd in DESCRIPTORS {
        let expected = format!("/proc/thread-self/fdinfo/{fd}");
        let (_, reference) = capture(|| {
            for _ in 0..ITERATIONS {
                let path = format!("/proc/thread-self/fdinfo/{}", std::hint::black_box(fd));
                assert_eq!(std::hint::black_box(path.as_str()), expected);
            }
        });
        assert_eq!(reference.allocations, ITERATIONS);
        assert_eq!(reference.reallocations, 0);
        assert!(reference.requested_bytes > 0);
        println!(
            "HC074-PIDFD-PATH-REFERENCE {}",
            serde_json::json!({"fd":fd,"iterations":ITERATIONS,"reference":reference,"rust_requests_only":true,"profile":false,"server":false,"product":false})
        );
    }
}

#[test]
fn pidfd_info_path_candidate_requests() {
    use crate::diagnostic_kernel_admission::origin::pidfd_info_path::InfoPath;
    for fd in DESCRIPTORS {
        let expected = format!("/proc/thread-self/fdinfo/{fd}");
        let (_, candidate) = capture(|| {
            for _ in 0..ITERATIONS {
                let path = InfoPath::new(std::hint::black_box(fd));
                assert_eq!(
                    std::hint::black_box(path.as_path().as_os_str().as_encoded_bytes()),
                    expected.as_bytes()
                );
            }
        });
        assert_eq!(candidate, Counts::default());
        println!(
            "HC074-PIDFD-PATH-CANDIDATE {}",
            serde_json::json!({"fd":fd,"iterations":ITERATIONS,"candidate":candidate,"rust_requests_only":true,"profile":false,"server":false,"product":false})
        );
    }
}
