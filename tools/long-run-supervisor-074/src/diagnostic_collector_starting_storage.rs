//! Private opt-in phase on the original storage; default two-file inspection is unchanged.
use super::*;

pub(crate) const STARTING_NAME: &str = "diagnostic-collector-starting-074.json";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectorStartingStage {
    BeforeCreate,
    Created,
    CreateDirectorySynced,
    PrefixWritten,
    BytesWritten,
    Sealed,
    FileSynced,
    DirectorySynced,
    Acknowledging,
}
pub(crate) const STAGES: [CollectorStartingStage; 9] = [
    CollectorStartingStage::BeforeCreate,
    CollectorStartingStage::Created,
    CollectorStartingStage::CreateDirectorySynced,
    CollectorStartingStage::PrefixWritten,
    CollectorStartingStage::BytesWritten,
    CollectorStartingStage::Sealed,
    CollectorStartingStage::FileSynced,
    CollectorStartingStage::DirectorySynced,
    CollectorStartingStage::Acknowledging,
];
pub(super) struct StartingPhase {
    document: Option<Document>,
    next: usize,
    acknowledged: bool,
}
impl CollectorReservationStorage<'_> {
    pub(crate) fn begin_starting(&mut self) -> Result<(), StorageAttemptError> {
        if self.starting.is_some() {
            self.latch_refusal();
            return Err(StorageAttemptError::Refused);
        }
        self.revalidate()?;
        // Attempt latch precedes creation; no API resets or returns this phase.
        self.starting = Some(StartingPhase {
            document: None,
            next: 0,
            acknowledged: false,
        });
        Ok(())
    }
    pub(super) fn inspect_starting(&mut self) -> Result<(), StorageAttemptError> {
        if self.is_refused()
            || !self.acknowledged
            || !self.original.acknowledged
            || self.original.name != CONTROLLER_NAME
        {
            return Err(StorageAttemptError::Refused);
        }
        self.original.ancestry()?;
        let leaf = self.original.leaf()?;
        let controller = self
            .original
            .document
            .as_ref()
            .ok_or(StorageAttemptError::Refused)?;
        let reserved = self.document.as_ref().ok_or(StorageAttemptError::Refused)?;
        let phase = self.starting.as_ref().ok_or(StorageAttemptError::Refused)?;
        controller.inspect(leaf)?;
        reserved.inspect(leaf)?;
        if let Some(doc) = &phase.document {
            doc.inspect(leaf)?;
        }
        let fresh = open_at(leaf, ".", libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        let mut seen = [false; 3];
        for entry in std::fs::read_dir(format!("/proc/self/fd/{}", fresh.as_raw_fd()))? {
            let name = entry?.file_name();
            let index = if name == CONTROLLER_NAME {
                0
            } else if name == NAME {
                1
            } else if phase.document.is_some() && name == STARTING_NAME {
                2
            } else {
                return Err(StorageAttemptError::Context);
            };
            if seen[index] {
                return Err(StorageAttemptError::Context);
            }
            seen[index] = true;
        }
        if seen != [true, true, phase.document.is_some()] {
            return Err(StorageAttemptError::Context);
        }
        controller.inspect(leaf)?;
        reserved.inspect(leaf)?;
        if let Some(doc) = &phase.document {
            doc.inspect(leaf)?;
        }
        self.original.ancestry()
    }
    fn starting_effect(
        &mut self,
        at: CollectorStartingStage,
        raw: &[u8],
    ) -> Result<(), StorageAttemptError> {
        self.inspect_starting()?;
        let phase = self.starting.as_ref().ok_or(StorageAttemptError::Refused)?;
        if phase.acknowledged
            || STAGES.get(phase.next) != Some(&at)
            || raw.is_empty()
            || raw.len() > 16_384
        {
            return Err(StorageAttemptError::Refused);
        }
        match at {
            CollectorStartingStage::BeforeCreate | CollectorStartingStage::Acknowledging => {}
            CollectorStartingStage::Created => {
                let file = open_at(
                    self.original.leaf()?,
                    STARTING_NAME,
                    libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
                    0o600,
                )?;
                // Retain the FD even if metadata inspection subsequently fails.
                self.starting.as_mut().unwrap().document = Some(Document {
                    name: STARTING_NAME,
                    file,
                    stamp: [0; 11],
                    raw: Vec::new(),
                    mode: 0o600,
                });
                let doc = self.starting.as_mut().unwrap().document.as_mut().unwrap();
                doc.stamp = stamp(&doc.file.metadata()?);
            }
            CollectorStartingStage::CreateDirectorySynced
            | CollectorStartingStage::DirectorySynced => self.original.leaf()?.sync_all()?,
            CollectorStartingStage::PrefixWritten
            | CollectorStartingStage::BytesWritten
            | CollectorStartingStage::Sealed => {
                let leaf = open_at(
                    self.original.leaf()?,
                    ".",
                    libc::O_RDONLY | libc::O_DIRECTORY,
                    0,
                )?;
                let middle = raw.len() / 2;
                let (bytes, seal) = match at {
                    CollectorStartingStage::PrefixWritten => (&raw[..middle], false),
                    CollectorStartingStage::BytesWritten => (&raw[middle..], false),
                    _ => (&[][..], true),
                };
                self.starting
                    .as_mut()
                    .unwrap()
                    .document
                    .as_mut()
                    .ok_or(StorageAttemptError::Refused)?
                    .owned_write(&leaf, bytes, seal)?;
            }
            CollectorStartingStage::FileSynced => self
                .starting
                .as_ref()
                .unwrap()
                .document
                .as_ref()
                .ok_or(StorageAttemptError::Refused)?
                .file
                .sync_all()?,
        }
        self.starting.as_mut().unwrap().next += 1;
        self.inspect_starting()
    }
    pub(crate) fn starting_step(
        &mut self,
        at: CollectorStartingStage,
        raw: &[u8],
    ) -> Result<(), StorageAttemptError> {
        let result = self.starting_effect(at, raw);
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
    pub(crate) fn acknowledge_starting(&mut self) -> Result<(), StorageAttemptError> {
        let result = (|| {
            self.revalidate()?;
            let phase = self.starting.as_mut().ok_or(StorageAttemptError::Refused)?;
            if phase.next != STAGES.len() || phase.acknowledged {
                return Err(StorageAttemptError::Refused);
            }
            phase.acknowledged = true;
            Ok(())
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
    use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::with_owned_empty_fence_for_test;
    use std::os::unix::fs::{symlink, PermissionsExt};

    fn with_storage(run: impl FnOnce(&mut CollectorReservationStorage<'_>, &std::path::Path)) {
        with_owned_empty_fence_for_test(|fence, _, leaf| {
            let mut attempt = AttemptStorage::for_controller(fence);
            attempt
                .retain_with(b"controller\n", |_| Ok::<_, StorageAttemptError>(()))
                .unwrap();
            let mut storage = CollectorReservationStorage::from_original(attempt);
            storage
                .publish_with(b"reserved\n", |_| Ok::<_, StorageAttemptError>(()))
                .unwrap();
            run(&mut storage, leaf);
        });
    }
    #[test]
    fn collector_starting_storage_exact_phase_order_and_old_inventory() {
        with_storage(|storage, leaf| {
            storage.revalidate().unwrap();
            storage.begin_starting().unwrap();
            for (index, at) in STAGES.into_iter().enumerate() {
                storage.starting_step(at, b"starting\n").unwrap();
                assert_eq!(storage.starting.as_ref().unwrap().next, index + 1);
                assert!(!storage.starting.as_ref().unwrap().acknowledged);
                if index >= 1 {
                    assert!(storage.inspect().is_err());
                }
                storage.revalidate().unwrap();
            }
            assert_eq!(
                std::fs::read(leaf.join(STARTING_NAME)).unwrap(),
                b"starting\n"
            );
            storage.acknowledge_starting().unwrap();
            assert!(storage.starting.as_ref().unwrap().acknowledged);
            storage.revalidate().unwrap();
            assert!(storage
                .starting_step(CollectorStartingStage::BeforeCreate, b"starting\n")
                .is_err());
            assert!(storage.is_refused());
        });
    }
    #[test]
    fn collector_starting_storage_refuses_reentry_skips_early_ack_and_unexpected_third_file() {
        for kind in 0..5 {
            with_storage(|storage, leaf| {
                if kind == 4 {
                    assert!(storage.acknowledge_starting().is_err());
                    assert!(storage.is_refused());
                    return;
                }
                storage.begin_starting().unwrap();
                let result = match kind {
                    0 => storage.begin_starting(),
                    1 => storage.starting_step(CollectorStartingStage::Created, b"starting\n"),
                    2 => storage.acknowledge_starting(),
                    _ => {
                        std::fs::write(leaf.join(STARTING_NAME), b"foreign").unwrap();
                        storage.starting_step(CollectorStartingStage::BeforeCreate, b"starting\n")
                    }
                };
                assert!(result.is_err());
                assert!(storage.is_refused());
            });
        }
    }
    #[test]
    fn collector_starting_storage_exclusive_create_and_fd_mutations_refuse() {
        for kind in 0..6 {
            with_storage(|storage, leaf| {
                storage.begin_starting().unwrap();
                storage
                    .starting_step(CollectorStartingStage::BeforeCreate, b"starting\n")
                    .unwrap();
                if kind == 0 {
                    std::fs::write(leaf.join(STARTING_NAME), b"foreign").unwrap();
                    assert!(storage
                        .starting_step(CollectorStartingStage::Created, b"starting\n")
                        .is_err());
                    assert_eq!(std::fs::read(leaf.join(STARTING_NAME)).unwrap(), b"foreign");
                } else {
                    storage
                        .starting_step(CollectorStartingStage::Created, b"starting\n")
                        .unwrap();
                    let path = leaf.join(STARTING_NAME);
                    match kind {
                        1 => std::fs::write(&path, b"external").unwrap(),
                        2 => {
                            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400))
                                .unwrap()
                        }
                        3 => {
                            std::fs::remove_file(&path).unwrap();
                            symlink(NAME, &path).unwrap();
                        }
                        4 => std::fs::hard_link(&path, leaf.join("extra")).unwrap(),
                        _ => std::fs::rename(&path, leaf.join("renamed")).unwrap(),
                    }
                    assert!(storage
                        .starting_step(CollectorStartingStage::CreateDirectorySynced, b"starting\n")
                        .is_err());
                }
                assert!(storage.is_refused());
            });
        }
    }
}
