//! Multi-subscriber filtered listener model with per-partition continuity.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FilteredEventKind {
    Added,
    Updated,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilteredEvent {
    pub partition: u32,
    pub generation: u64,
    pub watermark: u64,
    pub key: Vec<u8>,
    pub kind: FilteredEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionDelivery {
    Event(FilteredEvent),
    Gap {
        partition: u32,
        generation: u64,
        after_watermark: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionFilter {
    pub key_prefix: Vec<u8>,
    pub kinds: BTreeSet<FilteredEventKind>,
}

impl SubscriptionFilter {
    fn matches(&self, event: &FilteredEvent) -> bool {
        event.key.starts_with(&self.key_prefix)
            && (self.kinds.is_empty() || self.kinds.contains(&event.kind))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionError {
    InvalidBound,
    InvalidSubscription,
    Capacity,
    DuplicateSubscription,
    UnknownSubscription,
    UnknownPartition,
    StaleGeneration,
    GapRequired,
    RepairNotRequired,
    InvalidRepair,
}

impl fmt::Display for SubscriptionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SubscriptionError {}

#[derive(Debug, Clone, Copy)]
struct Cursor {
    generation: u64,
    watermark: u64,
    gap: bool,
}

#[derive(Debug)]
struct Subscriber {
    filter: SubscriptionFilter,
    cursors: BTreeMap<u32, Cursor>,
    backlog: VecDeque<SubscriptionDelivery>,
}

#[derive(Debug)]
pub struct FilteredSubscriptionHub {
    max_subscriptions: usize,
    max_backlog: usize,
    subscribers: BTreeMap<u64, Subscriber>,
}

impl FilteredSubscriptionHub {
    pub fn new(max_subscriptions: usize, max_backlog: usize) -> Result<Self, SubscriptionError> {
        if max_subscriptions == 0 || max_backlog == 0 {
            return Err(SubscriptionError::InvalidBound);
        }
        Ok(Self {
            max_subscriptions,
            max_backlog,
            subscribers: BTreeMap::new(),
        })
    }

    pub fn subscribe(
        &mut self,
        id: u64,
        filter: SubscriptionFilter,
        partitions: impl IntoIterator<Item = (u32, u64, u64)>,
    ) -> Result<(), SubscriptionError> {
        if id == 0 {
            return Err(SubscriptionError::InvalidSubscription);
        }
        if self.subscribers.contains_key(&id) {
            return Err(SubscriptionError::DuplicateSubscription);
        }
        if self.subscribers.len() >= self.max_subscriptions {
            return Err(SubscriptionError::Capacity);
        }
        let cursors = partitions
            .into_iter()
            .map(|(partition, generation, watermark)| {
                (
                    partition,
                    Cursor {
                        generation,
                        watermark,
                        gap: false,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        if cursors.is_empty() || cursors.values().any(|cursor| cursor.generation == 0) {
            return Err(SubscriptionError::InvalidSubscription);
        }
        self.subscribers.insert(
            id,
            Subscriber {
                filter,
                cursors,
                backlog: VecDeque::new(),
            },
        );
        Ok(())
    }

    pub fn unsubscribe(&mut self, id: u64) -> Result<usize, SubscriptionError> {
        self.subscribers
            .remove(&id)
            .map(|subscriber| subscriber.backlog.len())
            .ok_or(SubscriptionError::UnknownSubscription)
    }

    pub fn publish(&mut self, event: FilteredEvent) -> Result<usize, SubscriptionError> {
        let mut delivered = 0;
        for subscriber in self.subscribers.values_mut() {
            let Some(cursor) = subscriber.cursors.get_mut(&event.partition) else {
                continue;
            };
            if event.generation < cursor.generation {
                return Err(SubscriptionError::StaleGeneration);
            }
            if cursor.gap {
                continue;
            }
            if event.generation > cursor.generation
                || event.watermark > cursor.watermark.saturating_add(1)
                || subscriber.backlog.len() >= self.max_backlog
            {
                cursor.generation = cursor.generation.max(event.generation);
                cursor.gap = true;
                subscriber.backlog.clear();
                subscriber.backlog.push_back(SubscriptionDelivery::Gap {
                    partition: event.partition,
                    generation: cursor.generation,
                    after_watermark: cursor.watermark,
                });
                continue;
            }
            if event.watermark <= cursor.watermark {
                continue;
            }
            cursor.watermark = event.watermark;
            if subscriber.filter.matches(&event) {
                subscriber
                    .backlog
                    .push_back(SubscriptionDelivery::Event(event.clone()));
                delivered += 1;
            }
        }
        Ok(delivered)
    }

    pub fn repair(
        &mut self,
        id: u64,
        partition: u32,
        generation: u64,
        watermark: u64,
    ) -> Result<(), SubscriptionError> {
        let subscriber = self
            .subscribers
            .get_mut(&id)
            .ok_or(SubscriptionError::UnknownSubscription)?;
        let cursor = subscriber
            .cursors
            .get_mut(&partition)
            .ok_or(SubscriptionError::UnknownPartition)?;
        if !cursor.gap {
            return Err(SubscriptionError::RepairNotRequired);
        }
        if generation < cursor.generation || watermark < cursor.watermark {
            return Err(SubscriptionError::InvalidRepair);
        }
        cursor.generation = generation;
        cursor.watermark = watermark;
        cursor.gap = false;
        Ok(())
    }

    pub fn drain(&mut self, id: u64) -> Result<Vec<SubscriptionDelivery>, SubscriptionError> {
        self.subscribers
            .get_mut(&id)
            .map(|subscriber| subscriber.backlog.drain(..).collect())
            .ok_or(SubscriptionError::UnknownSubscription)
    }

    pub const fn global_order_guaranteed(&self) -> bool {
        false
    }
}
