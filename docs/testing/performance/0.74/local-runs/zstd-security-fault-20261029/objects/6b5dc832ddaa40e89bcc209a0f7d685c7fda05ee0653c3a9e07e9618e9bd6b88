//! Fixture-only read/writer/pidfd aliases. No ordinary export or completion.
use super::*;
use std::os::fd::{AsRawFd, FromRawFd};
impl OwnedRootGuardians<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn fixture_streams(&self) -> [std::fs::File; 4] {
        let frame = &self.owner.original.original.original;
        let fds = frame.inventory.as_ref().unwrap().fds;
        [fds[2], fds[3], fds[6], fds[7]].map(|fd| {
            let alias = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 3) };
            assert!(alias >= 0);
            unsafe { std::fs::File::from_raw_fd(alias) }
        })
    }
    pub(crate) fn fixture_guardians(&self) -> [std::fs::File; 2] {
        self.controls.owned_fixture_pins()
    }
    pub(crate) fn fixture_custodians(&self) -> [std::fs::File; 2] {
        let frame = &self.owner.original.original.original;
        [
            frame
                .collector
                .as_ref()
                .unwrap()
                .pinned
                .try_clone()
                .unwrap(),
            frame
                .custodian
                .as_ref()
                .unwrap()
                .pinned
                .try_clone()
                .unwrap(),
        ]
    }
    pub(crate) fn fixture_origins(&self) -> ([u64; 4], [u64; 4]) {
        let source = &self.owner.binding.as_ref().unwrap().source;
        (source.origins, source.bounds)
    }
}
#[test]
fn owned_root_manager_test_api_remains_test_only() {
    fn signature(value: &OwnedRootGuardians<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        let [out, err, writer, other] = value.fixture_streams();
        assert_ne!(out.as_raw_fd(), writer.as_raw_fd());
        assert_ne!(err.as_raw_fd(), other.as_raw_fd());
    }
    let _signature = signature;
}
