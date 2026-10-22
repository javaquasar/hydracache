//! Logical append/state/reopen mechanics only; no publication or filesystem.
use super::*;
impl<L: Storage> Journal<L> {
    pub fn outer_failure(&mut self, site: Event) -> Result<(), AuthorityError> {
        if self.failed {
            return Err(AuthorityError::Refused);
        }
        let result = (|| {
            self.revalidate(Fault::default())?;
            gate(
                site,
                Fault {
                    gate: Some(site),
                    leaf: None,
                },
            )
        })();
        if result.is_err() {
            self.latch();
        }
        result
    }
    pub fn append(&mut self, fail_after: bool) -> Result<(), AuthorityError> {
        if self.failed {
            return Err(AuthorityError::Refused);
        }
        self.revalidate(Fault::default())?;
        if self.entries.len() == 128 {
            self.latch();
            return Err(AuthorityError::Budget);
        }
        crate::begin_round();
        self.add_entry();
        crate::begin_round();
        if fail_after {
            self.latch();
            return Err(AuthorityError::Invalid);
        }
        self.revalidate(Fault::default())
    }
    pub fn operator_refuse(&mut self) -> Result<(), AuthorityError> {
        if self.refused {
            self.latch();
            return Err(AuthorityError::Invalid);
        }
        self.append(false)?;
        self.refused = true;
        self.evict();
        Ok(())
    }
    pub fn operator_advance(&mut self, epoch: u64) -> Result<(), AuthorityError> {
        if !self.refused || epoch <= self.epoch {
            self.latch();
            return Err(AuthorityError::Invalid);
        }
        self.append(false)?;
        self.epoch = epoch;
        self.refused = false;
        Ok(())
    }
}
