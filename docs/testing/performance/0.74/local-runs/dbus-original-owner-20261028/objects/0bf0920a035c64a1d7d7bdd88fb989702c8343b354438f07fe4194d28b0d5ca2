//! Crate-private kernel adapter. A pinned root is NOT host admission.
use super::*;

#[cfg(test)]
pub(super) fn queued_ack_for_fixture(peer: RawFd, enroll: &[u8; PACKET_BYTES]) {
    let mut bytes = [0u8; PACKET_BYTES];
    // No ancillary buffer: a peek must not install or retain any writer/FD alias.
    // The real notice() separately authenticates credentials and ancillary shape.
    let length = unsafe {
        libc::recv(
            peer,
            bytes.as_mut_ptr().cast(),
            bytes.len(),
            libc::MSG_PEEK | libc::MSG_TRUNC | libc::MSG_DONTWAIT,
        )
    };
    let mut expected = *enroll;
    expected[8] = 2;
    assert_eq!(length, PACKET_BYTES as isize);
    assert_eq!(
        bytes, expected,
        "cut requires a queued Armed ACK, not refusal/readability"
    );
}

pub(crate) struct RootSource {
    root: PinnedRoot,
}

#[cfg(test)]
impl BoundGuardians<'_> {
    pub(crate) fn owned_fixture_pins(&self) -> [File; 2] {
        self.pair
            .guards
            .each_ref()
            .map(|guard| guard.pin.try_clone().unwrap())
    }
}
impl RootSource {
    pub(crate) fn pin(parent: BorrowedFd<'_>, name: &CStr) -> io::Result<Self> {
        Ok(Self {
            root: PinnedRoot::pin_empty(parent, name)?,
        })
    }
    pub(crate) fn identity(&self) -> [u64; 2] {
        [self.root.identity.device, self.root.identity.inode]
    }
    pub(crate) fn prepare(
        &mut self,
        fence: BorrowedFd<'_>,
        alarm: BorrowedFd<'_>,
        binding: RootBindingClaims,
    ) -> io::Result<BoundGuardians<'_>> {
        Ok(BoundGuardians {
            pair: RootPair::spawn(
                fence,
                alarm,
                &mut self.root,
                binding,
                #[cfg(test)]
                0,
            )?,
        })
    }
}
pub(crate) struct BoundGuardians<'root> {
    pair: RootPair<'root>,
}
impl BoundGuardians<'_> {
    pub(crate) fn observe(&mut self) -> io::Result<()> {
        let result = (|| {
            if self.pair.refused {
                return Err(refused());
            }
            self.pair.live()?;
            let binding = super::super::super::inspect_packet(&self.pair.frame.enroll)
                .map_err(|_| refused())?
                .binding;
            inspect_timer(&self.pair.alarm, binding.deadline_boottime_ns)?;
            let root = self.pair.root.observe()?;
            if root.frozen || (!self.pair.armed && root.populated) {
                return Err(refused());
            }
            Ok(())
        })();
        if result.is_err() {
            self.pair.fail();
        }
        result
    }
    pub(crate) fn enroll_once(&mut self) -> io::Result<()> {
        self.pair.enroll_once()
    }
    pub(crate) fn refuse(&mut self) {
        self.pair.fail();
    }
    #[cfg(test)]
    pub(crate) fn cleanup_for_fixture(&mut self) {
        self.pair.cleanup_for_fixture();
    }
}

#[path = "diagnostic_root_placement_binding.rs"]
mod placement;

#[path = "diagnostic_root_guardian_projection.rs"]
mod projection;
