//! One original owner through effect-local working and closing live rounds.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputJoinStage {
    Starting(CollectorStartingStage),
    Pipes(PipePreparationStage),
    Sinks(SinkPreparationStage),
}

impl OriginalCollectorStarting<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn joined_round(&mut self) -> Result<(), CollectorStartingError> {
        if !self.acknowledged || self.is_refused() {
            self.original.refuse();
            return Err(OriginalStartConsentError::Refused.into());
        }
        self.original.round(|_| {}).map_err(Into::into)
    }
    fn publish_joined(
        &mut self,
        checkpoint: &mut impl FnMut(OutputJoinStage) -> Result<(), OriginalSinkError>,
    ) -> Result<(), OriginalSinkError> {
        let r = &self
            .original
            .original
            .prepared
            .original
            .original
            .reservation;
        let raw = encode_collector_starting_record(
            &r.request_raw,
            &r.raw,
            self.original.original.packet.bytes(),
        )
        .map_err(CollectorStartingError::from)
        .map_err(OriginalPipeError::from)?;
        self.original
            .original
            .prepared
            .original
            .original
            .reservation
            .storage
            .begin_starting()?;
        for at in starting_storage::STAGES {
            // No successful round is reused across an effect or checkpoint.
            self.original
                .round(|_| {})
                .map_err(CollectorStartingError::from)
                .map_err(OriginalPipeError::from)?;
            self.original
                .original
                .prepared
                .original
                .original
                .reservation
                .storage
                .starting_step(at, &raw)?;
            checkpoint(OutputJoinStage::Starting(at))?;
            self.original
                .round(|_| {})
                .map_err(CollectorStartingError::from)
                .map_err(OriginalPipeError::from)?;
        }
        self.original
            .round(|_| {})
            .map_err(CollectorStartingError::from)
            .map_err(OriginalPipeError::from)?;
        self.original
            .original
            .prepared
            .original
            .original
            .reservation
            .storage
            .acknowledge_starting()?;
        self.acknowledged = true;
        Ok(())
    }
}
impl OriginalCollectorPipes<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn prepare_joined(
        &mut self,
        checkpoint: &mut impl FnMut(OutputJoinStage) -> Result<(), OriginalSinkError>,
    ) -> Result<(), OriginalSinkError> {
        self.starting
            .joined_round()
            .map_err(OriginalPipeError::from)?;
        checkpoint(OutputJoinStage::Pipes(PipePreparationStage::BeforeCreate))?;
        for index in 0..2 {
            self.inspect(false)?;
            self.starting
                .joined_round()
                .map_err(OriginalPipeError::from)?;
            self.inspect(false)?;
            self.pipes[index] = Some(Pair::open()?);
            checkpoint(OutputJoinStage::Pipes(if index == 0 {
                PipePreparationStage::StdoutCreated
            } else {
                PipePreparationStage::StderrCreated
            }))?;
            self.pipes[index].as_mut().unwrap().initialize()?;
            self.inspect(false)?;
            self.starting
                .joined_round()
                .map_err(OriginalPipeError::from)?;
            self.inspect(false)?;
            checkpoint(OutputJoinStage::Pipes(if index == 0 {
                PipePreparationStage::StdoutReady
            } else {
                PipePreparationStage::StderrReady
            }))?;
        }
        self.inspect(true)?;
        checkpoint(OutputJoinStage::Pipes(PipePreparationStage::Acknowledging))?;
        self.starting
            .joined_round()
            .map_err(OriginalPipeError::from)?;
        self.inspect(true)?;
        self.acknowledged = true;
        Ok(())
    }
    pub(super) fn join_revalidate(&mut self) -> Result<(), OriginalPipeError> {
        if self.is_refused() || !self.acknowledged {
            self.refuse();
            return Err(OriginalPipeError::Refused);
        }
        let result = (|| {
            self.inspect(true)?;
            self.starting.joined_round()?;
            self.inspect(true)
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalFramedConsent<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    /// Consume the admitted originals; no FD, path, time or start capability is supplied.
    /// No process, writer handoff, completion or recovered acknowledgement is created.
    pub fn prepare_original_outputs(
        self,
    ) -> Result<
        OriginalCollectorSinks<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalSinkFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.join_outputs_with(0, 0, |_| Ok(()))
    }
    #[cfg(test)]
    pub(crate) fn prepare_original_outputs_for_test(
        self,
        checkpoint: impl FnMut(OutputJoinStage) -> Result<(), OriginalSinkError>,
    ) -> Result<
        OriginalCollectorSinks<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalSinkFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        assert!(uid > 0 && gid > 0, "modeled output join remains nonroot");
        self.join_outputs_with(uid, gid, checkpoint)
    }
    fn join_outputs_with(
        self,
        uid: u32,
        gid: u32,
        mut checkpoint: impl FnMut(OutputJoinStage) -> Result<(), OriginalSinkError>,
    ) -> Result<
        OriginalCollectorSinks<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalSinkFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = OriginalCollectorSinks {
            sinks: [None, None],
            directory: None,
            root: None,
            pipes: OriginalCollectorPipes {
                pipes: [None, None],
                starting: OriginalCollectorStarting {
                    original: self,
                    acknowledged: false,
                },
                acknowledged: false,
                refused: false,
            },
            name: String::new(),
            uid,
            gid,
            acknowledged: false,
            refused: false,
        };
        let result = (|| {
            if unsafe { libc::geteuid() } != uid || unsafe { libc::getegid() } != gid {
                return Err(OriginalSinkError::Privilege);
            }
            // The entire uncertain chain is already retained before the first effect.
            held.pipes.starting.publish_joined(&mut checkpoint)?;
            held.pipes.prepare_joined(&mut checkpoint)?;
            held.prepare(|at| checkpoint(OutputJoinStage::Sinks(at)))
        })();
        match result {
            Ok(()) => Ok(held),
            Err(error) => {
                held.refuse();
                Err(OriginalSinkFailure {
                    error,
                    held: Box::new(held),
                })
            }
        }
    }
}
