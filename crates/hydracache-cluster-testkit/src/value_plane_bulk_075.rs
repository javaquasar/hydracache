//! Bounded partition-grouped bulk execution model with stable partial retry receipts.

use std::collections::{BTreeMap, BTreeSet};

use crate::value_plane_model_075::{CanonicalMapKey, MutationIdentity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BulkBounds {
    pub max_items: usize,
    pub max_request_bytes: usize,
    pub max_partitions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkInput {
    pub index: usize,
    pub key: CanonicalMapKey,
    pub identity: MutationIdentity,
    pub request_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkGroup {
    pub partition: u32,
    pub owner_generation: u64,
    pub input_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkOutcome {
    Applied {
        version: u64,
    },
    Absent,
    Failed {
        reason: &'static str,
        retryable: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkReceiptItem {
    pub input_index: usize,
    pub partition: u32,
    pub owner_generation: u64,
    pub outcome: Option<BulkOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkReceipt {
    pub items: Vec<BulkReceiptItem>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkError {
    InvalidBound(&'static str),
    ItemLimit,
    RequestBytesLimit,
    PartitionLimit,
    InvalidInputOrder,
    DuplicateKey,
    DuplicateIdentity,
    UnknownInput,
    StaleOwnerGeneration { expected: u64, actual: u64 },
    GenerationMustAdvance,
    ConflictingCompletion,
}

#[derive(Debug, Clone)]
struct ItemState {
    input: BulkInput,
    owner_generation: u64,
    outcome: Option<BulkOutcome>,
}

#[derive(Debug)]
pub struct PartitionedBulkExecution {
    items: Vec<ItemState>,
}

impl PartitionedBulkExecution {
    pub fn new(
        bounds: BulkBounds,
        inputs: Vec<BulkInput>,
        owner_generations: &BTreeMap<u32, u64>,
    ) -> Result<Self, BulkError> {
        for (name, value) in [
            ("items", bounds.max_items),
            ("request_bytes", bounds.max_request_bytes),
            ("partitions", bounds.max_partitions),
        ] {
            if value == 0 {
                return Err(BulkError::InvalidBound(name));
            }
        }
        if inputs.len() > bounds.max_items {
            return Err(BulkError::ItemLimit);
        }
        if inputs
            .iter()
            .map(|input| input.request_bytes)
            .sum::<usize>()
            > bounds.max_request_bytes
        {
            return Err(BulkError::RequestBytesLimit);
        }
        if inputs
            .iter()
            .enumerate()
            .any(|(expected, input)| input.index != expected)
        {
            return Err(BulkError::InvalidInputOrder);
        }
        let keys = inputs
            .iter()
            .map(|input| &input.key)
            .collect::<BTreeSet<_>>();
        if keys.len() != inputs.len() {
            return Err(BulkError::DuplicateKey);
        }
        let identities = inputs
            .iter()
            .map(|input| &input.identity)
            .collect::<BTreeSet<_>>();
        if identities.len() != inputs.len() {
            return Err(BulkError::DuplicateIdentity);
        }
        let partitions = inputs
            .iter()
            .map(|input| input.key.partition)
            .collect::<BTreeSet<_>>();
        if partitions.len() > bounds.max_partitions {
            return Err(BulkError::PartitionLimit);
        }
        let mut items = Vec::with_capacity(inputs.len());
        for input in inputs {
            let owner_generation = owner_generations.get(&input.key.partition).copied().ok_or(
                BulkError::StaleOwnerGeneration {
                    expected: 1,
                    actual: 0,
                },
            )?;
            items.push(ItemState {
                input,
                owner_generation,
                outcome: None,
            });
        }
        Ok(Self { items })
    }

    pub fn pending_groups(&self) -> Vec<BulkGroup> {
        let mut groups = BTreeMap::<(u32, u64), Vec<usize>>::new();
        for item in self.items.iter().filter(|item| item.outcome.is_none()) {
            groups
                .entry((item.input.key.partition, item.owner_generation))
                .or_default()
                .push(item.input.index);
        }
        groups
            .into_iter()
            .map(|((partition, owner_generation), input_indices)| BulkGroup {
                partition,
                owner_generation,
                input_indices,
            })
            .collect()
    }

    pub fn complete(
        &mut self,
        input_index: usize,
        owner_generation: u64,
        outcome: BulkOutcome,
    ) -> Result<(), BulkError> {
        let item = self
            .items
            .get_mut(input_index)
            .ok_or(BulkError::UnknownInput)?;
        if owner_generation != item.owner_generation {
            return Err(BulkError::StaleOwnerGeneration {
                expected: item.owner_generation,
                actual: owner_generation,
            });
        }
        if let Some(existing) = &item.outcome {
            return if existing == &outcome {
                Ok(())
            } else {
                Err(BulkError::ConflictingCompletion)
            };
        }
        item.outcome = Some(outcome);
        Ok(())
    }

    pub fn advance_owner(&mut self, partition: u32, new_generation: u64) -> Result<(), BulkError> {
        let current = self
            .items
            .iter()
            .filter(|item| item.input.key.partition == partition)
            .map(|item| item.owner_generation)
            .max()
            .ok_or(BulkError::UnknownInput)?;
        if new_generation <= current {
            return Err(BulkError::GenerationMustAdvance);
        }
        for item in self
            .items
            .iter_mut()
            .filter(|item| item.input.key.partition == partition && item.outcome.is_none())
        {
            item.owner_generation = new_generation;
        }
        Ok(())
    }

    pub fn receipt(&self) -> BulkReceipt {
        let items = self
            .items
            .iter()
            .map(|item| BulkReceiptItem {
                input_index: item.input.index,
                partition: item.input.key.partition,
                owner_generation: item.owner_generation,
                outcome: item.outcome.clone(),
            })
            .collect::<Vec<_>>();
        BulkReceipt {
            complete: items.iter().all(|item| item.outcome.is_some()),
            items,
        }
    }
}
