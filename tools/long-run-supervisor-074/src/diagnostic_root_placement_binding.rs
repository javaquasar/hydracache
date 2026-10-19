//! Actual retained enrollment only. No kernel descriptor or execution is exported.
use super::*;
impl BoundGuardians<'_> {
    pub(crate) fn binding_for_placement(&mut self) -> io::Result<RootBindingClaims> {
        let result = (|| {
            self.observe()?;
            if !self.pair.consumed || !self.pair.armed || self.pair.refused {
                return Err(refused());
            }
            let packet = super::super::super::super::inspect_packet(&self.pair.frame.enroll)
                .map_err(|_| refused())?;
            if packet.kind != super::super::super::super::Kind::Enroll
                || packet.binding.root_device != self.pair.root.identity.device
                || packet.binding.root_inode != self.pair.root.identity.inode
            {
                return Err(refused());
            }
            Ok(packet.binding)
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
}
