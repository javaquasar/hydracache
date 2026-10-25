//! Requested-layout counting lane. No CPU, latency, goodput or retention admission.
use super::{Input, Owner, TimedObservation};
use crate::security::{MtlsFixture, TransportReceipt};
use serde::Serialize;

pub const PROFILE: &str = "get-owner-full-d3-allocation-074-v1";

/// Implemented only by the separate executable's counting allocator scope.
/// Dropping an unfinished scope must stop counting (including cancellation).
pub trait Scope {
    type Output: Serialize;
    fn finish(self) -> Self::Output;
}

#[derive(Serialize)]
pub struct Report<R: Serialize> {
    pub schema_version: u32,
    pub profile_id: &'static str,
    pub input: Input,
    pub workload_sha256: String,
    pub get_owner_feature: bool,
    pub counting_allocator_required_by_executable: bool,
    pub timing_claims_allowed: bool,
    pub allocator_retention_proven: bool,
    pub admission_allowed: bool,
    pub warmup_completed: u64,
    pub transport_security: Option<TransportReceipt>,
    pub requested_layouts: Option<R>,
    pub observation: Option<TimedObservation>,
    pub final_dataset_verified: bool,
    pub shutdown_verified: bool,
    pub error: Option<String>,
}

async fn measured<S: Scope, F: FnOnce() -> S>(
    report: &mut Report<S::Output>,
    owner: &Owner,
    begin: F,
) -> Result<(), String> {
    owner.warmup(report.input.warmup_calls).await?;
    report.warmup_completed = report.input.warmup_calls;
    if owner.verify().await? != report.input.dataset_sha256 {
        return Err("post-warmup allocation corpus drift".to_owned());
    }
    // Identical production adapters and original non-skipping schedule. Only
    // this interval is counted; preparation, verification and JSON stay outside.
    let scope = begin();
    let observed = owner.measure(&report.input.schedule).await;
    report.requested_layouts = Some(scope.finish());
    report.observation = Some(observed?);
    report.final_dataset_verified = owner.verify().await? == report.input.dataset_sha256;
    if !report.final_dataset_verified {
        return Err("final allocation corpus drift".to_owned());
    }
    let observed = report
        .observation
        .as_ref()
        .expect("stored observation")
        .operations();
    if observed.successes != observed.offered
        || !observed.owned_tasks_drained
        || observed.scheduled_response_latency.overflow_count != 0
        || observed.service_response_latency.overflow_count != 0
    {
        return Err(
            "allocation observation contains errors, incomplete owners or overflow".to_owned(),
        );
    }
    Ok(())
}

/// Metadata scope and historical v1 bounds are preserved. The executable and
/// external coordinator must enforce the original 60-second process deadline,
/// source/binary/material seals, owned cgroup and bounded durable streams.
pub async fn run<S: Scope, F: FnOnce() -> S>(
    input: Input,
    material: Option<&MtlsFixture>,
    begin: F,
) -> Report<S::Output> {
    let mut report = Report {
        schema_version: 1,
        profile_id: PROFILE,
        workload_sha256: input.workload_sha256(),
        input,
        get_owner_feature: cfg!(feature = "get-owner"),
        counting_allocator_required_by_executable: true,
        timing_claims_allowed: false,
        allocator_retention_proven: false,
        admission_allowed: false,
        warmup_completed: 0,
        transport_security: None,
        requested_layouts: None,
        observation: None,
        final_dataset_verified: false,
        shutdown_verified: false,
        error: None,
    };
    if let Err(error) = report.input.validate() {
        report.error = Some(error);
        return report;
    }
    // Unlike the legacy ephemeral mode, this lane never invents fresh PKI.
    if report.input.surface.uses_mtls() != material.is_some() {
        report.error = Some("allocation material/surface mismatch before fixture".to_owned());
        return report;
    }
    let owner = match Owner::start(&report.input, material).await {
        Ok(owner) => owner,
        Err(error) => {
            report.error = Some(error);
            return report;
        }
    };
    report.transport_security = owner.security();
    if let Err(error) = measured(&mut report, &owner, begin).await {
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
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    struct Marker<'a>(&'a Cell<u32>);
    impl Scope for Marker<'_> {
        type Output = u32;
        fn finish(self) -> u32 {
            self.0.set(self.0.get() + 1);
            self.0.get()
        }
    }
    fn input() -> Input {
        super::super::tests::input()
    }

    #[tokio::test]
    async fn counting_window_runs_once_without_timing_or_retention_admission() {
        let calls = Cell::new(0);
        let report = run(input(), None, || {
            calls.set(calls.get() + 1);
            Marker(&calls)
        })
        .await;
        assert!(report.error.is_none());
        assert!(report.final_dataset_verified && report.shutdown_verified);
        assert_eq!(report.warmup_completed, 1);
        assert_eq!(report.requested_layouts, Some(2));
        assert_eq!(calls.get(), 2);
        assert!(
            !report.timing_claims_allowed
                && !report.allocator_retention_proven
                && !report.admission_allowed
        );
        assert_eq!(report.observation.unwrap().operations().successes, 1);
    }

    #[tokio::test]
    async fn invalid_input_and_missing_secure_material_never_start_counting() {
        for secure in [false, true] {
            let mut config = input();
            if secure {
                config.surface = super::super::Surface::Hc2Mtls;
            } else {
                config.seed = 0;
            }
            let report = run(config, None, || -> Marker<'static> {
                panic!("counting started before validation")
            })
            .await;
            assert!(report.error.is_some());
            assert!(report.requested_layouts.is_none() && report.observation.is_none());
            assert!(!report.shutdown_verified);
        }
    }

    #[tokio::test]
    async fn cancelling_a_started_window_drops_scope_without_claiming_completion() {
        struct CancelScope<'a> {
            dropped: &'a Cell<bool>,
            finished: &'a Cell<bool>,
        }
        impl Scope for CancelScope<'_> {
            type Output = ();
            fn finish(self) {
                self.finished.set(true);
            }
        }
        impl Drop for CancelScope<'_> {
            fn drop(&mut self) {
                self.dropped.set(true);
            }
        }
        let dropped = Cell::new(false);
        let finished = Cell::new(false);
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut config = input();
        config.schedule.operations = 2;
        config.schedule.offered_rate_per_second = 1;
        let mut pending = Box::pin(run(config, None, || {
            tx.send(()).unwrap();
            CancelScope {
                dropped: &dropped,
                finished: &finished,
            }
        }));
        // Synchronize on scope entry, never a guessed setup sleep or changed
        // production deadline. No allocation/CPU measurement is taken here.
        tokio::select! {
            _ = &mut pending => panic!("fixture completed before cancellation"),
            ready = rx => ready.unwrap(),
        }
        drop(pending);
        assert!(dropped.get());
        assert!(!finished.get());
    }
}
