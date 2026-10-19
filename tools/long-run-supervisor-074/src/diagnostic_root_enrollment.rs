//! Versioned expected root enrollment bytes, never a live Armed capability.
//! No supplied digest, inode, deadline or ACK authenticates a runtime owner.
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_two_phase_launch::root_enrollment::RootBindingClaims;
//! fn launch(claims: RootBindingClaims) { claims.start_worker(); }
//! ```
pub const PACKET_BYTES: usize = 104;
pub const PROTOCOL: &str = "hc074-cgroup-root-enrollment-v2";
const MAGIC: &[u8; 8] = b"HCRG0742";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootBindingClaims {
    pub attempt_sha256: [u8; 32],
    pub root_device: u64,
    pub root_inode: u64,
    pub deadline_boottime_ns: u64,
    pub original_context_sha256: [u8; 32],
}
impl RootBindingClaims {
    fn validate(&self) -> Result<(), PacketError> {
        if self.attempt_sha256 == [0; 32]
            || self.original_context_sha256 == [0; 32]
            || self.root_device == 0
            || self.root_inode == 0
            || self.deadline_boottime_ns == 0
            || self.deadline_boottime_ns > i64::MAX as u64
        {
            return Err(PacketError);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Enroll = 1,
    Armed = 2,
    Refused = 3,
    Revoking = 4,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketClaims {
    pub kind: Kind,
    pub binding: RootBindingClaims,
}
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[error("root enrollment packet or claimed exchange refused")]
pub struct PacketError;

impl PacketClaims {
    pub fn encode(&self) -> Result<[u8; PACKET_BYTES], PacketError> {
        self.binding.validate()?;
        let mut bytes = [0; PACKET_BYTES];
        bytes[..8].copy_from_slice(MAGIC);
        bytes[8] = self.kind as u8;
        bytes[16..48].copy_from_slice(&self.binding.attempt_sha256);
        bytes[48..56].copy_from_slice(&self.binding.root_device.to_le_bytes());
        bytes[56..64].copy_from_slice(&self.binding.root_inode.to_le_bytes());
        bytes[64..72].copy_from_slice(&self.binding.deadline_boottime_ns.to_le_bytes());
        bytes[72..104].copy_from_slice(&self.binding.original_context_sha256);
        Ok(bytes)
    }
}
/// Byte inspection only. Peer credentials and FD role admission are separate.
pub fn inspect_packet(bytes: &[u8]) -> Result<PacketClaims, PacketError> {
    if bytes.len() != PACKET_BYTES || &bytes[..8] != MAGIC || bytes[9..16] != [0; 7] {
        return Err(PacketError);
    }
    let kind = match bytes[8] {
        1 => Kind::Enroll,
        2 => Kind::Armed,
        3 => Kind::Refused,
        4 => Kind::Revoking,
        _ => return Err(PacketError),
    };
    let word = |start| u64::from_le_bytes(bytes[start..start + 8].try_into().unwrap());
    let binding = RootBindingClaims {
        attempt_sha256: bytes[16..48].try_into().unwrap(),
        root_device: word(48),
        root_inode: word(56),
        deadline_boottime_ns: word(64),
        original_context_sha256: bytes[72..104].try_into().unwrap(),
    };
    binding.validate()?;
    Ok(PacketClaims { kind, binding })
}

/// Pure one-shot exchange accounting. An inspected ACK is still only a claim.
pub struct ClaimExchange {
    expected: RootBindingClaims,
    consumed: bool,
    acknowledged: bool,
    refused: bool,
}
impl ClaimExchange {
    pub fn new(expected: RootBindingClaims) -> Result<Self, PacketError> {
        expected.validate()?;
        Ok(Self {
            expected,
            consumed: false,
            acknowledged: false,
            refused: false,
        })
    }
    /// Must precede the first future send syscall, even when that syscall fails.
    pub fn prepare_first_send(&mut self) -> Result<[u8; PACKET_BYTES], PacketError> {
        if self.refused || self.consumed {
            self.refused = true;
            return Err(PacketError);
        }
        self.consumed = true;
        PacketClaims {
            kind: Kind::Enroll,
            binding: self.expected,
        }
        .encode()
    }
    /// No credentials or rights are authenticated by this data-only method.
    pub fn inspect_armed_ack_claim(&mut self, bytes: &[u8]) -> Result<(), PacketError> {
        let expected = PacketClaims {
            kind: Kind::Armed,
            binding: self.expected,
        };
        if self.refused
            || !self.consumed
            || self.acknowledged
            || inspect_packet(bytes) != Ok(expected)
        {
            self.refused = true;
            return Err(PacketError);
        }
        self.acknowledged = true;
        Ok(())
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[allow(dead_code)]
#[path = "diagnostic_cgroup_root.rs"]
mod kernel;

#[cfg(test)]
#[path = "diagnostic_root_enrollment_tests.rs"]
mod tests;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) use kernel::{BoundGuardians, RootSource};
