//! Fixture-only observations; no ordinary FD/process export.
use super::*;
fn assert_fds(pid: libc::pid_t, mut expected: Vec<RawFd>) {
    let mut actual = std::fs::read_dir(format!("/proc/{pid}/fd"))
        .unwrap()
        .map(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .parse::<RawFd>()
                .unwrap()
        })
        .collect::<Vec<_>>();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCollectorSinks<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_enrollment_fixture_for_test(
        self,
        path: &std::path::Path,
        fault: u8,
        mut checkpoint: impl FnMut(
            &'static str,
            &mut OriginalEnrollmentFrame<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ),
    ) -> Result<
        OriginalEnrollmentFrame<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        EnrollmentFrameFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        assert_ne!(unsafe { libc::geteuid() }, 0);
        assert_ne!(unsafe { libc::getegid() }, 0);
        self.prepare_enrollment_frame_with(Some(path), fault, &mut checkpoint)
    }
}
impl OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn deadline_for_test(&self) -> u64 {
        self.original.original.alarm.as_ref().unwrap().deadline
    }
    pub(crate) fn origins_for_test(&self) -> [u64; 6] {
        self.original.origins_for_test()
    }
    pub(crate) fn offsets_for_test(&self) -> [u64; 3] {
        self.original.offsets_for_test()
    }
    pub(crate) fn pinned_for_test(&self) -> [File; 2] {
        [
            self.collector.as_ref().unwrap().pinned.try_clone().unwrap(),
            self.custodian.as_ref().unwrap().pinned.try_clone().unwrap(),
        ]
    }
    pub(crate) fn assert_original_for_test(&self) {
        let fds = self.inventory.as_ref().unwrap().fds;
        let collector = self.collector.as_ref().unwrap();
        let custodian = self.custodian.as_ref().unwrap();
        collector.alive().unwrap();
        custodian.alive().unwrap();
        let mut expected = fds[..6].to_vec();
        expected.push(collector.child_peer);
        assert_fds(collector.pid, expected);
        assert_fds(custodian.pid, vec![fds[0], fds[1], custodian.child_peer]);
        for (pid, count) in [(collector.pid, 6), (custodian.pid, 2)] {
            for fd in &fds[..count] {
                assert_eq!(
                    std::fs::read_link(format!("/proc/{pid}/fd/{fd}")).unwrap(),
                    std::fs::read_link(format!("/proc/self/fd/{fd}")).unwrap()
                );
            }
        }
        for fd in &fds[4..6] {
            assert_eq!(
                unsafe { libc::fcntl(*fd, libc::F_GETFL) } & libc::O_ACCMODE,
                libc::O_RDWR
            );
            assert_eq!(unsafe { libc::lseek(*fd, 0, libc::SEEK_CUR) }, 0);
        }
    }
    pub(crate) fn tamper_for_test(&mut self, kind: &str) {
        match kind {
            "collector-kill" => self.collector.as_mut().unwrap().stop().unwrap(),
            "custodian-kill" => self.custodian.as_mut().unwrap().stop().unwrap(),
            other => self.original.tamper_for_test(other),
        }
    }
}
impl EnrollmentFrameFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn origins_for_test(&self) -> [u64; 6] {
        self.held.origins_for_test()
    }
    pub(crate) fn assert_lost_b_for_test(&self) {
        let kernel = self.held.collector.as_ref().unwrap();
        let mut poll = [
            libc::pollfd {
                fd: kernel.peer.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: kernel.pinned.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        // Post-refusal fail-safe only; this cannot extend an admission window.
        assert_eq!(unsafe { libc::poll(poll.as_mut_ptr(), 2, 5000) }, 1);
        assert_eq!(poll[0].revents, libc::POLLIN);
        assert_eq!(poll[1].revents, 0);
        let mut byte = 0u8;
        assert_eq!(
            unsafe {
                libc::recv(
                    kernel.peer.as_raw_fd(),
                    (&mut byte as *mut u8).cast(),
                    1,
                    libc::MSG_TRUNC,
                )
            },
            1
        );
        assert_eq!(byte, b'E');
        kernel.alive().unwrap();
        let custodian = self.held.custodian.as_ref().unwrap();
        custodian.alive().unwrap();
        let fds = self.held.inventory.as_ref().unwrap().fds;
        assert_fds(kernel.pid, vec![fds[0], fds[1], kernel.child_peer]);
        assert_fds(custodian.pid, vec![fds[0], fds[1], custodian.child_peer]);
    }
    pub(crate) fn reason_for_test(&self) -> String {
        format!("{:?}", self.error)
    }
    pub(crate) fn refused_for_test(&self) -> bool {
        self.held.original.is_refused()
    }
    pub(crate) fn retry_for_test(&mut self) -> Result<(), BarredCustodyError> {
        self.held.revalidate()
    }
}
