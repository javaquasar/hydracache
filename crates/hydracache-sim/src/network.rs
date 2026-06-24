use std::collections::{BTreeMap, BTreeSet, VecDeque};

use hydracache::{ClusterNodeId, ClusterNodeMessage, LogicalDuration, LogicalTime};

use crate::SimRng;

type LinkKey = (ClusterNodeId, ClusterNodeId);

/// Deterministic network fault for one directed link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkFault {
    /// Delay the next packet on this link.
    Delay(LogicalDuration),
    /// Drop the next packet on this link.
    Drop,
    /// Deliver the next packet twice.
    Duplicate,
    /// Delay the next packet by one millisecond, letting later packets overtake it.
    Reorder,
    /// Partition both directions between two sides.
    PartitionSym,
    /// Partition only this directed link.
    PartitionAsym,
}

/// Directionality for explicit partition calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionSymmetry {
    /// Block both directions between the two sides.
    Symmetric,
    /// Block messages from the left side to the right side only.
    LeftToRight,
    /// Block messages from the right side to the left side only.
    RightToLeft,
}

/// Packet currently in flight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedMessage {
    /// Source node.
    pub from: ClusterNodeId,
    /// Destination node.
    pub to: ClusterNodeId,
    /// Payload.
    pub message: ClusterNodeMessage,
    /// Logical delivery timestamp.
    pub deliver_at: LogicalTime,
    packet_id: u64,
}

#[derive(Debug, Clone, Default)]
struct LinkState {
    faults: VecDeque<LinkFault>,
}

/// Deterministic packet simulator.
#[derive(Debug, Clone)]
pub struct SimNetwork {
    rng: SimRng,
    in_flight: BTreeMap<(LogicalTime, u64), TimedMessage>,
    links: BTreeMap<LinkKey, LinkState>,
    partitions: BTreeSet<LinkKey>,
    next_packet_id: u64,
}

impl SimNetwork {
    /// Create an empty network from a seed.
    pub fn from_seed(seed: u64) -> Self {
        Self {
            rng: SimRng::from_seed(seed),
            in_flight: BTreeMap::new(),
            links: BTreeMap::new(),
            partitions: BTreeSet::new(),
            next_packet_id: 1,
        }
    }

    /// Queue a fault for the next packet on a directed link.
    pub fn inject_link_fault(
        &mut self,
        from: impl Into<ClusterNodeId>,
        to: impl Into<ClusterNodeId>,
        fault: LinkFault,
    ) {
        self.link_mut(from.into(), to.into())
            .faults
            .push_back(fault);
    }

    /// Deterministically inject one recoverable link fault from the network RNG.
    pub fn inject_recoverable_fault_from_rng(
        &mut self,
        from: impl Into<ClusterNodeId>,
        to: impl Into<ClusterNodeId>,
    ) -> LinkFault {
        let fault = match self.rng.next_index(4) {
            0 => LinkFault::Delay(LogicalDuration::from_millis(5)),
            1 => LinkFault::Drop,
            2 => LinkFault::Duplicate,
            _ => LinkFault::Reorder,
        };
        self.inject_link_fault(from, to, fault.clone());
        fault
    }

    /// Send one packet at logical time `now`.
    pub fn send(
        &mut self,
        from: ClusterNodeId,
        to: ClusterNodeId,
        message: ClusterNodeMessage,
        now: LogicalTime,
    ) {
        if self.partitions.contains(&(from.clone(), to.clone())) {
            return;
        }

        match self.pop_link_fault(from.clone(), to.clone()) {
            Some(LinkFault::Drop) | Some(LinkFault::PartitionAsym) => {}
            Some(LinkFault::PartitionSym) => {
                self.partition((&[from], &[to]), PartitionSymmetry::Symmetric);
            }
            Some(LinkFault::Duplicate) => {
                self.enqueue(from.clone(), to.clone(), message.clone(), now);
                self.enqueue(from, to, message, now);
            }
            Some(LinkFault::Delay(duration)) => {
                self.enqueue(from, to, message, now.saturating_add(duration));
            }
            Some(LinkFault::Reorder) => {
                self.enqueue(
                    from,
                    to,
                    message,
                    now.saturating_add(LogicalDuration::from_millis(1)),
                );
            }
            None => self.enqueue(from, to, message, now),
        }
    }

    /// Drain packets whose delivery time is not greater than `now`.
    pub fn deliverable(
        &mut self,
        now: LogicalTime,
    ) -> Vec<(ClusterNodeId, ClusterNodeId, ClusterNodeMessage)> {
        let ready_keys: Vec<_> = self
            .in_flight
            .keys()
            .take_while(|(deliver_at, _)| *deliver_at <= now)
            .cloned()
            .collect();

        let mut delivered = Vec::with_capacity(ready_keys.len());
        for key in ready_keys {
            if let Some(packet) = self.in_flight.remove(&key) {
                if !self
                    .partitions
                    .contains(&(packet.from.clone(), packet.to.clone()))
                {
                    delivered.push((packet.from, packet.to, packet.message));
                }
            }
        }
        delivered
    }

    /// Partition traffic between two node sets.
    pub fn partition(
        &mut self,
        sides: (&[ClusterNodeId], &[ClusterNodeId]),
        mode: PartitionSymmetry,
    ) {
        let (left, right) = sides;
        for from in left {
            for to in right {
                if matches!(
                    mode,
                    PartitionSymmetry::Symmetric | PartitionSymmetry::LeftToRight
                ) {
                    self.partitions.insert((from.clone(), to.clone()));
                }
                if matches!(
                    mode,
                    PartitionSymmetry::Symmetric | PartitionSymmetry::RightToLeft
                ) {
                    self.partitions.insert((to.clone(), from.clone()));
                }
            }
        }
    }

    /// Clear all active partitions.
    pub fn heal(&mut self) {
        self.partitions.clear();
    }

    /// Return the number of packets currently in flight.
    pub fn in_flight_len(&self) -> usize {
        self.in_flight.len()
    }

    /// Return whether a directed link can currently deliver packets.
    pub fn can_deliver(&self, from: &ClusterNodeId, to: &ClusterNodeId) -> bool {
        !self.partitions.contains(&(from.clone(), to.clone()))
    }

    /// Return packets currently in flight for one directed link.
    pub fn in_flight_between(&self, from: &ClusterNodeId, to: &ClusterNodeId) -> usize {
        self.in_flight
            .values()
            .filter(|packet| &packet.from == from && &packet.to == to)
            .count()
    }

    /// Return the largest pending delay on one directed link at `now`.
    pub fn max_pending_delay(
        &self,
        from: &ClusterNodeId,
        to: &ClusterNodeId,
        now: LogicalTime,
    ) -> Option<LogicalDuration> {
        self.in_flight
            .values()
            .filter(|packet| &packet.from == from && &packet.to == to && packet.deliver_at > now)
            .map(|packet| {
                LogicalDuration::from_millis(
                    packet
                        .deliver_at
                        .as_millis()
                        .saturating_sub(now.as_millis()),
                )
            })
            .max()
    }

    fn enqueue(
        &mut self,
        from: ClusterNodeId,
        to: ClusterNodeId,
        message: ClusterNodeMessage,
        deliver_at: LogicalTime,
    ) {
        let packet_id = self.next_packet_id;
        self.next_packet_id = self.next_packet_id.saturating_add(1);
        self.in_flight.insert(
            (deliver_at, packet_id),
            TimedMessage {
                from,
                to,
                message,
                deliver_at,
                packet_id,
            },
        );
    }

    fn pop_link_fault(&mut self, from: ClusterNodeId, to: ClusterNodeId) -> Option<LinkFault> {
        self.links
            .get_mut(&(from, to))
            .and_then(|link| link.faults.pop_front())
    }

    fn link_mut(&mut self, from: ClusterNodeId, to: ClusterNodeId) -> &mut LinkState {
        self.links.entry((from, to)).or_default()
    }
}
