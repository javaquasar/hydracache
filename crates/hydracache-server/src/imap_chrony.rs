//! Validate one chrony tracking sample before making it eligible for IMap TTL.

use std::net::IpAddr;

use hydracache::TimeObservation;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(target_os = "linux")]
pub(crate) mod linux;

const NS_PER_MS: u64 = 1_000_000;
const NS_PER_SEC: u64 = 1_000_000_000;
// Account for rounding of the three tracking fields printed with nine decimal places.
const TRACKING_ROUNDING_NS: u64 = 2;

/// A trusted collector's paired realtime and boottime read. Its generation must
/// change on a clock step, suspend, watcher loss or restart.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClockReading {
    pub realtime_ns: u64,
    pub boottime_ns: u64,
    pub read_window_ns: u64,
    pub generation: u64,
}

/// Bounds selected by the managed host profile, including its maximum slew rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChronyPolicy {
    pub maximum_uncertainty_ms: u64,
    pub maximum_snapshot_age_ms: u64,
    pub maximum_reference_age_ms: u64,
    pub maximum_acquisition_ms: u64,
    pub maximum_drift_ppm: u32,
}

/// Reasons for refusing a tracking sample or a later observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ChronyEvidenceError {
    #[error("invalid chrony tracking report")]
    Malformed,
    #[error("chrony is unsynchronized or reports a leap transition")]
    Unsynchronized,
    #[error("chrony selected a reference outside the admitted network sources")]
    UntrustedReference,
    #[error("clock generation, monotonicity or read window changed")]
    ClockChanged,
    #[error("chrony snapshot is too old")]
    StaleSnapshot,
    #[error("chrony reference timestamp is stale or in the future")]
    StaleReference,
    #[error("chrony uncertainty exceeds the admitted bound or overflows")]
    Uncertainty,
}

/// Immutable evidence captured within a single clock generation.
#[derive(Debug)]
pub struct ChronySample {
    acquired: ClockReading,
    reference_ns: u64,
    error_ns: u64,
    policy: ChronyPolicy,
}

impl ChronySample {
    /// Parse `chronyc -n -c tracking` collected between the two trusted clock reads.
    /// The caller admits the source identities and supplies a live clock watcher.
    pub fn capture(
        tracking: &str,
        allowed: &[IpAddr],
        [before, after]: [ClockReading; 2],
        policy: ChronyPolicy,
    ) -> Result<Self, ChronyEvidenceError> {
        use ChronyEvidenceError as Error;

        let acquisition = policy.elapsed(before, after)?;
        let window = acquisition
            .checked_add(before.read_window_ns)
            .and_then(|value| value.checked_add(after.read_window_ns))
            .ok_or(Error::Uncertainty)?;
        if window > to_ns(policy.maximum_acquisition_ms)? {
            return Err(Error::ClockChanged);
        }
        let tracking = Tracking::parse(tracking, allowed)?;
        let acquisition_drift = policy.drift(acquisition)?;
        let error_ns = tracking
            .offset_ns
            .checked_add(tracking.dispersion_ns)
            .and_then(|value| value.checked_add(tracking.delay_ns.div_ceil(2)))
            .and_then(|value| value.checked_add(window))
            .and_then(|value| value.checked_add(acquisition_drift))
            .and_then(|value| value.checked_add(TRACKING_ROUNDING_NS))
            .ok_or(Error::Uncertainty)?;
        let sample = Self {
            acquired: after,
            reference_ns: tracking.reference_ns,
            error_ns,
            policy,
        };
        sample.observe(after)?;
        Ok(sample)
    }

    /// Age both the snapshot and its upstream reference without renewing either.
    pub fn observe(&self, now: ClockReading) -> Result<TimeObservation, ChronyEvidenceError> {
        use ChronyEvidenceError as Error;

        let age = self.policy.elapsed(self.acquired, now)?;
        if age > to_ns(self.policy.maximum_snapshot_age_ms)? {
            return Err(Error::StaleSnapshot);
        }
        let error = self
            .error_ns
            .checked_add(self.policy.drift(age)?)
            .and_then(|value| value.checked_add(now.read_window_ns))
            .ok_or(Error::Uncertainty)?;
        let reference_age = now
            .realtime_ns
            .checked_add(error)
            .ok_or(Error::Uncertainty)?
            .checked_sub(self.reference_ns)
            .ok_or(Error::StaleReference)?;
        if reference_age > to_ns(self.policy.maximum_reference_age_ms)? {
            return Err(Error::StaleReference);
        }
        // The millisecond midpoint truncates realtime. Widen the uncertainty to
        // retain a conservative upper bound after that conversion.
        let uncertainty_ms = error
            .checked_add(now.realtime_ns % NS_PER_MS)
            .ok_or(Error::Uncertainty)?
            .div_ceil(NS_PER_MS);
        let observation = TimeObservation {
            now_ms: now.realtime_ns / NS_PER_MS,
            uncertainty_ms,
        };
        observation
            .bounds(self.policy.maximum_uncertainty_ms)
            .ok_or(Error::Uncertainty)?;
        Ok(observation)
    }
}

impl ChronyPolicy {
    fn drift(self, age_ns: u64) -> Result<u64, ChronyEvidenceError> {
        let ns = (u128::from(age_ns) * u128::from(self.maximum_drift_ppm)).div_ceil(1_000_000);
        u64::try_from(ns).map_err(|_| ChronyEvidenceError::Uncertainty)
    }

    fn elapsed(self, before: ClockReading, now: ClockReading) -> Result<u64, ChronyEvidenceError> {
        use ChronyEvidenceError as Error;

        let limit = to_ns(self.maximum_acquisition_ms)?;
        if before.generation == 0
            || before.generation != now.generation
            || before.read_window_ns > limit
            || now.read_window_ns > limit
        {
            return Err(Error::ClockChanged);
        }
        let age = now
            .boottime_ns
            .checked_sub(before.boottime_ns)
            .ok_or(Error::ClockChanged)?;
        let wall_elapsed = now
            .realtime_ns
            .checked_sub(before.realtime_ns)
            .ok_or(Error::ClockChanged)?;
        let tolerance = self
            .drift(age)?
            .checked_add(before.read_window_ns)
            .and_then(|value| value.checked_add(now.read_window_ns))
            .ok_or(Error::Uncertainty)?;
        if wall_elapsed.abs_diff(age) > tolerance {
            return Err(Error::ClockChanged);
        }
        Ok(age)
    }
}

fn to_ns(ms: u64) -> Result<u64, ChronyEvidenceError> {
    ms.checked_mul(NS_PER_MS)
        .ok_or(ChronyEvidenceError::Uncertainty)
}

struct Tracking {
    reference_ns: u64,
    offset_ns: u64,
    delay_ns: u64,
    dispersion_ns: u64,
}

impl Tracking {
    fn parse(line: &str, allowed: &[IpAddr]) -> Result<Self, ChronyEvidenceError> {
        use ChronyEvidenceError as Error;

        let row = line.strip_suffix('\n').unwrap_or(line);
        if row.len() > 1024 || !row.is_ascii() || row.contains(['\r', '\n']) {
            return Err(Error::Malformed);
        }
        let fields: Vec<_> = row.split(',').collect();
        let [id, address, stratum, reference, offset, last, rms, frequency, residual, skew, delay, dispersion, interval, leap] =
            fields.as_slice()
        else {
            return Err(Error::Malformed);
        };
        let stratum: u8 = stratum.parse().map_err(|_| Error::Malformed)?;
        if !(1..=15).contains(&stratum) || *leap != "Normal" {
            return Err(Error::Unsynchronized);
        }
        if id.len() != 8 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Error::Malformed);
        }
        let id = u32::from_str_radix(id, 16).map_err(|_| Error::Malformed)?;
        let address = address
            .parse::<IpAddr>()
            .map_err(|_| Error::UntrustedReference)?;
        if matches!(id, 0 | 0x7f7f_0101)
            || address.is_loopback()
            || address.is_unspecified()
            || address.is_multicast()
            || !allowed.contains(&address)
        {
            return Err(Error::UntrustedReference);
        }
        // Validate every numeric field, including those not used in the bound.
        for unused in [last, frequency, residual] {
            absolute_ns(unused)?;
        }
        for unused in [rms, skew, interval] {
            seconds_ns(unused)?;
        }
        let reference_ns = seconds_ns(reference)?;
        if reference_ns == 0 {
            return Err(Error::Unsynchronized);
        }
        Ok(Self {
            reference_ns,
            offset_ns: absolute_ns(offset)?,
            delay_ns: seconds_ns(delay)?,
            dispersion_ns: seconds_ns(dispersion)?,
        })
    }
}

fn absolute_ns(value: &str) -> Result<u64, ChronyEvidenceError> {
    seconds_ns(value.strip_prefix(['+', '-']).unwrap_or(value))
}

fn seconds_ns(value: &str) -> Result<u64, ChronyEvidenceError> {
    use ChronyEvidenceError as Error;

    let (whole, fractional) = value.split_once('.').map_or((value, ""), |parts| parts);
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fractional.len() > 9
        || !fractional.bytes().all(|byte| byte.is_ascii_digit())
        || (fractional.is_empty() && value.ends_with('.'))
    {
        return Err(Error::Malformed);
    }
    let whole = whole.parse::<u64>().map_err(|_| Error::Uncertainty)?;
    let fractional_ns = if fractional.is_empty() {
        0
    } else {
        fractional.parse::<u64>().map_err(|_| Error::Malformed)?
            * 10_u64.pow(9 - fractional.len() as u32)
    };
    whole
        .checked_mul(NS_PER_SEC)
        .and_then(|value| value.checked_add(fractional_ns))
        .ok_or(Error::Uncertainty)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACKING: &str = "C0000201,192.0.2.1,2,1000.000000000,-0.001000000,0.0,0.0005,5.0,-0.001,0.1,0.001000001,0.000500000,1.0,Normal";
    const POLICY: ChronyPolicy = ChronyPolicy {
        maximum_uncertainty_ms: 10,
        maximum_snapshot_age_ms: 3_000,
        maximum_reference_age_ms: 2_000,
        maximum_acquisition_ms: 20,
        maximum_drift_ppm: 500,
    };

    fn clock(ahead_ns: u64) -> ClockReading {
        ClockReading {
            realtime_ns: 1001 * NS_PER_SEC + ahead_ns,
            boottime_ns: 5 * NS_PER_SEC + ahead_ns,
            read_window_ns: 0,
            generation: 7,
        }
    }

    fn capture(
        tracking: &str,
        readings: [ClockReading; 2],
        policy: ChronyPolicy,
    ) -> Result<ChronySample, ChronyEvidenceError> {
        ChronySample::capture(tracking, &["192.0.2.1".parse().unwrap()], readings, policy)
    }

    #[test]
    fn rounds_the_tracking_error_and_read_window_outward() {
        let acquired = ClockReading {
            read_window_ns: 120_000,
            ..clock(1_700_000)
        };
        let sample = capture(TRACKING, [clock(0), acquired], POLICY).unwrap();
        assert_eq!(
            sample.observe(acquired).unwrap(),
            TimeObservation {
                now_ms: 1_001_001,
                uncertainty_ms: 5
            }
        );
        let limits = sample
            .observe(clock(2_700_000))
            .unwrap()
            .bounds(10)
            .unwrap();
        assert!(limits.0 <= 1_001_002 && limits.1 >= 1_001_006);
    }

    #[test]
    fn rejects_untrusted_unsynchronized_and_malformed_tracking() {
        for (old, new, reason) in [
            (
                "192.0.2.1",
                "192.0.2.2",
                ChronyEvidenceError::UntrustedReference,
            ),
            (
                "C0000201",
                "7F7F0101",
                ChronyEvidenceError::UntrustedReference,
            ),
            (
                "Normal",
                "Not synchronised",
                ChronyEvidenceError::Unsynchronized,
            ),
            (
                "Normal",
                "Insert second",
                ChronyEvidenceError::Unsynchronized,
            ),
            (",2,1000", ",16,1000", ChronyEvidenceError::Unsynchronized),
            ("0.001000001", "NaN", ChronyEvidenceError::Malformed),
            ("0.001000001", "-0.004", ChronyEvidenceError::Malformed),
            ("-0.001,0.1", "--0.001,0.1", ChronyEvidenceError::Malformed),
            (
                "0.000500000",
                "0.0005000001",
                ChronyEvidenceError::Malformed,
            ),
            (
                "0.000500000",
                "18446744073709551615",
                ChronyEvidenceError::Uncertainty,
            ),
        ] {
            assert_eq!(
                capture(&TRACKING.replace(old, new), [clock(0); 2], POLICY).unwrap_err(),
                reason
            );
        }
        assert_eq!(
            capture(&format!("{TRACKING}\n{TRACKING}"), [clock(0); 2], POLICY).unwrap_err(),
            ChronyEvidenceError::Malformed
        );
        assert!(capture(&format!("{TRACKING}\n"), [clock(0); 2], POLICY).is_ok());
    }

    #[test]
    fn ages_snapshot_and_reference_independently_even_after_recollection() {
        let sample = capture(TRACKING, [clock(0); 2], POLICY).unwrap();
        let later = clock(1_100 * NS_PER_MS);
        assert_eq!(
            sample.observe(later),
            Err(ChronyEvidenceError::StaleReference)
        );
        assert_eq!(
            capture(TRACKING, [later; 2], POLICY).unwrap_err(),
            ChronyEvidenceError::StaleReference
        );
        assert_eq!(
            sample.observe(clock(3_001 * NS_PER_MS)),
            Err(ChronyEvidenceError::StaleSnapshot)
        );
        let future = TRACKING.replace("1000.000000000", "1002.000000000");
        assert_eq!(
            capture(&future, [clock(0); 2], POLICY).unwrap_err(),
            ChronyEvidenceError::StaleReference
        );
    }

    #[test]
    fn fences_steps_suspend_and_read_timeout_and_allows_bounded_slew() {
        let sample = capture(TRACKING, [clock(0); 2], POLICY).unwrap();
        let now = clock(100 * NS_PER_MS);
        for changed in [
            ClockReading {
                generation: 8,
                ..now
            },
            ClockReading {
                realtime_ns: now.realtime_ns + 5 * NS_PER_MS,
                ..now
            },
            ClockReading {
                realtime_ns: now.realtime_ns - 5 * NS_PER_MS,
                ..now
            },
            ClockReading {
                boottime_ns: now.boottime_ns + 50 * NS_PER_MS,
                ..now
            },
            ClockReading {
                read_window_ns: 21 * NS_PER_MS,
                ..now
            },
        ] {
            assert_eq!(
                sample.observe(changed),
                Err(ChronyEvidenceError::ClockChanged)
            );
        }
        assert_eq!(
            capture(TRACKING, [clock(0), clock(21 * NS_PER_MS)], POLICY).unwrap_err(),
            ChronyEvidenceError::ClockChanged
        );
        for realtime_ns in [now.realtime_ns - 49_999, now.realtime_ns + 49_999] {
            assert!(sample.observe(ClockReading { realtime_ns, ..now }).is_ok());
        }
    }

    #[test]
    fn rejects_observation_when_drift_or_arithmetic_overflow_breaks_the_bound() {
        let policy = ChronyPolicy {
            maximum_uncertainty_ms: 3,
            ..POLICY
        };
        let sample = capture(TRACKING, [clock(0); 2], policy).unwrap();
        assert_eq!(
            sample
                .observe(clock(900 * NS_PER_MS))
                .unwrap()
                .uncertainty_ms,
            3
        );
        let tighter = ChronyPolicy {
            maximum_uncertainty_ms: 2,
            ..POLICY
        };
        assert_eq!(
            capture(TRACKING, [clock(0); 2], tighter).unwrap_err(),
            ChronyEvidenceError::Uncertainty
        );
        let overflow = ClockReading {
            realtime_ns: u64::MAX,
            ..clock(0)
        };
        assert_eq!(
            capture(TRACKING, [overflow; 2], POLICY).unwrap_err(),
            ChronyEvidenceError::Uncertainty
        );
        let profile = ChronyPolicy {
            maximum_reference_age_ms: 50_000,
            maximum_drift_ppm: 20_000,
            ..POLICY
        };
        let sample = capture(TRACKING, [clock(0); 2], profile).unwrap();
        assert_eq!(
            sample.observe(clock(500 * NS_PER_MS)),
            Err(ChronyEvidenceError::Uncertainty)
        );
    }
}
