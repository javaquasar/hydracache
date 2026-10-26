//! Bounded unprofiled instrumentation. Never an admission/cohort coordinator.
mod cpu;

use crate::{
    embedded::EmbeddedControl,
    native::{
        Dataset, NativeControl, Operation as NativeOperation, Surface as NativeSurface, SEED,
    },
    resp::{Dialect, Operation as RespOperation, RespControl, RespObservation},
    scheduled::{self, Config, Observation},
    security::{MtlsFixture, TransportReceipt},
    target::{Target, TargetOutcome, TargetRequest},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub const PROFILE: &str = "unprofiled-timing-controls-074-v1";

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Surface {
    Embedded,
    Direct,
    Hc1,
    Hc2Mtls,
    Resp2,
    Resp3,
    Resp2Mtls,
    Resp3Mtls,
}
impl Surface {
    fn resp(self) -> Option<(Dialect, bool)> {
        match self {
            Self::Resp2 => Some((Dialect::Resp2, false)),
            Self::Resp3 => Some((Dialect::Resp3, false)),
            Self::Resp2Mtls => Some((Dialect::Resp2, true)),
            Self::Resp3Mtls => Some((Dialect::Resp3, true)),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Get,
    Put,
}
impl Operation {
    fn native(self) -> NativeOperation {
        match self {
            Self::Get => NativeOperation::Get,
            Self::Put => NativeOperation::Put,
        }
    }
    fn resp(self) -> RespOperation {
        match self {
            Self::Get => RespOperation::Get,
            Self::Put => RespOperation::Set,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub schema_version: u32,
    pub profile_id: String,
    pub surface: Surface,
    pub operation: Operation,
    pub seed: u64,
    pub keyspace: usize,
    pub payload_bytes: usize,
    pub dataset_sha256: String,
    pub slots: usize,
    pub pipeline_depth: usize,
    pub warmup_calls: u64,
    pub minimum_usable_cpu_ns: u64,
    pub minimum_usable_measurement_wall_ns: u64,
    pub schedule: Config,
}
impl Input {
    pub fn validate(&self) -> Result<(), String> {
        self.schedule.validate()?;
        if self.schedule.highest_trackable_ns < 2 {
            return Err("timing histogram requires at least two nanosecond units".to_owned());
        }
        if self.schema_version != 1
            || self.profile_id != PROFILE
            || self.seed != SEED
            || ![1, 8, 32, 128].contains(&self.slots)
            || self.slots != self.schedule.concurrency
            || self.warmup_calls > 64
            || self.minimum_usable_cpu_ns < 1_000_000_000
            || self.minimum_usable_measurement_wall_ns < 1_000_000_000
        {
            return Err("timing configuration identity/bounds drift".to_owned());
        }
        if (self.surface.resp().is_some() && ![1, 10, 50].contains(&self.pipeline_depth))
            || (self.surface.resp().is_none() && self.pipeline_depth != 0)
        {
            return Err("invalid pipeline depth for surface".to_owned());
        }
        if Dataset::new(self.keyspace, self.payload_bytes)?.digest() != self.dataset_sha256 {
            return Err("timing corpus digest mismatch".to_owned());
        }
        Ok(())
    }
    /// Canonical field ordering, including every schedule/warmup/security shape.
    /// Binary/features/source are separate identities, never folded into workload.
    pub fn workload_sha256(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).expect("finite configuration"))
        )
    }
}

#[derive(Serialize)]
#[serde(tag = "boundary", content = "observed", rename_all = "snake_case")]
pub enum TimedObservation {
    Native(Box<Observation>),
    Resp(Box<RespObservation>),
}
impl TimedObservation {
    pub fn operations(&self) -> &Observation {
        match self {
            Self::Native(o) => o,
            Self::Resp(o) => &o.operations,
        }
    }
}
#[derive(Serialize)]
pub struct CpuMeasurement {
    pub clock: cpu::Clock,
    pub process_cpu_ns: u64,
    pub wall_elapsed_ns: u64,
    pub cpu_ns_per_offer: f64,
    pub cpu_ns_per_success: Option<f64>,
    pub usable_for_ratio: bool,
    pub unusable_reasons: Vec<&'static str>,
    pub scope: &'static str,
}
#[derive(Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub profile_id: &'static str,
    pub input: Input,
    pub workload_sha256: String,
    pub get_owner_feature: bool,
    pub runtime: &'static str,
    pub allocator: &'static str,
    pub admission_allowed: bool,
    pub product_numeric_claims_allowed: bool,
    pub cross_surface_numeric_comparison_allowed: bool,
    pub secure_fresh_process_material_parity_proven: bool,
    pub warmup_completed: u64,
    pub transport_security: Option<TransportReceipt>,
    pub cpu: Option<CpuMeasurement>,
    pub observation: Option<TimedObservation>,
    pub final_dataset_verified: bool,
    pub shutdown_verified: bool,
    pub error: Option<String>,
}

enum Owner {
    Embedded(Arc<EmbeddedControl>),
    Native(Arc<NativeControl>),
    Resp(Arc<RespControl>),
}
impl Owner {
    async fn start(input: &Input) -> Result<Self, String> {
        let dataset = Dataset::new(input.keyspace, input.payload_bytes)?;
        if let Some((dialect, secure)) = input.surface.resp() {
            let control = if secure {
                RespControl::start_mtls(
                    dataset,
                    input.pipeline_depth,
                    input.slots,
                    input.operation.resp(),
                    dialect,
                    &MtlsFixture::new()?,
                )
                .await?
            } else {
                RespControl::start_connections_dialect(
                    dataset,
                    input.pipeline_depth,
                    input.slots,
                    input.operation.resp(),
                    dialect,
                )
                .await?
            };
            return Ok(Self::Resp(Arc::new(control)));
        }
        if matches!(input.surface, Surface::Embedded) {
            return Ok(Self::Embedded(Arc::new(
                EmbeddedControl::start(input.slots, dataset, input.operation.native()).await?,
            )));
        }
        let control = match input.surface {
            Surface::Direct | Surface::Hc1 => {
                NativeControl::start(
                    if matches!(input.surface, Surface::Direct) {
                        NativeSurface::DirectClientSurface
                    } else {
                        NativeSurface::Hc1Http
                    },
                    input.slots,
                    dataset,
                    input.operation.native(),
                )
                .await?
            }
            Surface::Hc2Mtls => {
                NativeControl::start_hc2_mtls(
                    input.slots,
                    dataset,
                    input.operation.native(),
                    &MtlsFixture::new()?,
                )
                .await?
            }
            _ => unreachable!("handled surfaces"),
        };
        Ok(Self::Native(Arc::new(control)))
    }
    fn security(&self) -> Option<TransportReceipt> {
        match self {
            Self::Resp(c) => c.transport_security().cloned(),
            Self::Native(c) => c.transport_security().cloned(),
            Self::Embedded(_) => None,
        }
    }
    async fn warmup(&self, calls: u64) -> Result<(), String> {
        if let Self::Resp(c) = self {
            return c.warmup(calls).await;
        }
        for sequence in 0..calls {
            let request = TargetRequest { sequence };
            let outcome = match self {
                Self::Embedded(c) => c.execute(request).await,
                Self::Native(c) => c.execute(request).await,
                Self::Resp(_) => unreachable!(),
            };
            if outcome != TargetOutcome::Success {
                return Err("native warmup failed; no retry".to_owned());
            }
        }
        Ok(())
    }
    async fn verify(&self) -> Result<String, String> {
        match self {
            Self::Embedded(c) => c.verify().await,
            Self::Native(c) => c.verify().await,
            Self::Resp(c) => c.verify().await,
        }
    }
    async fn measure(&self, config: &Config) -> Result<TimedObservation, String> {
        match self {
            Self::Embedded(c) => scheduled::run(Arc::clone(c), config)
                .await
                .map(|o| TimedObservation::Native(Box::new(o))),
            Self::Native(c) => scheduled::run(Arc::clone(c), config)
                .await
                .map(|o| TimedObservation::Native(Box::new(o))),
            Self::Resp(c) => c
                .run(config)
                .await
                .map(|o| TimedObservation::Resp(Box::new(o))),
        }
    }
    async fn shutdown(self) -> Result<(), String> {
        match self {
            Self::Embedded(c) => {
                Arc::try_unwrap(c)
                    .map_err(|_| "embedded timing owner leaked")?
                    .shutdown()
                    .await
            }
            Self::Native(c) => {
                Arc::try_unwrap(c)
                    .map_err(|_| "native timing owner leaked")?
                    .shutdown()
                    .await
            }
            Self::Resp(c) => {
                Arc::try_unwrap(c)
                    .map_err(|_| "RESP timing owner leaked")?
                    .shutdown()
                    .await
            }
        }
    }
}

fn cpu_measurement(
    input: &Input,
    observed: &TimedObservation,
    before: u64,
    after: u64,
    wall_elapsed_ns: u64,
    clock: cpu::Clock,
) -> Result<CpuMeasurement, String> {
    let process_cpu_ns = cpu::delta(before, after)?;
    let operations = observed.operations();
    let mut reasons = Vec::new();
    if process_cpu_ns < input.minimum_usable_cpu_ns {
        reasons.push("CPU-below-predeclared-minimum");
    }
    if wall_elapsed_ns < input.minimum_usable_measurement_wall_ns {
        reasons.push("wall-below-predeclared-minimum");
    }
    if operations.successes != operations.offered
        || !operations.owned_tasks_drained
        || operations.scheduled_response_latency.overflow_count != 0
        || operations.service_response_latency.overflow_count != 0
    {
        reasons.push("incomplete-error-or-overflow-observation");
    }
    Ok(CpuMeasurement { clock, process_cpu_ns, wall_elapsed_ns,
        cpu_ns_per_offer: process_cpu_ns as f64 / operations.offered as f64,
        cpu_ns_per_success: (operations.successes != 0).then(|| process_cpu_ns as f64 / operations.successes as f64),
        usable_for_ratio: reasons.is_empty(), unusable_reasons: reasons,
        scope: "whole-process scheduled-driver+byte-oracles+task/wire-drain+observation-projection; not server-only" })
}

async fn measured_fixture(report: &mut Report, owner: &Owner) -> Result<(), String> {
    owner.warmup(report.input.warmup_calls).await?;
    report.warmup_completed = report.input.warmup_calls;
    if owner.verify().await? != report.input.dataset_sha256 {
        return Err("post-warmup corpus drift".to_owned());
    }
    let (before, clock) = cpu::read()?;
    let wall = Instant::now();
    let measured = owner.measure(&report.input.schedule).await;
    let wall_elapsed_ns =
        u64::try_from(wall.elapsed().as_nanos()).map_err(|_| "measurement wall overflow")?;
    // Keep successful projection/sidecar data even if the subsequent CPU query
    // fails. A missing CPU provider never erases an otherwise completed trace.
    report.observation = Some(measured?);
    let (after, after_clock) = cpu::read()?;
    if clock.provider != after_clock.provider
        || clock.unit_resolution_ns != after_clock.unit_resolution_ns
    {
        return Err("process CPU provider changed".to_owned());
    }
    // Retain observed errors and censored/tombstoned samples; never rerun.
    report.cpu = Some(cpu_measurement(
        &report.input,
        report.observation.as_ref().expect("stored observation"),
        before,
        after,
        wall_elapsed_ns,
        clock,
    )?);
    report.final_dataset_verified = owner.verify().await? == report.input.dataset_sha256;
    if !report.final_dataset_verified {
        return Err("final corpus drift".to_owned());
    }
    let operations = report
        .observation
        .as_ref()
        .expect("stored observation")
        .operations();
    if operations.successes != operations.offered || !operations.owned_tasks_drained {
        return Err("timing workload contains non-success outcomes; no retry".to_owned());
    }
    Ok(())
}

fn initial_report(input: Input) -> Report {
    Report {
        schema_version: 1,
        profile_id: PROFILE,
        workload_sha256: input.workload_sha256(),
        input,
        get_owner_feature: cfg!(feature = "get-owner"),
        runtime: "current-thread-required-by-executable",
        allocator: "System-without-counting-wrapper-required-by-executable",
        admission_allowed: false,
        product_numeric_claims_allowed: false,
        cross_surface_numeric_comparison_allowed: false,
        secure_fresh_process_material_parity_proven: false,
        warmup_completed: 0,
        transport_security: None,
        cpu: None,
        observation: None,
        final_dataset_verified: false,
        shutdown_verified: false,
        error: None,
    }
}

async fn finish_fixture(report: &mut Report, owner: Owner) {
    report.transport_security = owner.security();
    let measured = tokio::time::timeout(Duration::from_secs(60), measured_fixture(report, &owner))
        .await
        .unwrap_or_else(|_| Err("timing fixture deadline; not a completed measurement".to_owned()));
    if let Err(error) = measured {
        report.error = Some(error);
    }
    match owner.shutdown().await {
        Ok(()) => report.shutdown_verified = true,
        Err(error) => {
            report.error = Some(format!(
                "{}; shutdown: {error}",
                report.error.as_deref().unwrap_or("measurement completed")
            ))
        }
    }
}

/// One bounded fixture; the caller must supply a current-thread runtime and
/// external process ownership/deadline/source/placement seal for numerical use.
/// Tests invoke this only as semantic instrumentation, never as A/A-A/B data.
pub async fn run(input: Input) -> Report {
    let mut report = initial_report(input);
    if let Err(error) = report.input.validate() {
        report.error = Some(error);
        return report;
    }
    let owner = match Owner::start(&report.input).await {
        Ok(owner) => owner,
        Err(error) => {
            report.error = Some(error);
            return report;
        }
    };
    finish_fixture(&mut report, owner).await;
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> Input {
        Input {
            schema_version: 1,
            profile_id: PROFILE.to_owned(),
            surface: Surface::Direct,
            operation: Operation::Get,
            seed: SEED,
            keyspace: 1,
            payload_bytes: 1,
            dataset_sha256: Dataset::new(1, 1).unwrap().digest(),
            slots: 1,
            pipeline_depth: 0,
            warmup_calls: 1,
            minimum_usable_cpu_ns: 1_000_000_000,
            minimum_usable_measurement_wall_ns: 1_000_000_000,
            schedule: Config {
                operations: 1,
                offered_rate_per_second: 1,
                concurrency: 1,
                maximum_queued: 0,
                operation_timeout_ns: 5_000_000_000,
                drain_timeout_ns: 5_000_000_000,
                slo_ns: 5_000_000_000,
                highest_trackable_ns: 10_000_000_000,
            },
        }
    }
    #[tokio::test]
    async fn warmup_and_verification_failure_still_attempt_explicit_shutdown() {
        for warmup in [0, 1] {
            let mut config = input();
            config.warmup_calls = warmup;
            let mut report = initial_report(config);
            let owner = Owner::start(&report.input).await.unwrap();
            if let Owner::Native(c) = &owner {
                c.delete_dataset().await.unwrap();
            } else {
                panic!("expected native fixture");
            }
            finish_fixture(&mut report, owner).await;
            assert!(report.error.is_some());
            assert!(report.shutdown_verified);
            assert!(!report.final_dataset_verified);
            assert!(report.cpu.is_none());
            assert!(report.observation.is_none());
            assert!(!report.admission_allowed);
        }
    }
    #[tokio::test]
    async fn leaked_owner_is_not_reported_as_successful_shutdown() {
        let mut report = initial_report(input());
        let owner = Owner::start(&report.input).await.unwrap();
        let leaked = match &owner {
            Owner::Native(c) => Arc::clone(c),
            _ => panic!("native fixture"),
        };
        finish_fixture(&mut report, owner).await;
        assert!(report.final_dataset_verified);
        assert!(!report.shutdown_verified);
        assert!(report.error.unwrap().contains("timing owner leaked"));
        Arc::try_unwrap(leaked)
            .unwrap_or_else(|_| panic!("remaining owner"))
            .shutdown()
            .await
            .unwrap();
    }
    #[test]
    fn cpu_ratio_quality_never_promotes_zero_short_or_failed_samples() {
        let config = Config {
            operations: 1,
            offered_rate_per_second: 1,
            concurrency: 1,
            maximum_queued: 0,
            operation_timeout_ns: 1,
            drain_timeout_ns: 1,
            slo_ns: 1,
            highest_trackable_ns: 2,
        };
        let input = Input {
            schema_version: 1,
            profile_id: PROFILE.to_owned(),
            surface: Surface::Direct,
            operation: Operation::Get,
            seed: SEED,
            keyspace: 1,
            payload_bytes: 1,
            dataset_sha256: Dataset::new(1, 1).unwrap().digest(),
            slots: 1,
            pipeline_depth: 0,
            warmup_calls: 0,
            minimum_usable_cpu_ns: 1_000_000_000,
            minimum_usable_measurement_wall_ns: 1_000_000_000,
            schedule: config.clone(),
        };
        for outcome in [scheduled::Outcome::Success, scheduled::Outcome::Error] {
            let sample = scheduled::Sample {
                sequence: 0,
                scheduled_ns: 0,
                started_ns: Some(0),
                terminal_ns: 1,
                outcome,
                scheduled_latency_ns: Some(1),
                service_latency_ns: Some(1),
                incomplete_lower_bound_ns: None,
            };
            let observed = TimedObservation::Native(Box::new(
                scheduled::project(&config, vec![sample], 1, 1, true).unwrap(),
            ));
            let clock = cpu::Clock {
                provider: "test",
                scope: "test",
                unit_resolution_ns: 1,
                resolution_is_accuracy_claim: false,
            };
            let zero = cpu_measurement(&input, &observed, 0, 0, 1, clock.clone()).unwrap();
            assert!(!zero.usable_for_ratio);
            assert_eq!(zero.cpu_ns_per_offer, 0.0);
            let long =
                cpu_measurement(&input, &observed, 0, 1_000_000_000, 1_000_000_000, clock).unwrap();
            assert_eq!(
                long.usable_for_ratio,
                matches!(outcome, scheduled::Outcome::Success)
            );
        }
    }
}
