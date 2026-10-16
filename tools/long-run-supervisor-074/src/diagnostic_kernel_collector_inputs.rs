//! Private original peer/time snapshot, not an exported admission capability.
use super::*;

#[path = "diagnostic_kernel_start_consent.rs"]
mod start_consent;

#[path = "diagnostic_kernel_progress.rs"]
mod progress;

fn peer(uid: u32, gid: u32, supplementary: &[u32]) -> DiagnosticPeer {
    let mut gids = supplementary.to_vec();
    gids.push(gid);
    gids.sort_unstable();
    gids.dedup();
    DiagnosticPeer { uid, gids }
}
impl KernelPacket {
    // The previous point is retained by the original reservation owner, never IPC input.
    pub(crate) fn original_reservation_time(
        &mut self,
        previous: Option<&KernelTimePoint>,
    ) -> Result<KernelTimePoint, KernelError> {
        let result = (|| {
            if self.refused {
                return Err(KernelError::Refused);
            }
            self.sender.revalidate(&self.origin)?;
            let now = self.origin.sample()?;
            valid_hold(&self.received, &now)?;
            if let Some(previous) = previous {
                ordered(previous, &now)?;
            }
            self.sender.revalidate(&self.origin)?;
            Ok(now)
        })();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }

    pub(crate) fn original_collector_inputs(
        &mut self,
    ) -> Result<(DiagnosticPeer, u64), KernelError> {
        let result = (|| {
            if self.refused {
                return Err(KernelError::Refused);
            }
            self.sender.revalidate(&self.origin)?;
            let now = self.origin.sample()?;
            valid_hold(&self.received, &now)?;
            let peer = peer(
                self.sender.credentials.uid,
                self.sender.credentials.gid,
                &self.sender.projection.groups,
            );
            self.sender.revalidate(&self.origin)?;
            Ok((peer, now.unix_seconds()))
        })();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_collector_peer_uses_canonical_primary_group_union_without_truncation() {
        for (primary, supplementary, expected) in [
            (4, vec![8, 12], vec![4, 8, 12]),
            (8, vec![4, 8, 12], vec![4, 8, 12]),
            (12, vec![4, 8], vec![4, 8, 12]),
            (4, vec![], vec![4]),
        ] {
            let observed = peer(1000, primary, &supplementary);
            assert_eq!(observed.uid, 1000);
            assert_eq!(observed.gids, expected);
        }
        assert_eq!(peer(1000, 1, &(2..34).collect::<Vec<_>>()).gids.len(), 33);
    }
}
