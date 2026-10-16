//! Post-ack challenge response only; no lifecycle handoff, commit or execution.
use super::*;
#[path = "diagnostic_original_start_consent.rs"]
pub mod start_consent;

use crate::diagnostic_ipc::SignedDiagnosticRequest;
use crate::diagnostic_lease::DiagnosticIdentity;
use ring::rand::{SecureRandom, SystemRandom};

pub const CHALLENGE_SCHEMA: &str = "diagnostic-collector-progress-challenge-074-v1";
pub const CHALLENGE_DOMAIN: &[u8] = b"hydracache-diagnostic-collector-progress-challenge-074-v1";
pub const MAX_CHALLENGE_BYTES: usize = 4096;

#[derive(Debug, thiserror::Error)]
pub enum CollectorCausalError {
    #[error("original causal progress permanently refused")]
    Refused,
    #[error("original reservation refused: {0}")]
    Reservation(#[from] CollectorReservedError),
    #[error("original progress refused: {0}")]
    Progress(#[from] CollectorProgressError),
    #[error("original kernel/worker input refused: {0}")]
    Input(#[from] TrustInputError),
    #[error("system entropy unavailable; challenge cannot be issued")]
    Entropy,
    #[error("challenge document is malformed, noncanonical or over budget")]
    Document,
    #[error("challenge or authenticated response differs from its original owner")]
    Binding,
    #[error("original challenge freshness deadline expired or clock differs")]
    Time,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: String,
    identity: DiagnosticIdentity,
    expected_state_revision: u64,
    controller_attempt_sha256: String,
    collector_reserved_sha256: String,
    reserved_state_sha256: String,
    random_hex: String,
    issued_boottime_ns: u64,
}
fn encode(record: &Record) -> Result<Vec<u8>, CollectorCausalError> {
    let mut raw = canonical_json(record).map_err(|_| CollectorCausalError::Document)?;
    raw.push(b'\n');
    Ok(raw)
}
fn decode(raw: &[u8]) -> Result<Record, CollectorCausalError> {
    if raw.is_empty() || raw.len() > MAX_CHALLENGE_BYTES {
        return Err(CollectorCausalError::Document);
    }
    let record: Record = serde_json::from_slice(raw).map_err(|_| CollectorCausalError::Document)?;
    if record.schema_version != CHALLENGE_SCHEMA
        || encode(&record)? != raw
        || record.expected_state_revision != 1
        || record.issued_boottime_ns == 0
        || crate::diagnostic_lease::validate_identity(&record.identity).is_err()
        || [
            &record.controller_attempt_sha256,
            &record.collector_reserved_sha256,
            &record.reserved_state_sha256,
            &record.random_hex,
        ]
        .iter()
        .any(|s| !is_hash(s))
    {
        return Err(CollectorCausalError::Document);
    }
    Ok(record)
}
/// Bounded canonical byte inspection/digest only, not authentication or a guard factory.
pub fn collector_challenge_nonce(raw: &[u8]) -> Result<String, CollectorCausalError> {
    decode(raw)?;
    let mut message = CHALLENGE_DOMAIN.to_vec();
    message.push(0);
    message.extend_from_slice(raw);
    Ok(sha256_hex(&message))
}
fn age(issued: u64, now: u64) -> Result<(), CollectorCausalError> {
    if issued == 0
        || now < issued
        || now - issued >= crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS * 1_000_000_000
    {
        return Err(CollectorCausalError::Time);
    }
    Ok(())
}
fn window(
    reservation: &mut OriginalCollectorReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    issued: &KernelTimePoint,
) -> Result<(), CollectorCausalError> {
    let now = reservation
        .original
        .original
        .original
        .worker
        .original_reservation_time(Some(issued))?;
    if now.boot_id() != issued.boot_id() {
        return Err(CollectorCausalError::Time);
    }
    age(issued.boottime_ns(), now.boottime_ns())
}
fn check(
    reservation: &OriginalCollectorReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    raw: &[u8],
    nonce: &str,
    issued: &KernelTimePoint,
) -> Result<(), CollectorCausalError> {
    let record = decode(raw)?;
    if collector_challenge_nonce(raw)? != nonce
        || record.controller_attempt_sha256 != sha256_hex(&reservation.request_raw)
        || record.collector_reserved_sha256 != sha256_hex(&reservation.raw)
        || record.reserved_state_sha256
            != sha256_hex(
                &canonical_json(&reservation.state).map_err(|_| CollectorCausalError::Document)?,
            )
        || record.identity != reservation.state.identity
        || record.identity.boot_id != issued.boot_id()
        || record.issued_boottime_ns != issued.boottime_ns()
        || reservation.state.revision != 1
        || reservation.state.stage != DiagnosticStage::Reserved
        || nonce == reservation.original.original.original.request.nonce_sha256
    {
        return Err(CollectorCausalError::Binding);
    }
    Ok(())
}

/// Original owner with one immutable post-ack challenge. Exported bytes are data only.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::OriginalCollectorChallenge;
/// fn clone(c: OriginalCollectorChallenge<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { c.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::OriginalCollectorChallenge;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<OriginalCollectorChallenge<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
pub struct OriginalCollectorChallenge<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    reservation: OriginalCollectorReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    raw: Vec<u8>,
    nonce: String,
    issued: Option<KernelTimePoint>,
}
impl fmt::Debug for OriginalCollectorChallenge<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalCollectorChallenge")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCollectorChallenge<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub fn is_refused(&self) -> bool {
        self.reservation.is_refused()
    }
    /// May be stale; this immutable copy/export is not fresh permission or acknowledgement.
    pub fn challenge_to_sign(&self) -> &[u8] {
        &self.raw
    }
    fn refuse(&mut self) {
        self.reservation.refuse();
    }
    fn prepare(
        &mut self,
        random: impl FnOnce() -> Result<[u8; 32], CollectorCausalError>,
    ) -> Result<(), CollectorCausalError> {
        self.reservation.revalidate()?;
        let random = random()?;
        let issued = self
            .reservation
            .original
            .original
            .original
            .worker
            .original_reservation_time(Some(&self.reservation.last))?;
        let raw = encode(&Record {
            schema_version: CHALLENGE_SCHEMA.into(),
            identity: self.reservation.state.identity.clone(),
            expected_state_revision: 1,
            controller_attempt_sha256: sha256_hex(&self.reservation.request_raw),
            collector_reserved_sha256: sha256_hex(&self.reservation.raw),
            reserved_state_sha256: sha256_hex(
                &canonical_json(&self.reservation.state)
                    .map_err(|_| CollectorCausalError::Document)?,
            ),
            random_hex: crate::hex(&random),
            issued_boottime_ns: issued.boottime_ns(),
        })?;
        self.nonce = collector_challenge_nonce(&raw)?;
        self.raw = raw;
        self.issued = Some(issued);
        self.reservation.revalidate()?;
        let issued = self.issued.as_ref().expect("issued after original checks");
        check(&self.reservation, &self.raw, &self.nonce, issued)?;
        window(&mut self.reservation, issued)
    }
    /// Consume this one challenge and one original kernel response; no retry or start.
    /// ```compile_fail
    /// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::OriginalCollectorChallenge;
    /// use hydracache_long_run_supervisor_074::diagnostic_kernel_admission::KernelPacket;
    /// fn twice(c: OriginalCollectorChallenge<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>, a: KernelPacket, b: KernelPacket) {
    ///     let first = c.accept_response(a); let second = c.accept_response(b);
    /// }
    /// ```
    pub fn accept_response(
        mut self,
        mut packet: KernelPacket,
    ) -> Result<
        OriginalCausalProgress<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        CollectorCausalFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let prepared = (|| {
            if self.is_refused() || packet.is_refused() {
                return Err(CollectorCausalError::Refused);
            }
            let issued = self.issued.as_ref().ok_or(CollectorCausalError::Binding)?;
            check(&self.reservation, &self.raw, &self.nonce, issued)?;
            window(&mut self.reservation, issued)
        })();
        if let Err(error) = prepared {
            self.refuse();
            packet.latch_refusal();
            return Err(CollectorCausalFailure {
                error,
                held: Held::Challenge(Box::new(self), Some(Box::new(packet))),
            });
        }
        let issued = self.issued.expect("checked original challenge issuance");
        let mut read = OriginalCausalProgress {
            original: OriginalCollectorProgress {
                reservation: self.reservation,
                packet,
                after: issued.clone(),
            },
            raw: self.raw,
            nonce: self.nonce,
            issued,
        };
        match read.revalidate() {
            Ok(()) => Ok(read),
            Err(error) => {
                read.refuse();
                Err(CollectorCausalFailure {
                    error,
                    held: Held::Progress(Box::new(read)),
                })
            }
        }
    }
}

/// Original authenticated response bound to its post-ack challenge, never start authority.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::OriginalCausalProgress;
/// fn start(mut c: OriginalCausalProgress<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { c.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::OriginalCausalProgress;
/// fn replace(mut c: OriginalCausalProgress<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { c.revalidate(b"replacement"); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::OriginalCausalProgress;
/// use hydracache_long_run_supervisor_074::diagnostic_kernel_admission::KernelPacket;
/// fn again(c: OriginalCausalProgress<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>, p: KernelPacket) { c.accept_response(p); }
/// ```
pub struct OriginalCausalProgress<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    original: OriginalCollectorProgress<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    raw: Vec<u8>,
    nonce: String,
    issued: KernelTimePoint,
}
impl fmt::Debug for OriginalCausalProgress<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalCausalProgress")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl OriginalCausalProgress<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.original.is_refused()
    }
    fn refuse(&mut self) {
        self.original.refuse();
    }
    pub fn revalidate(&mut self) -> Result<(), CollectorCausalError> {
        if self.is_refused() {
            self.refuse();
            return Err(CollectorCausalError::Refused);
        }
        let result = (|| {
            self.original.revalidate()?;
            check(
                &self.original.reservation,
                &self.raw,
                &self.nonce,
                &self.issued,
            )?;
            // The unchanged original guard authenticated this same immutable owned packet.
            let response: SignedDiagnosticRequest =
                serde_json::from_slice(self.original.packet.bytes())
                    .map_err(|_| CollectorCausalError::Document)?;
            if response.request.nonce_sha256 != self.nonce {
                return Err(CollectorCausalError::Binding);
            }
            window(&mut self.original.reservation, &self.issued)
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
}
enum Held<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    Challenge(
        Box<OriginalCollectorChallenge<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
        Option<Box<KernelPacket>>,
    ),
    Progress(Box<OriginalCausalProgress<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>),
}
pub struct CollectorCausalFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: CollectorCausalError,
    held: Held<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
}
impl CollectorCausalFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &CollectorCausalError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        match &self.held {
            Held::Challenge(c, p) => c.is_refused() || p.as_ref().is_some_and(|p| p.is_refused()),
            Held::Progress(p) => p.is_refused(),
        }
    }
}
impl fmt::Debug for CollectorCausalFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for CollectorCausalFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for CollectorCausalFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCollectorReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    /// ```compile_fail
    /// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::OriginalCollectorReservation;
    /// fn twice(r: OriginalCollectorReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
    ///     let first = r.issue_progress_challenge(); let second = r.issue_progress_challenge();
    /// }
    /// ```
    pub fn issue_progress_challenge(
        self,
    ) -> Result<
        OriginalCollectorChallenge<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        CollectorCausalFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.issue_with(|| {
            let mut random = [0; 32];
            SystemRandom::new()
                .fill(&mut random)
                .map_err(|_| CollectorCausalError::Entropy)?;
            Ok(random)
        })
    }
    #[cfg(test)]
    pub(crate) fn issue_progress_challenge_with(
        self,
        random: Result<[u8; 32], CollectorCausalError>,
    ) -> Result<
        OriginalCollectorChallenge<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        CollectorCausalFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.issue_with(|| random)
    }
    fn issue_with(
        self,
        random: impl FnOnce() -> Result<[u8; 32], CollectorCausalError>,
    ) -> Result<
        OriginalCollectorChallenge<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        CollectorCausalFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut read = OriginalCollectorChallenge {
            reservation: self,
            raw: Vec::new(),
            nonce: String::new(),
            issued: None,
        };
        match read.prepare(random) {
            Ok(()) => Ok(read),
            Err(error) => {
                read.refuse();
                Err(CollectorCausalFailure {
                    error,
                    held: Held::Challenge(Box::new(read), None),
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_collector_causal_codec_tests.rs"]
mod tests;
