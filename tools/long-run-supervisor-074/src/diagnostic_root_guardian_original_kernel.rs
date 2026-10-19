//! Crate-private kernel adapter. A pinned root is NOT host admission.
use super::*;

pub(crate) struct RootSource {
    root: PinnedRoot,
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
