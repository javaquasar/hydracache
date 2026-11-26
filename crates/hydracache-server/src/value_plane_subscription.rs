//! Bounded polling of certified partition histories for one logical HC/2 subscription.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{stream, StreamExt};
use hydracache::{ClusterEpoch, PartitionId};
use hydracache_client_hc2::wire::{EventGapDetails, EventGapReason, PartitionGap};
use serde::{Deserialize, Serialize};
use tokio::time::{timeout, MissedTickBehavior};

use crate::imap_hc2_route::RoutedValueBackend;
use crate::imap_topology::PartitionMember;
use crate::imap_value_journal::hash;
use hydracache_cluster_raft::SledRaftLogStore;

pub(crate) type MemberValuePlane = Arc<RoutedValueBackend<SledRaftLogStore>>;
pub(crate) const MAX_CLUSTER_SUBSCRIPTIONS: usize = 32;
pub(crate) const MAX_PREFIX_BYTES: usize = 4096;
pub(crate) const MAX_EVENT_PAGE: usize = 8;
pub(crate) const EVENT_SCHEMA: u16 = 1;
const POLL_INTERVAL: Duration = Duration::from_millis(40);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct EventCursor {
    pub partition: PartitionId,
    pub after: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EventPoll {
    pub event_schema: u16,
    pub scope: [u8; 32],
    pub epoch: ClusterEpoch,
    pub source: PartitionMember,
    pub target: PartitionMember,
    pub cursors: Vec<EventCursor>,
    pub key_prefix: Vec<u8>,
    pub include_value: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CertifiedEvent {
    pub version: u64,
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>,
    pub removed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EventProgress {
    pub partition: PartitionId,
    pub through: u64,
    pub gap: bool,
    pub events: Vec<CertifiedEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EventPage {
    pub fingerprint: [u8; 32],
    pub progress: Vec<EventProgress>,
}

pub(crate) struct ListenerView {
    pub epoch: ClusterEpoch,
    pub partitions: BTreeMap<PartitionId, (Option<PartitionMember>, u64)>,
}

pub(crate) enum ClusterEvent {
    Gap(EventGapDetails),
    Mutation(CertifiedEvent),
}

pub(crate) struct ClusterSubscriptionCoordinator {
    plane: MemberValuePlane,
    epoch: ClusterEpoch,
    cursors: BTreeMap<PartitionId, u64>,
    owners: BTreeMap<PartitionId, Option<PartitionMember>>,
    turn: usize,
    key_prefix: Vec<u8>,
    include_value: bool,
}

impl ClusterSubscriptionCoordinator {
    pub fn new(
        plane: MemberValuePlane,
        view: &ListenerView,
        key_prefix: Vec<u8>,
        include_value: bool,
    ) -> Self {
        Self {
            plane,
            epoch: view.epoch,
            cursors: frontier(view),
            owners: owners(view),
            turn: 0,
            key_prefix,
            include_value,
        }
    }

    pub async fn run<E, F>(mut self, mut emit: impl FnMut(ClusterEvent) -> F) -> Result<(), E>
    where
        F: std::future::Future<Output = Result<(), E>>,
    {
        let mut tick = tokio::time::interval(POLL_INTERVAL);
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let Some(view) = self.plane.listener_view() else {
                continue;
            };
            if self.epoch != view.epoch {
                let details = topology_gap(&self.owners, &mut self.cursors, &view);
                emit(ClusterEvent::Gap(details)).await?;
                self.epoch = view.epoch;
                self.owners = owners(&view);
            }
            let selected = view
                .partitions
                .iter()
                .filter_map(|(&partition, (owner, latest))| {
                    let after = self.cursors.get(&partition).copied().unwrap_or(0);
                    (*latest > after).then_some((owner.as_ref(), EventCursor { partition, after }))
                })
                .collect::<Vec<_>>();
            if selected.is_empty() {
                continue;
            }
            let mut groups = BTreeMap::new();
            for offset in 0..selected.len().min(MAX_EVENT_PAGE) {
                let (owner, cursor) = selected[(self.turn + offset) % selected.len()];
                if let Some(owner) = owner {
                    groups
                        .entry(owner.node.clone())
                        .or_insert_with(|| (owner.clone(), Vec::new()))
                        .1
                        .push(cursor);
                }
            }
            self.turn = (self.turn + MAX_EVENT_PAGE) % selected.len();
            let requests: Vec<_> = groups
                .into_values()
                .map(|(target, cursors)| EventPoll {
                    event_schema: EVENT_SCHEMA,
                    scope: self.plane.event_scope(),
                    epoch: view.epoch,
                    source: self.plane.local_member().clone(),
                    target,
                    cursors,
                    key_prefix: self.key_prefix.clone(),
                    include_value: self.include_value,
                })
                .collect();
            let mut pages = stream::iter(requests)
                .map(|request| {
                    let plane = self.plane.clone();
                    async move {
                        let fingerprint = hash(&request);
                        let page =
                            timeout(Duration::from_millis(600), plane.fetch_events(&request))
                                .await
                                .ok()?
                                .ok()?;
                        (page.fingerprint == fingerprint && valid_page(&request, &page))
                            .then_some(page)
                    }
                })
                .buffer_unordered(4);
            while let Some(page) = pages.next().await {
                let Some(page) = page else { continue };
                for item in page.progress {
                    if item.gap {
                        emit(ClusterEvent::Gap(EventGapDetails {
                            reason: EventGapReason::HistoryLost as i32,
                            partitions: vec![PartitionGap {
                                partition_id: item.partition.value(),
                                previous_version: self
                                    .cursors
                                    .get(&item.partition)
                                    .copied()
                                    .unwrap_or(0),
                                observed_version: item.through,
                            }],
                        }))
                        .await?;
                    }
                    for event in item.events {
                        emit(ClusterEvent::Mutation(event)).await?;
                    }
                    self.cursors.insert(item.partition, item.through);
                }
            }
        }
    }
}

fn owners(view: &ListenerView) -> BTreeMap<PartitionId, Option<PartitionMember>> {
    view.partitions
        .iter()
        .map(|(&id, (owner, _))| (id, owner.clone()))
        .collect()
}

fn topology_gap(
    previous_owners: &BTreeMap<PartitionId, Option<PartitionMember>>,
    cursors: &mut BTreeMap<PartitionId, u64>,
    view: &ListenerView,
) -> EventGapDetails {
    let partitions = view
        .partitions
        .iter()
        .filter_map(|(&id, (owner, observed))| {
            let previous = cursors.get(&id).copied().unwrap_or(0);
            if previous > *observed {
                cursors.insert(id, *observed);
            }
            (previous_owners.get(&id) != Some(owner) || previous > *observed).then_some(
                PartitionGap {
                    partition_id: id.value(),
                    previous_version: previous,
                    observed_version: *observed,
                },
            )
        })
        .collect();
    EventGapDetails {
        reason: EventGapReason::TopologyChanged as i32,
        partitions,
    }
}

fn frontier(view: &ListenerView) -> BTreeMap<PartitionId, u64> {
    view.partitions
        .iter()
        .map(|(&partition, (_, version))| (partition, *version))
        .collect()
}

fn valid_page(request: &EventPoll, page: &EventPage) -> bool {
    if page.progress.len() > request.cursors.len()
        || page
            .progress
            .iter()
            .map(|item| item.events.len())
            .sum::<usize>()
            > MAX_EVENT_PAGE
    {
        return false;
    }
    let mut seen = std::collections::BTreeSet::new();
    page.progress.iter().all(|progress| {
        let Some(cursor) = request
            .cursors
            .iter()
            .find(|cursor| cursor.partition == progress.partition)
        else {
            return false;
        };
        let mut previous = cursor.after;
        seen.insert(progress.partition)
            && progress.through > cursor.after
            && (progress.gap || progress.through - cursor.after <= MAX_EVENT_PAGE as u64)
            && (!progress.gap || progress.events.is_empty())
            && progress.events.iter().all(|event| {
                let valid = event.version > previous
                    && event.version <= progress.through
                    && event.key.starts_with(&request.key_prefix)
                    && (request.include_value || event.value.is_none())
                    && (!event.removed || event.value.is_none());
                previous = event.version;
                valid
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydracache::{ClusterGeneration, ClusterNodeId};

    #[test]
    fn owner_pages_reject_duplicates_stale_versions_and_unrequested_values() {
        let member = PartitionMember {
            node: ClusterNodeId::new("owner"),
            generation: ClusterGeneration::new(1),
        };
        let partition = PartitionId::new(9);
        let mut request = EventPoll {
            event_schema: EVENT_SCHEMA,
            scope: [1; 32],
            epoch: ClusterEpoch::new(3),
            source: member.clone(),
            target: member,
            cursors: vec![EventCursor {
                partition,
                after: 4,
            }],
            key_prefix: b"prefix/".to_vec(),
            include_value: false,
        };
        let event = |version| CertifiedEvent {
            version,
            key: b"prefix/key".to_vec(),
            value: None,
            removed: false,
        };
        let mut page = EventPage {
            fingerprint: [0; 32],
            progress: vec![EventProgress {
                partition,
                through: 6,
                gap: false,
                events: vec![event(5), event(6)],
            }],
        };
        assert!(valid_page(&request, &page));
        page.progress[0].events.push(event(6));
        assert!(!valid_page(&request, &page));
        page.progress[0].events.pop();
        page.progress[0].events[0].value = Some(vec![1]);
        assert!(!valid_page(&request, &page));
        request.include_value = true;
        assert!(valid_page(&request, &page));
        page.progress[0].gap = true;
        assert!(!valid_page(&request, &page));
        page.progress[0].events.clear();
        page.progress[0].through = 30;
        assert!(valid_page(&request, &page));
    }
}
