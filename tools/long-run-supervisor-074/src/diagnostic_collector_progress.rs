//! One consumed heartbeat preflight; no state transition, runtime lease or start.
use super::*;
use crate::diagnostic_ipc::{DiagnosticOperation, DiagnosticRequest};
use crate::diagnostic_kernel_admission::KernelPacket;

#[derive(Debug, thiserror::Error)]
pub enum CollectorProgressError {
    #[error("original collector progress permanently refused")]
    Refused,
    #[error("original collector reservation refused: {0}")]
    Reservation(#[from] CollectorReservedError),
    #[error("original authenticated progress input refused: {0}")]
    Input(#[from] TrustInputError),
    #[error("authenticated progress differs from the original reservation")]
    Binding,
}

fn require_binding(
    original: &DiagnosticRequest,
    next: &DiagnosticRequest,
    state: &DiagnosticState,
) -> Result<(), CollectorProgressError> {
    if next.operation != DiagnosticOperation::Heartbeat
        || next.expected_state_revision != state.revision
        || state.revision != 1
        || state.stage != DiagnosticStage::Reserved
        || next.identity != original.identity
        || next.controller != original.controller
        || next.source_commit != original.source_commit
        || next.preset != original.preset
        || next.request_id == original.request_id
        || next.nonce_sha256 == original.nonce_sha256
    {
        return Err(CollectorProgressError::Binding);
    }
    Ok(())
}

/// Holds both original owners. Revalidation never refreshes the heartbeat's receive time.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::OriginalCollectorProgress;
/// fn start(mut p: OriginalCollectorProgress<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::OriginalCollectorProgress;
/// fn replace(mut p: OriginalCollectorProgress<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.revalidate(b"replacement"); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::OriginalCollectorProgress;
/// fn clone(p: OriginalCollectorProgress<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::OriginalCollectorProgress;
/// use hydracache_long_run_supervisor_074::diagnostic_kernel_admission::KernelPacket;
/// fn again(p: OriginalCollectorProgress<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>, next: KernelPacket) { p.bind_authenticated_progress(next); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::OriginalCollectorProgress;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<OriginalCollectorProgress<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
pub struct OriginalCollectorProgress<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    reservation: OriginalCollectorReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    packet: KernelPacket,
    after: KernelTimePoint,
}
impl fmt::Debug for OriginalCollectorProgress<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalCollectorProgress")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl OriginalCollectorProgress<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.reservation.is_refused() || self.packet.is_refused()
    }
    fn refuse(&mut self) {
        self.reservation.refuse();
        self.packet.latch_refusal();
    }
    fn progress(&mut self) -> Result<(), CollectorProgressError> {
        let controller = &mut self.reservation.original.original.original;
        let next = controller.worker.authenticate_original_progress(
            &mut self.packet,
            &controller.controller.source.policy,
            &self.after,
        )?;
        require_binding(&controller.request, &next, &self.reservation.state)?;
        // Total lease remains anchored to the frozen receiver-observed reservation.
        if self
            .reservation
            .last
            .boottime_ns()
            .checked_sub(self.reservation.state.reserved_monotonic_ns)
            .filter(|age| *age < crate::diagnostic_lease::TOTAL_SECONDS * 1_000_000_000)
            .is_none()
        {
            return Err(CollectorProgressError::Binding);
        }
        Ok(())
    }
    pub fn revalidate(&mut self) -> Result<(), CollectorProgressError> {
        if self.is_refused() {
            self.refuse();
            return Err(CollectorProgressError::Refused);
        }
        let result = (|| {
            self.reservation.revalidate()?;
            self.progress()?;
            self.reservation.revalidate()?;
            // Check age/signature again after the potentially slow original-chain checks.
            self.progress()
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
}

pub struct CollectorProgressFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: CollectorProgressError,
    held: Box<OriginalCollectorProgress<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl CollectorProgressFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &CollectorProgressError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.held.is_refused()
    }
}
impl fmt::Debug for CollectorProgressFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for CollectorProgressFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for CollectorProgressFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {}

impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCollectorReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    /// Consume one reservation and one fresh kernel packet without publishing or starting.
    /// ```compile_fail
    /// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::OriginalCollectorReservation;
    /// use hydracache_long_run_supervisor_074::diagnostic_kernel_admission::KernelPacket;
    /// fn twice(r: OriginalCollectorReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>, a: KernelPacket, b: KernelPacket) {
    ///     let first = r.bind_authenticated_progress(a); let second = r.bind_authenticated_progress(b);
    /// }
    /// ```
    pub fn bind_authenticated_progress(
        self,
        packet: KernelPacket,
    ) -> Result<
        OriginalCollectorProgress<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        CollectorProgressFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let after = self.last.clone();
        let mut read = OriginalCollectorProgress {
            reservation: self,
            packet,
            after,
        };
        match read.revalidate() {
            Ok(()) => Ok(read),
            Err(error) => {
                read.refuse();
                Err(CollectorProgressFailure {
                    error,
                    held: Box::new(read),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collector_progress_seeded_original_binding_mutations() {
        // Pure binding checks only; signatures/kernel observations have separate tests.
        let original: DiagnosticRequest = serde_json::from_value(serde_json::json!({
            "schema_version":1,"request_id":"a".repeat(64),"nonce_sha256":"b".repeat(64),
            "operation":"reserve","expected_state_revision":0,
            "identity":{"lease_id":"c".repeat(64),"boot_id":"boot","binary_sha256":"d".repeat(64),"build_provenance_sha256":"e".repeat(64)},
            "source_commit":"source","preset":"preset",
            "controller":{"repository_id":1,"run_id":2,"run_attempt":1,"actor_id":3},
            "issued_at_unix_seconds":1,"expires_at_unix_seconds":61
        })).unwrap();
        let state = DiagnosticState {
            identity: original.identity.clone(),
            revision: 1,
            stage: DiagnosticStage::Reserved,
            completed_cells: 0,
            reserved_monotonic_ns: 1,
            last_observed_monotonic_ns: 1,
            controller_monotonic_ns: 1,
            cell_started_monotonic_ns: None,
            cgroup_inode: None,
            reason: None,
            cleanup_confirmed: true,
            promotable: false,
            admission_allowed: false,
        };
        let mut next = original.clone();
        next.operation = DiagnosticOperation::Heartbeat;
        next.expected_state_revision = 1;
        next.request_id = "f".repeat(64);
        next.nonce_sha256 = "0".repeat(64);
        require_binding(&original, &next, &state).unwrap();
        let mut cases: [u8; 12] = std::array::from_fn(|i| i as u8);
        let mut seed = 0x0740c09c_u64;
        for i in (1..cases.len()).rev() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            cases.swap(i, seed as usize % (i + 1));
        }
        for case in cases {
            let mut n = next.clone();
            let mut s = state.clone();
            match case {
                0 => n.operation = DiagnosticOperation::Cancel,
                1 => n.operation = DiagnosticOperation::Status,
                2 => n.expected_state_revision = 2,
                3 => n.identity.lease_id = "f".repeat(64),
                4 => n.identity.binary_sha256 = "f".repeat(64),
                5 => n.controller.run_id += 1,
                6 => n.controller.run_attempt += 1,
                7 => n.source_commit.push('x'),
                8 => n.preset.push('x'),
                9 => n.request_id = original.request_id.clone(),
                10 => n.nonce_sha256 = original.nonce_sha256.clone(),
                _ => s.stage = DiagnosticStage::Starting,
            }
            assert!(
                matches!(
                    require_binding(&original, &n, &s),
                    Err(CollectorProgressError::Binding)
                ),
                "case={case}"
            );
        }
        println!("collector_progress seed=0x0740c09c binding_mutations=12");
    }
}
