//! Consuming original delivery candidate. Readiness is not release/completion.
use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::entry::installed::entry_material::held_wire::{HeldRequest,child_exchange::{Pair,Challenge}};
use sha2::{Digest,Sha256};
#[derive(Debug, thiserror::Error)]
pub(crate) enum DeliveryError {
    #[error("original full frame refused: {0}")]
    Frame(#[from] BootstrapError),
    #[error("original sandbox frame refused: {0}")]
    Sandbox(#[from] SandboxError),
    #[error("held request or readiness I/O refused: {0}")]
    Io(#[from] std::io::Error),
    #[error("held delivery owner, manager or one-shot state differs")]
    Binding,
}
enum Owner<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    Entry(InstalledEntryUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>),
    Sandbox(SandboxObservedUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>),
    Failed(SandboxFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>),
}
#[derive(Debug, PartialEq, Eq)]
enum Stage {
    Preparing,
    Prepared,
    Uncertain,
    Delivered,
    AwaitingChild,
    ReadyObserved,
    Refused,
}
pub(crate) struct HeldDelivery<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Refuse original custody before closing ANY published transport aliases.
    owner: Option<Owner<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
    request: Option<HeldRequest>,
    pair: Option<Pair>,
    context: Option<Context>,
    entry: Option<[u8; 32]>,
    peer: Option<[u32; 3]>,
    stage: Stage,
}
pub(crate) struct DeliveryFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: DeliveryError,
    held: Box<HeldDelivery<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for DeliveryFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl DeliveryFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn error(&self) -> &DeliveryError {
        &self.error
    }
    pub(crate) fn is_refused(&self) -> bool {
        self.held.stage == Stage::Refused
    }
}
impl<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    HeldDelivery<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    fn entry_owner(
        &self,
    ) -> Result<
        &InstalledEntryUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        DeliveryError,
    > {
        match self.owner.as_ref() {
            Some(Owner::Entry(v)) => Ok(v),
            Some(Owner::Sandbox(v)) => Ok(&v.owner),
            _ => Err(DeliveryError::Binding),
        }
    }
    fn frame(&mut self) -> Result<(), DeliveryError> {
        if self.stage == Stage::Refused {
            return Err(DeliveryError::Binding);
        }
        match self.owner.as_mut() {
            Some(Owner::Entry(v)) => v.revalidate()?,
            Some(Owner::Sandbox(v)) => v.revalidate()?,
            _ => return Err(DeliveryError::Binding),
        }
        if let Some(context) = &self.context {
            let owner = self.entry_owner()?;
            if owner.owner.owner.owner.owner.projection.context != *context
                || owner
                    .retained
                    .as_ref()
                    .ok_or(DeliveryError::Binding)?
                    .held_entry_digest()?
                    != self.entry.ok_or(DeliveryError::Binding)?
            {
                return Err(DeliveryError::Binding);
            }
        }
        if let Some(pair) = &self.pair {
            pair.check()?;
        }
        if let Some(request) = &self.request {
            request.inspect()?;
        }
        Ok(())
    }
    fn prepare(&mut self) -> Result<(), DeliveryError> {
        self.frame()?;
        let pair = Pair::create()?;
        let owner = self.entry_owner()?;
        let p = &owner.owner.owner.owner.owner.projection;
        let context = p.context.clone();
        let entry = owner
            .retained
            .as_ref()
            .ok_or(DeliveryError::Binding)?
            .held_entry_digest()?;
        let request = owner
            .retained
            .as_ref()
            .ok_or(DeliveryError::Binding)?
            .prepare_held_request(
                &p.unit,
                &p.properties,
                &context.names.0,
                &context.names.1,
                context.serial,
                pair.child_fd(),
            )?;
        self.pair = Some(pair);
        self.request = Some(request);
        self.context = Some(context);
        self.entry = Some(entry);
        self.frame()?;
        self.stage = Stage::Prepared;
        Ok(())
    }
    pub(crate) fn deliver_once(&mut self) -> Result<(), DeliveryError> {
        let result = (|| {
            if self.stage != Stage::Prepared {
                return Err(DeliveryError::Binding);
            }
            self.stage = Stage::Uncertain;
            self.frame()?;
            let Some(Owner::Entry(owner)) = self.owner.as_mut() else {
                return Err(DeliveryError::Binding);
            };
            let original = &mut owner
                .owner
                .owner
                .owner
                .owner
                .owner
                .placement
                .held
                .held
                .owner
                .original;
            let channel = original
                .original
                .channel
                .as_mut()
                .ok_or(DeliveryError::Binding)?;
            channel
                .deliver_held_once(self.request.as_mut().ok_or(DeliveryError::Binding)?)
                .map_err(|_| DeliveryError::Binding)?;
            self.frame()?;
            self.stage = Stage::Delivered;
            Ok(())
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
    pub(crate) fn challenge_child_once(&mut self) -> Result<(), DeliveryError> {
        let result = (|| {
            if self.stage != Stage::Delivered {
                return Err(DeliveryError::Binding);
            }
            self.stage = Stage::AwaitingChild;
            self.frame()?;
            let Some(Owner::Entry(original)) = self.owner.take() else {
                return Err(DeliveryError::Binding);
            };
            match original.observe_managed_sandbox() {
                Ok(v) => self.owner = Some(Owner::Sandbox(v)),
                Err(e) => {
                    self.owner = Some(Owner::Failed(e));
                    return Err(DeliveryError::Binding);
                }
            }
            self.frame()?;
            let Some(Owner::Sandbox(observed)) = self.owner.as_ref() else {
                return Err(DeliveryError::Binding);
            };
            let binding = observed.retained.as_ref().ok_or(DeliveryError::Binding)?;
            let c = self.context.as_ref().ok_or(DeliveryError::Binding)?;
            if binding.source.context != *c {
                return Err(DeliveryError::Binding);
            }
            let mut nonce = [0; 32];
            if unsafe {
                libc::getrandom(nonce.as_mut_ptr().cast(), nonce.len(), libc::GRND_NONBLOCK)
            } != 32
            {
                return Err(DeliveryError::Binding);
            }
            let mut creator = [0; 3];
            for (i, v) in [c.owner[0], c.owner[4], c.owner[5]].iter().enumerate() {
                creator[i] = u32::try_from(*v).map_err(|_| DeliveryError::Binding)?;
            }
            let challenge = Challenge {
                deadline: c.bounds[3],
                creator,
                child: binding.managed.0,
                nonce,
                entry: self.entry.ok_or(DeliveryError::Binding)?,
                context: c.root.original_context_sha256,
                manager: Sha256::digest(c.names.0.as_bytes()).into(),
                client: Sha256::digest(c.names.1.as_bytes()).into(),
                invocation: binding.managed.1,
            };
            self.peer = Some([
                binding.managed.0,
                binding.source.worker.uid,
                binding.source.worker.gid,
            ]);
            self.pair
                .as_mut()
                .ok_or(DeliveryError::Binding)?
                .begin(challenge)?;
            self.frame()
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
    pub(crate) fn try_child_ack(&mut self) -> Result<bool, DeliveryError> {
        let result = (|| {
            if self.stage != Stage::AwaitingChild {
                return Err(DeliveryError::Binding);
            }
            self.frame()?;
            let ready = self
                .pair
                .as_mut()
                .ok_or(DeliveryError::Binding)?
                .try_ack(self.peer.ok_or(DeliveryError::Binding)?)?;
            self.frame()?;
            if ready {
                self.stage = Stage::ReadyObserved;
            }
            Ok(ready)
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
    fn refuse(&mut self) {
        match self.owner.as_mut() {
            Some(Owner::Entry(v)) => v.refuse(),
            Some(Owner::Sandbox(v)) => v.refuse(),
            Some(Owner::Failed(v)) => {
                let _ = v.is_refused();
            }
            None => {}
        }
        if let Some(v) = self.request.as_mut() {
            v.refuse();
        }
        if let Some(v) = self.pair.as_mut() {
            v.refuse();
        }
        self.stage = Stage::Refused;
    }
}
impl Drop for HeldDelivery<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        self.refuse();
    }
}
impl<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    InstalledEntryUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_original_held_delivery(
        self,
    ) -> Result<
        HeldDelivery<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        DeliveryFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = HeldDelivery {
            owner: Some(Owner::Entry(self)),
            request: None,
            pair: None,
            context: None,
            entry: None,
            peer: None,
            stage: Stage::Preparing,
        };
        match held.prepare() {
            Ok(()) => Ok(held),
            Err(error) => {
                held.refuse();
                Err(DeliveryFailure {
                    error,
                    held: Box::new(held),
                })
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn held_delivery_original_constructor_consumes_and_exports_no_owner_or_fd() {
        fn contract(owner: InstalledEntryUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
            if let Ok(mut held) = owner.prepare_original_held_delivery() {
                let _ = held.deliver_once();
                let _ = held.challenge_child_once();
                let _ = held.try_child_ack();
            }
        }
        let _signature = contract;
    }
}
