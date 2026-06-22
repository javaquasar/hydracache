use super::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Transport-security posture declared for a controlled cluster pilot.
///
/// HydraCache does not terminate TLS or manage certificates. This structure is
/// a loud, machine-readable contract that says whether the embedded HTTP
/// transport is protected by HydraCache auth headers or by an explicitly
/// declared external mesh/mTLS boundary.
pub struct TransportPosture {
    /// Whether HydraCache transport auth is configured on routes and clients.
    pub auth: bool,
    /// Whether strict current wire-version compatibility is enforced.
    pub wire_strict: bool,
    /// Whether an operator declared that an external mesh/mTLS boundary handles
    /// identity and transport security.
    pub mesh_declared: bool,
}

impl TransportPosture {
    /// Create a posture from explicit booleans.
    pub const fn new(auth: bool, wire_strict: bool, mesh_declared: bool) -> Self {
        Self {
            auth,
            wire_strict,
            mesh_declared,
        }
    }

    /// Return whether the posture is acceptable for the 0.40 pilot gate.
    pub fn is_safe(&self) -> bool {
        (self.auth && self.wire_strict) || self.mesh_declared
    }

    /// Return the actuator highlight for an unsafe missing-auth posture.
    pub fn highlight(&self) -> Option<&'static str> {
        if !self.auth && !self.mesh_declared {
            Some("AUTH MISSING")
        } else {
            None
        }
    }
}

/// Client-side routing behavior for owner peer-fetch traffic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingMode {
    /// Smart routing: resolve the owner for each key and contact that owner.
    #[default]
    Direct,
    /// Unisocket routing: always send owner traffic through a configured
    /// gateway/single endpoint.
    SingleEndpoint,
}

/// Minimal epoch fence for topology-authoritative decisions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyFence {
    committed_epoch: ClusterEpoch,
}

impl TopologyFence {
    /// Create a fence at the provided committed epoch.
    pub const fn new(committed_epoch: ClusterEpoch) -> Self {
        Self { committed_epoch }
    }

    /// Return the latest committed topology epoch known to this fence.
    pub fn committed_epoch(&self) -> ClusterEpoch {
        self.committed_epoch
    }

    /// Return whether a message stamped with `msg_epoch` is still admissible.
    pub fn admit(&self, msg_epoch: ClusterEpoch) -> bool {
        msg_epoch >= self.committed_epoch
    }

    /// Advance the fence. Older epochs never move it backwards.
    pub fn commit(&mut self, epoch: ClusterEpoch) {
        if epoch > self.committed_epoch {
            self.committed_epoch = epoch;
        }
    }
}
