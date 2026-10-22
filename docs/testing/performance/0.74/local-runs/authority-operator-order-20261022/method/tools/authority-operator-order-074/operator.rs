//! Restricted change/publish framing, not real Record serialization or durability.
use super::*;

#[derive(Clone, Copy)]
pub(crate) enum Operation {
    Refuse,
    Advance(u64),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cut {
    BeforeAppend,
    AfterAppend,
}
#[derive(Clone, Copy, Default)]
pub(crate) struct Failures {
    pub change: Fault,
    pub publish: Fault,
    pub post: Fault,
    pub cut: Option<Cut>,
}
pub(crate) enum OperatorError {
    Authority(AuthorityError),
    Transition,
}
impl From<AuthorityError> for OperatorError {
    fn from(error: AuthorityError) -> Self {
        Self::Authority(error)
    }
}

impl<L: Storage> Journal<L> {
    pub fn candidate_refuse(&mut self, faults: Failures) -> Result<(), OperatorError> {
        self.candidate_change(Operation::Refuse, faults)
    }
    pub fn candidate_advance(&mut self, epoch: u64, faults: Failures) -> Result<(), OperatorError> {
        self.candidate_change(Operation::Advance(epoch), faults)
    }
    fn candidate_change(
        &mut self,
        operation: Operation,
        faults: Failures,
    ) -> Result<(), OperatorError> {
        self.revalidate(faults.change)?;
        let next = (|| {
            let next = match operation {
                Operation::Refuse if !self.refused => (true, self.epoch),
                Operation::Advance(epoch) if self.refused && epoch > self.epoch => (false, epoch),
                _ => return Err(OperatorError::Transition),
            };
            if self.entries.len() >= 128 {
                return Err(AuthorityError::Budget.into());
            }
            Ok(next)
        })();
        match next {
            Ok((refused, epoch)) => self.candidate_publish(refused, epoch, faults),
            Err(error) => {
                self.latch();
                Err(error)
            }
        }
    }
    fn candidate_publish(
        &mut self,
        refused: bool,
        epoch: u64,
        faults: Failures,
    ) -> Result<(), OperatorError> {
        if self.failed {
            return Err(AuthorityError::Refused.into());
        }
        let result = (|| {
            self.revalidate(faults.publish)?;
            if faults.cut == Some(Cut::BeforeAppend) {
                gate(
                    Event::Publication,
                    Fault {
                        gate: Some(Event::Publication),
                        leaf: None,
                    },
                )?;
            }
            self.add_entry();
            self.refused = refused;
            self.epoch = epoch;
            if faults.cut == Some(Cut::AfterAppend) {
                gate(
                    Event::Publication,
                    Fault {
                        gate: Some(Event::Publication),
                        leaf: None,
                    },
                )?;
            }
            self.revalidate(faults.post)?;
            Ok(())
        })();
        if result.is_err() {
            self.latch();
        }
        // A logical data refusal may evict storage, but never marks a healthy
        // journal failed or grants an original signed guard a new epoch.
        if result.is_ok() && refused {
            self.evict();
        }
        result
    }
}
