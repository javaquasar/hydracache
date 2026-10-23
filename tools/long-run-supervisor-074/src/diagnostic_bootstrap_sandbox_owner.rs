//! Consumes original ownership for read-only managed sandbox observation only.
use super::*;
use crate::diagnostic_process::sandbox::{Expected, SandboxRead};
use crate::diagnostic_worker_policy::{SignedWorkerPolicy, WorkerPolicy};
type Managed = (u32, [u8; 16], String);
#[derive(Debug, thiserror::Error)]
pub(crate) enum SandboxError {
    #[error("original installed owner frame refused: {0}")]
    Frame(#[from] BootstrapError),
    #[error("original manager sandbox query refused")]
    Manager,
    #[error("managed sandbox kernel observation refused: {0}")]
    Kernel(#[from] crate::diagnostic_process::ProcessError),
    #[error("original sandbox source or retained observation changed")]
    Binding,
}
#[derive(PartialEq, Eq)]
struct Source {
    context: Context,
    unit: String,
    root: String,
    group: String,
    image: [u64; 3],
    worker: WorkerPolicy,
}
struct Binding<S> {
    source: S,
    managed: Managed,
}
trait SandboxChecks {
    type Source: PartialEq;
    fn refused(&self) -> bool;
    fn frame(&mut self) -> Result<(), SandboxError>;
    fn source(&self) -> Result<Self::Source, SandboxError>;
    fn manager(&mut self, source: &Self::Source) -> Result<Managed, SandboxError>;
    fn kernel(&mut self, source: &Self::Source, managed: &Managed) -> Result<(), SandboxError>;
    fn refuse(&mut self);
}
fn round<C: SandboxChecks>(
    owner: &mut C,
    prior: Option<&Binding<C::Source>>,
) -> Result<Binding<C::Source>, SandboxError> {
    let result = (|| {
        if owner.refused() {
            return Err(SandboxError::Binding);
        }
        owner.frame()?;
        let source = owner.source()?;
        if prior.is_some_and(|p| p.source != source) {
            return Err(SandboxError::Binding);
        }
        let managed = owner.manager(&source)?;
        if prior.is_some_and(|p| p.managed != managed) {
            return Err(SandboxError::Binding);
        }
        owner.kernel(&source, &managed)?;
        owner.frame()?;
        if owner.source()? != source || owner.manager(&source)? != managed {
            return Err(SandboxError::Binding);
        }
        owner.kernel(&source, &managed)?;
        owner.frame()?;
        if owner.source()? != source {
            return Err(SandboxError::Binding);
        }
        Ok(Binding { source, managed })
    })();
    if result.is_err() {
        owner.refuse();
    }
    result
}
pub(crate) struct SandboxObservedUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Explicit refusal and field order both precede published observation closure.
    owner: InstalledEntryUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    kernel: Option<SandboxRead>,
    retained: Option<Binding<Source>>,
    kernel_pinned: bool,
    refused: bool,
}
pub(crate) struct SandboxFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: Box<SandboxError>,
    held: Box<SandboxObservedUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for SandboxFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl SandboxFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn is_refused(&self) -> bool {
        self.held.refused
    }
    pub(crate) fn error(&self) -> &SandboxError {
        &self.error
    }
}
impl SandboxChecks for SandboxObservedUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    type Source = Source;
    fn refused(&self) -> bool {
        self.refused || self.owner.refused()
    }
    fn frame(&mut self) -> Result<(), SandboxError> {
        Ok(self.owner.revalidate()?)
    }
    fn source(&self) -> Result<Source, SandboxError> {
        let projected = &self.owner.owner.owner.owner.owner;
        let projection = &projected.projection;
        let documents = projected
            .owner
            .documents
            .as_ref()
            .ok_or(SandboxError::Binding)?
            .slices();
        if crate::sha256_hex(documents[1]) != projection.context.documents_sha256[1] {
            return Err(SandboxError::Binding);
        }
        // The complete original frame authenticates these SAME owned bytes and
        // current epoch. Parsing here neither selects trust nor upgrades claims.
        let worker: SignedWorkerPolicy =
            serde_json::from_slice(documents[1]).map_err(|_| SandboxError::Binding)?;
        if worker.policy.boot_id != projection.context.state.identity.boot_id {
            return Err(SandboxError::Binding);
        }
        Ok(Source {
            context: projection.context.clone(),
            unit: projection.unit.clone(),
            root: projection.root_path.clone(),
            group: projection.held_path.clone(),
            image: self
                .owner
                .owner
                .retained
                .as_ref()
                .ok_or(SandboxError::Binding)?
                .sandbox_image_identity(),
            worker: worker.policy,
        })
    }
    fn manager(&mut self, source: &Source) -> Result<Managed, SandboxError> {
        let projected = &mut self.owner.owner.owner.owner.owner;
        let message = &mut projected.owner.placement.held.held.owner.original;
        let channel = message
            .original
            .channel
            .as_mut()
            .ok_or(SandboxError::Binding)?;
        if (&channel.manager, &channel.client) != (&source.context.names.0, &source.context.names.1)
        {
            return Err(SandboxError::Binding);
        }
        channel
            .observe_sandbox(&source.unit, &source.group)
            .map_err(|_| SandboxError::Manager)
    }
    fn kernel(&mut self, source: &Source, managed: &Managed) -> Result<(), SandboxError> {
        if self.kernel.is_none() {
            if self.kernel_pinned {
                return Err(SandboxError::Binding);
            }
            let credentials = crate::diagnostic_process::AssertedWorkerCredentials::new(
                source.worker.uid,
                source.worker.gid,
                source.worker.supplementary_gids.clone(),
            )
            .map_err(|_| SandboxError::Binding)?;
            self.kernel = Some(SandboxRead::pin(
                managed.0,
                Expected {
                    boot: source.context.state.identity.boot_id.clone(),
                    root: source.root.clone(),
                    group: source.group.clone(),
                    root_id: [
                        source.context.root.root_device,
                        source.context.root.root_inode,
                    ],
                    image: source.image,
                    streams: source.context.streams,
                    credentials,
                },
            )?);
            self.kernel_pinned = true;
        }
        Ok(self
            .kernel
            .as_mut()
            .ok_or(SandboxError::Binding)?
            .revalidate()?)
    }
    fn refuse(&mut self) {
        self.owner.refuse();
        self.refused = true;
    }
}
impl SandboxObservedUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), SandboxError> {
        let prior = self.retained.take();
        let result = if prior.is_none() {
            self.refuse();
            Err(SandboxError::Binding)
        } else {
            round(self, prior.as_ref()).map(|_| ())
        };
        self.retained = prior;
        result
    }
}
impl Drop for SandboxObservedUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        self.refuse();
    }
}
impl<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    InstalledEntryUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn observe_managed_sandbox(
        self,
    ) -> Result<
        SandboxObservedUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        SandboxFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = SandboxObservedUnit {
            owner: self,
            kernel: None,
            retained: None,
            kernel_pinned: false,
            refused: false,
        };
        match round(&mut held, None) {
            Ok(value) => {
                held.retained = Some(value);
                Ok(held)
            }
            Err(error) => {
                held.refuse();
                Err(SandboxFailure {
                    error: Box::new(error),
                    held: Box::new(held),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Model {
        log: Vec<&'static str>,
        calls: usize,
        fail: usize,
        drift: usize,
        source_drift: bool,
        refused: bool,
    }
    impl Model {
        fn step(&mut self, name: &'static str) -> Result<(), SandboxError> {
            self.log.push(name);
            self.calls += 1;
            if self.calls == self.fail {
                Err(SandboxError::Binding)
            } else {
                Ok(())
            }
        }
    }
    impl SandboxChecks for Model {
        type Source = u32;
        fn refused(&self) -> bool {
            self.refused
        }
        fn frame(&mut self) -> Result<(), SandboxError> {
            self.step("frame")
        }
        fn source(&self) -> Result<u32, SandboxError> {
            Ok(if self.source_drift && self.calls >= 4 {
                75
            } else {
                74
            })
        }
        fn manager(&mut self, _: &u32) -> Result<Managed, SandboxError> {
            self.step("manager")?;
            Ok((
                if self.drift == self.calls { 75 } else { 74 },
                [7; 16],
                "fixed-unit".into(),
            ))
        }
        fn kernel(&mut self, _: &u32, _: &Managed) -> Result<(), SandboxError> {
            self.step("kernel")
        }
        fn refuse(&mut self) {
            self.refused = true;
        }
    }
    #[test]
    fn bootstrap_sandbox_owner_order_retains_all_three_complete_frames() {
        let mut m = Model::default();
        let binding = round(&mut m, None).unwrap();
        assert_eq!(
            m.log,
            ["frame", "manager", "kernel", "frame", "manager", "kernel", "frame"]
        );
        round(&mut m, Some(&binding)).unwrap();
        assert!(!m.refused);
    }
    #[test]
    fn bootstrap_sandbox_owner_every_cut_and_restore_remains_refused() {
        for fail in 1..=7 {
            let mut m = Model {
                fail,
                ..Model::default()
            };
            assert!(round(&mut m, None).is_err());
            let calls = m.calls;
            m.fail = 0;
            assert!(round(&mut m, None).is_err());
            assert_eq!(calls, m.calls);
            assert!(m.refused);
        }
        for drift in [2, 5] {
            let mut m = Model {
                drift,
                ..Model::default()
            };
            assert!(round(&mut m, None).is_err());
            assert!(m.refused);
        }
        let mut m = Model {
            source_drift: true,
            ..Model::default()
        };
        assert!(round(&mut m, None).is_err());
        assert!(m.refused);
    }
    #[test]
    fn bootstrap_sandbox_owner_retained_source_and_invocation_cannot_refresh() {
        let mut m = Model::default();
        let mut binding = round(&mut m, None).unwrap();
        binding.source = 75;
        assert!(round(&mut m, Some(&binding)).is_err());
        assert!(m.refused);
        for field in 0..3 {
            let mut m = Model::default();
            let mut binding = round(&mut m, None).unwrap();
            match field {
                0 => binding.managed.0 += 1,
                1 => binding.managed.1[0] ^= 1,
                _ => binding.managed.2.push('x'),
            }
            assert!(round(&mut m, Some(&binding)).is_err());
            assert!(m.refused);
        }
    }
    #[test]
    fn bootstrap_sandbox_owner_requires_consumed_original_no_claim_constructor() {
        fn signature(original: InstalledEntryUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
            if let Ok(mut observed) = original.observe_managed_sandbox() {
                let _ = observed.revalidate();
            }
        }
        let _contract = signature;
    }
}
