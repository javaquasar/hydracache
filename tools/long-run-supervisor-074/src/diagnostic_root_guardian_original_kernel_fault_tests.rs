//! Real kernel adapter only, not an original manager-channel fixture.
use super::*;
use crate::diagnostic_two_phase_launch::root_enrollment::{BoundGuardians, RootSource};
struct Cleanup<'a>(BoundGuardians<'a>);
impl Drop for Cleanup<'_> {
    fn drop(&mut self) {
        self.0.cleanup_for_fixture();
    }
}
#[test]
#[ignore = "manual isolated local root guardian adapter; no original channel or product"]
fn root_guardian_original_kernel_controls() {
    assert_eq!(
        std::env::var("HC074_ROOT_JOIN_FIXTURE").as_deref(),
        Ok("local-only-v1")
    );
    assert_eq!(unsafe { libc::geteuid() }, 0);
    for case in 0..4 {
        let mut fixture = Fixture::new();
        let mut root = RootSource::pin(fixture.parent.as_fd(), &fixture.name).unwrap();
        let identity = root.identity();
        let deadline = now()
            + if case == 3 {
                2_000_000_000
            } else {
                30_000_000_000
            };
        let timer = alarm(deadline);
        let binding = RootBindingClaims {
            attempt_sha256: [7; 32],
            original_context_sha256: [8; 32],
            root_device: identity[0],
            root_inode: identity[1],
            deadline_boottime_ns: deadline,
        };
        let mut controls = Cleanup(
            root.prepare(fixture.fence.as_fd(), timer.as_fd(), binding)
                .unwrap(),
        );
        controls.0.observe().unwrap();
        controls.0.enroll_once().unwrap();
        let direct = target(&mut fixture, false, None);
        let nested = target(&mut fixture, true, None);
        controls.0.observe().unwrap();
        match case {
            0 => controls.0.refuse(),
            1 => assert!(controls.0.enroll_once().is_err()),
            2 => {
                let mode = std::fs::metadata(&fixture.path).unwrap().permissions();
                std::fs::set_permissions(&fixture.path, std::fs::Permissions::from_mode(0o777))
                    .unwrap();
                assert!(controls.0.observe().is_err());
                std::fs::set_permissions(&fixture.path, mode).unwrap();
            }
            3 => {
                elapsed(deadline);
                assert!(controls.0.observe().is_err());
            }
            _ => unreachable!(),
        }
        assert!(controls.0.observe().is_err());
        assert!(controls.0.enroll_once().is_err());
        fixture.children[direct].killed();
        fixture.children[nested].killed();
        locked(&fixture);
        drop(controls);
        println!("HC074-ROOT-JOIN-KERNEL case={case} seed=0x074c2023 actual_sigkill=true original_frame=false completion=false");
    }
}
