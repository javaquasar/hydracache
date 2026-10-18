//! Test-only cuts: no detach, FD or process accessor exists in ordinary builds.
use super::*;
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_barred_fixture_for_test<'held>(
        &'held mut self,
        fault: u8,
        checkpoint: impl FnOnce(
            &mut OriginalBarredCustody<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ),
    ) -> Result<
        OriginalBarredCustody<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        BarredCustodyFailure<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        assert_ne!(unsafe { libc::geteuid() }, 0);
        self.prepare_barred_with(true, fault, checkpoint)
    }
}
impl OriginalBarredCustody<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn deadline_for_test(&self) -> u64 {
        self.original.original.alarm.as_ref().unwrap().deadline
    }
    pub(crate) fn assert_original_for_test(&self) {
        let kernel = self.kernel.as_ref().unwrap();
        kernel.alive().unwrap();
        let mut actual = std::fs::read_dir(format!("/proc/{}/fd", kernel.pid))
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
        let mut expected = vec![
            self.inventory.as_ref().unwrap().fds[0],
            self.inventory.as_ref().unwrap().fds[1],
            kernel.child_peer,
        ];
        expected.sort_unstable();
        assert_eq!(actual, expected);
        let inventory = self.inventory.as_ref().unwrap();
        let guardian = inventory.roles.guardian;
        let writers = inventory.roles.writers;
        assert!(actual.iter().all(|fd| !writers.contains(fd)));
        assert!(guardian
            .iter()
            .filter(|fd| !expected.contains(fd))
            .all(|fd| !actual.contains(fd)));
    }
    pub(crate) fn origins_for_test(&self) -> [u64; 6] {
        self.original.origins_for_test()
    }
    pub(crate) fn inherited_for_test(&self) -> [RawFd; 10] {
        self.inventory.as_ref().unwrap().fds
    }
    pub(crate) fn tamper_for_test(&mut self, kind: &str) {
        self.original.tamper_for_test(kind);
    }
    pub(crate) fn kill_for_test(&mut self) {
        self.kernel.as_mut().unwrap().stop().unwrap();
    }
    pub(crate) fn detach_for_test(mut self) -> KernelCustodian {
        self.kernel.take().unwrap()
    }
    pub(crate) fn pinned_for_test(&self) -> File {
        self.kernel.as_ref().unwrap().pinned.try_clone().unwrap()
    }
}
impl BarredCustodyFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn origins_for_test(&self) -> [u64; 6] {
        self.held.original.origins_for_test()
    }
    pub(crate) fn reason_for_test(&self) -> String {
        format!("{:?}", self.error)
    }
    pub(crate) fn refused_for_test(&self) -> bool {
        self.held.is_refused()
    }
    pub(crate) fn retry_for_test(&mut self) -> Result<(), BarredCustodyError> {
        self.held.revalidate()
    }
}
impl KernelCustodian {
    pub(crate) fn assert_retained_for_test(&self) {
        self.alive().unwrap();
    }
    pub(crate) fn disconnect_for_test(&self) {
        assert_eq!(
            unsafe { libc::shutdown(self.peer.as_raw_fd(), libc::SHUT_RDWR) },
            0
        );
    }
    pub(crate) fn wait_expired_for_test(&self) {
        let mut poll = [
            libc::pollfd {
                fd: self.peer.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: self.pinned.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        assert_eq!(unsafe { libc::poll(poll.as_mut_ptr(), 2, -1) }, 1);
        assert_eq!(poll[1].revents, 0);
        let mut byte = 0u8;
        assert_eq!(
            unsafe {
                libc::recv(
                    self.peer.as_raw_fd(),
                    (&mut byte as *mut u8).cast(),
                    1,
                    libc::MSG_TRUNC,
                )
            },
            1
        );
        assert_eq!(byte, b'E');
        self.alive().unwrap();
    }
    pub(crate) fn finish_for_test(&mut self) {
        self.stop().unwrap();
    }
}
#[test]
fn original_barred_clone_arguments_pin_atomically_without_shared_state() {
    let mut pidfd = -1;
    let args = clone_arguments(&mut pidfd);
    assert_eq!(std::mem::size_of_val(&args), 88);
    assert_eq!(args[0], libc::CLONE_PIDFD as u64);
    assert_eq!(args[1], (&mut pidfd as *mut RawFd) as u64);
    assert_eq!(args[4], libc::SIGCHLD as u64);
    assert!(args
        .iter()
        .enumerate()
        .filter(|(i, _)| ![0, 1, 4].contains(i))
        .all(|(_, value)| *value == 0));
}
