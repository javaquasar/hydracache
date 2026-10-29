//! No process/FD export or detach is available outside cfg(test).
use super::*;
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_collector_fixture_for_test<'held>(
        &'held mut self,
        fault: u8,
        checkpoint: impl FnOnce(
            &mut OriginalBarredCollector<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ),
    ) -> Result<
        OriginalBarredCollector<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        BarredCollectorFailure<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        assert_ne!(unsafe { libc::geteuid() }, 0);
        self.prepare_barred_collector_with(true, fault, checkpoint)
    }
}
fn assert_fds(pid: libc::pid_t, mut expected: Vec<RawFd>) {
    let mut actual = std::fs::read_dir(format!("/proc/{pid}/fd"))
        .unwrap()
        .map(|fd| {
            fd.unwrap()
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
impl OriginalBarredCollector<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn assert_original_for_test(&self) {
        self.custody.assert_original_for_test();
        let kernel = self.collector.as_ref().unwrap();
        kernel.alive().unwrap();
        let fds = self.custody.inventory.as_ref().unwrap().fds;
        let mut expected = fds[..6].to_vec();
        expected.push(kernel.child_peer);
        assert_fds(kernel.pid, expected);
        for fd in &fds[..6] {
            assert_eq!(
                std::fs::read_link(format!("/proc/{}/fd/{fd}", kernel.pid)).unwrap(),
                std::fs::read_link(format!("/proc/self/fd/{fd}")).unwrap()
            );
        }
        for fd in &fds[4..6] {
            assert_eq!(
                unsafe { libc::fcntl(*fd, libc::F_GETFL) } & libc::O_ACCMODE,
                libc::O_RDWR
            );
            assert_eq!(unsafe { libc::lseek(*fd, 0, libc::SEEK_CUR) }, 0);
        }
    }
    pub(crate) fn origins_for_test(&self) -> [u64; 6] {
        self.custody.origins_for_test()
    }
    pub(crate) fn pinned_for_test(&self) -> [File; 2] {
        [
            self.collector.as_ref().unwrap().pinned.try_clone().unwrap(),
            self.custody.pinned_for_test(),
        ]
    }
    pub(crate) fn tamper_for_test(&mut self, kind: &str) {
        match kind {
            "collector-kill" => self.collector.as_mut().unwrap().stop().unwrap(),
            "custodian-kill" => self.custody.kill_for_test(),
            other => self.custody.tamper_for_test(other),
        }
    }
    pub(crate) fn detach_for_test(mut self) -> ProcessesForTest {
        ProcessesForTest {
            collector: self.collector.take().unwrap(),
            custody: self.custody.kernel.take().unwrap(),
            fds: self.custody.inventory.as_ref().unwrap().fds,
        }
    }
}
pub(crate) struct ProcessesForTest {
    collector: KernelCollector,
    custody: KernelCustodian,
    fds: [RawFd; 10],
}
impl ProcessesForTest {
    pub(crate) fn quarantine_for_test(&self, case: &str) {
        if case == "peer-activity" {
            assert_eq!(
                unsafe {
                    libc::send(
                        self.collector.peer.as_raw_fd(),
                        b"X".as_ptr().cast(),
                        1,
                        libc::MSG_NOSIGNAL,
                    )
                },
                1
            );
        } else if case == "peer-loss" {
            assert_eq!(
                unsafe { libc::shutdown(self.collector.peer.as_raw_fd(), libc::SHUT_RDWR) },
                0
            );
        }
        if case != "peer-loss" {
            let mut poll = libc::pollfd {
                fd: self.collector.peer.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            assert_eq!(unsafe { libc::poll(&mut poll, 1, -1) }, 1);
            let mut byte = 0u8;
            assert_eq!(
                unsafe {
                    libc::recv(
                        self.collector.peer.as_raw_fd(),
                        (&mut byte as *mut u8).cast(),
                        1,
                        libc::MSG_TRUNC,
                    )
                },
                1
            );
            assert_eq!(byte, if case == "expiry" { b'E' } else { b'L' });
        }
        // Harness-only fail-safe for observing post-refusal FD closure. It
        // neither authorizes work nor changes an original admission deadline.
        let limit = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            self.collector.alive().unwrap();
            let count = std::fs::read_dir(format!("/proc/{}/fd", self.collector.pid))
                .unwrap()
                .count();
            if count == 3 {
                break;
            }
            assert!(
                std::time::Instant::now() < limit,
                "I/O descriptors not quarantined"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_fds(
            self.collector.pid,
            vec![self.fds[0], self.fds[1], self.collector.child_peer],
        );
    }
    pub(crate) fn kill_custody_for_test(&mut self) {
        self.custody.stop().unwrap();
        self.collector.alive().unwrap();
    }
    pub(crate) fn finish_for_test(&mut self) {
        self.collector.stop().unwrap();
    }
}
impl BarredCollectorFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn reason_for_test(&self) -> String {
        format!("{:?}", self.error)
    }
    pub(crate) fn retry_for_test(&mut self) -> Result<(), BarredCustodyError> {
        self.held.revalidate()
    }
    pub(crate) fn refused_for_test(&self) -> bool {
        self.held.custody.is_refused()
    }
}
