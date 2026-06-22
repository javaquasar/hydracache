use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use hydracache::ClusterNodeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    Partition {
        from: ClusterNodeId,
        to: ClusterNodeId,
        symmetric: bool,
    },
    Latency {
        node: ClusterNodeId,
        latency: Duration,
    },
}

#[derive(Debug, Clone)]
pub struct FaultInjector {
    state: u64,
    dropped_edges: BTreeSet<(ClusterNodeId, ClusterNodeId)>,
    latency: BTreeMap<ClusterNodeId, Duration>,
}

impl FaultInjector {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed,
            dropped_edges: BTreeSet::new(),
            latency: BTreeMap::new(),
        }
    }

    pub fn next_fault(&mut self, nodes: &[ClusterNodeId]) -> Option<Fault> {
        if nodes.len() < 2 {
            return None;
        }
        let left = self.next_index(nodes.len());
        let mut right = self.next_index(nodes.len());
        if left == right {
            right = (right + 1) % nodes.len();
        }
        let symmetric = self.next_bool();
        Some(Fault::Partition {
            from: nodes[left].clone(),
            to: nodes[right].clone(),
            symmetric,
        })
    }

    pub fn partition(
        &mut self,
        from: impl Into<ClusterNodeId>,
        to: impl Into<ClusterNodeId>,
        symmetric: bool,
    ) {
        let from = from.into();
        let to = to.into();
        self.dropped_edges.insert((from.clone(), to.clone()));
        if symmetric {
            self.dropped_edges.insert((to, from));
        }
    }

    pub fn can_deliver(&self, from: &ClusterNodeId, to: &ClusterNodeId) -> bool {
        !self.dropped_edges.contains(&(from.clone(), to.clone()))
    }

    pub fn inject_latency(&mut self, node: impl Into<ClusterNodeId>, latency: Duration) {
        self.latency.insert(node.into(), latency);
    }

    pub fn observed_latency(&self, node: &ClusterNodeId) -> Duration {
        self.latency.get(node).copied().unwrap_or_default()
    }

    fn next_index(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() % 2 == 0
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }
}
