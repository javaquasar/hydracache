//! Local single-guardian-failure experiment, never original or cgroup admission.
use super::*;

pub(super) struct GuardPair {
    guardians: [Watchdog; 2],
    origin: Origin,
    consumed: bool,
    armed: bool,
    refused: bool,
}
impl GuardPair {
    pub(super) fn spawn(
        fence: File,
        deadline: u64,
        #[cfg(test)] stop_second_before_ack: bool,
    ) -> io::Result<Self> {
        let origin = Origin::current();
        // Same OFD: closing one alias cannot drop the other guardian's flock.
        // Both alarms use this exact absolute bound; never now()+duration here.
        let mut first = Watchdog::spawn(
            fence.try_clone()?,
            deadline,
            #[cfg(test)]
            false,
        )?;
        first.pending()?;
        let mut second = Watchdog::spawn(
            fence,
            deadline,
            #[cfg(test)]
            stop_second_before_ack,
        )?;
        second.pending()?;
        let pair = Self {
            guardians: [first, second],
            origin,
            consumed: false,
            armed: false,
            refused: false,
        };
        if !pair.live(false) {
            return Err(refused());
        }
        Ok(pair)
    }
    fn live(&self, armed: bool) -> bool {
        Origin::current() == self.origin
            && self.guardians.iter().all(|g| {
                g.origin == self.origin
                    && g.pending
                    && !g.refused
                    && (!armed || g.armed)
                    && !terminal(g.alarm.as_raw_fd())
                    && !terminal(g.pinned.as_raw_fd())
                    && !terminal(g.peer.as_raw_fd())
            })
    }
    pub(super) fn enroll_once(&mut self, target: &File) -> io::Result<()> {
        if self.refused || self.consumed || !self.live(false) {
            self.refused = true;
            return Err(refused());
        }
        // Global uncertainty precedes even the first guardian's first send.
        self.consumed = true;
        for guardian in &mut self.guardians {
            if let Err(error) = guardian.enroll_once(target) {
                self.refused = true;
                return Err(error);
            }
        }
        if !self.live(true) {
            self.refused = true;
            return Err(refused());
        }
        self.armed = true;
        Ok(())
    }
    pub(super) fn ready(&mut self) -> io::Result<()> {
        if self.refused || !self.consumed || !self.armed || !self.live(true) {
            self.refused = true;
            return Err(refused());
        }
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn test_guardians(&self) -> &[Watchdog; 2] {
        &self.guardians
    }
    #[cfg(test)]
    pub(super) fn test_state(&self) -> (bool, bool, bool) {
        (self.consumed, self.armed, self.refused)
    }
}

#[cfg(test)]
#[path = "diagnostic_redundant_watchdog_tests.rs"]
mod tests;
