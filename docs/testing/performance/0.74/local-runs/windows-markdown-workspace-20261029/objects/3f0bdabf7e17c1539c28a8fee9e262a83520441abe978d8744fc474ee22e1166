//! Allocation-only, one fixture per fresh process. Never a timing executable.
#[path = "../../../resp-scratch-screen-074/src/memory.rs"]
mod memory;

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: memory::Allocator = memory::Allocator;

use get_owner_scheduled_controls_074::{
    native::{Dataset, NativeControl, Operation as NativeOperation, Surface, SEED},
    resp::{Dialect, Operation, RespControl},
    security::{MtlsFixture, TransportReceipt},
};
use serde::Serialize;
use std::time::Duration;

const KEYSPACE: usize = 16;
const ROUNDS: u64 = 128;

fn arguments(args: &[String]) -> Result<(&str, usize), String> {
    if args.len() != 2 {
        return Err("expected exactly SURFACE PAYLOAD_BYTES".to_owned());
    }
    if !["direct", "hc1", "hc2-mtls", "resp2-mtls", "resp3-mtls"].contains(&args[0].as_str()) {
        return Err("unsupported diagnostic surface".to_owned());
    }
    let payload = args[1].parse().map_err(|_| "invalid payload")?;
    if ![256, 65536].contains(&payload) {
        return Err("payload is outside preregistered diagnostic cells".to_owned());
    }
    Ok((&args[0], payload))
}

enum Owner {
    Native(Box<NativeControl>),
    Resp(Box<RespControl>),
}
impl Owner {
    async fn start(surface: &str, payload: usize) -> Result<Self, String> {
        let dataset = Dataset::new(KEYSPACE, payload)?;
        match surface {
            "direct" | "hc1" => Ok(Self::Native(Box::new(
                NativeControl::start(
                    if surface == "direct" {
                        Surface::DirectClientSurface
                    } else {
                        Surface::Hc1Http
                    },
                    8,
                    dataset,
                    NativeOperation::Get,
                )
                .await?,
            ))),
            "hc2-mtls" => Ok(Self::Native(Box::new(
                NativeControl::start_hc2_mtls(
                    8,
                    dataset,
                    NativeOperation::Get,
                    &MtlsFixture::new()?,
                )
                .await?,
            ))),
            "resp2-mtls" | "resp3-mtls" => Ok(Self::Resp(Box::new(
                RespControl::start_mtls(
                    dataset,
                    10,
                    8,
                    Operation::Get,
                    if surface == "resp2-mtls" {
                        Dialect::Resp2
                    } else {
                        Dialect::Resp3
                    },
                    &MtlsFixture::new()?,
                )
                .await?,
            ))),
            _ => Err("unsupported diagnostic surface".to_owned()),
        }
    }
    fn logical(&self) -> (usize, usize) {
        match self {
            Self::Native(c) => (c.retained_entries(), c.retained_value_bytes()),
            Self::Resp(c) => (c.retained_entries(), c.retained_value_bytes()),
        }
    }
    fn receipt(&self) -> Option<TransportReceipt> {
        match self {
            Self::Native(c) => c.transport_security().cloned(),
            Self::Resp(c) => c.transport_security().cloned(),
        }
    }
    async fn round(&self, write: bool, sequence: u64) -> Result<(), String> {
        match self {
            Self::Native(c) => c.diagnostic_round(write, sequence).await,
            Self::Resp(c) => c.diagnostic_round(write, sequence).await,
        }
    }
    async fn delete(&self) -> Result<(), String> {
        match self {
            Self::Native(c) => c.delete_dataset().await,
            Self::Resp(c) => c.delete_dataset().await,
        }
    }
    async fn refill(&self) -> Result<(), String> {
        match self {
            Self::Native(c) => c.refill_dataset().await,
            Self::Resp(c) => c.refill_dataset().await,
        }
    }
    async fn shutdown(self) -> Result<(), String> {
        match self {
            Self::Native(c) => c.shutdown().await,
            Self::Resp(c) => c.shutdown().await,
        }
    }
}

#[derive(Serialize)]
struct Phase {
    name: &'static str,
    workload_calls: u64,
    // Setup/delete/refill include additional read oracles; this denominator
    // never claims server-only allocation/op or matched cross-surface cost.
    allocations: memory::Measurement,
    process_rss_bytes: u64,
    logical_entries: usize,
    logical_value_bytes: usize,
}
#[derive(Serialize)]
struct Report {
    schema_version: u32,
    profile_id: &'static str,
    surface: String,
    payload_bytes: usize,
    seed: u64,
    keyspace: usize,
    connections: usize,
    pipeline_depth: usize,
    dataset_sha256: String,
    get_owner_feature: bool,
    runtime: &'static str,
    allocator: &'static str,
    rss_scope: &'static str,
    allocator_active_resident_retained: Option<u64>,
    allocator_retention_status: &'static str,
    product_numeric_claims_allowed: bool,
    cross_surface_numeric_comparison_allowed: bool,
    admission_allowed: bool,
    transport_security: Option<TransportReceipt>,
    phases: Vec<Phase>,
    error: Option<String>,
}

#[cfg(windows)]
const RSS_SCOPE: &str = "whole-process-Windows-working-set-not-heap-resident";
#[cfg(not(windows))]
const RSS_SCOPE: &str = "whole-process-Linux-VmRSS-not-heap-resident";

#[cfg(windows)]
fn process_rss() -> Result<u64, String> {
    #[repr(C)]
    struct Counters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn K32GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut Counters,
            size: u32,
        ) -> i32;
    }
    let mut counters = Counters {
        cb: std::mem::size_of::<Counters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };
    // SAFETY: repr(C) PROCESS_MEMORY_COUNTERS has initialized fields and the
    // exact API size. The pseudo-handle belongs to this current process only.
    if unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) } == 0 {
        return Err(format!(
            "working-set query failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(counters.working_set_size as u64)
}
#[cfg(not(windows))]
fn process_rss() -> Result<u64, String> {
    let status = std::fs::read_to_string("/proc/self/status").map_err(|e| e.to_string())?;
    let kb: u64 = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .ok_or("VmRSS unavailable")?
        .trim()
        .strip_suffix(" kB")
        .ok_or("unexpected VmRSS units")?
        .trim()
        .parse()
        .map_err(|_| "invalid VmRSS")?;
    kb.checked_mul(1024).ok_or("VmRSS overflow".to_owned())
}

async fn run(report: &mut Report) -> Result<(), String> {
    let scope = memory::Scope::start();
    let mut owner = Some(Owner::start(&report.surface, report.payload_bytes).await?);
    let allocations = scope.finish();
    let c = owner.as_ref().ok_or("missing owner")?;
    let logical = c.logical();
    if logical != (KEYSPACE, KEYSPACE * report.payload_bytes) {
        return Err("preload logical drift".to_owned());
    }
    report.transport_security = c.receipt();
    report.phases.push(Phase {
        name: "preload",
        workload_calls: KEYSPACE as u64,
        allocations,
        process_rss_bytes: process_rss()?,
        logical_entries: logical.0,
        logical_value_bytes: logical.1,
    });
    for name in ["get", "set", "idle", "delete", "refill", "shutdown"] {
        let scope = memory::Scope::start();
        let workload_calls = match name {
            "get" | "set" => {
                for sequence in 0..ROUNDS {
                    owner
                        .as_ref()
                        .ok_or("missing owner")?
                        .round(name == "set", sequence)
                        .await?;
                }
                ROUNDS
            }
            "idle" => {
                tokio::time::sleep(Duration::from_millis(250)).await;
                0
            }
            "delete" => {
                owner.as_ref().ok_or("missing owner")?.delete().await?;
                KEYSPACE as u64
            }
            "refill" => {
                owner.as_ref().ok_or("missing owner")?.refill().await?;
                KEYSPACE as u64
            }
            "shutdown" => {
                owner.take().ok_or("missing owner")?.shutdown().await?;
                0
            }
            _ => unreachable!(),
        };
        let allocations = scope.finish();
        let logical = owner.as_ref().map(Owner::logical).unwrap_or((0, 0));
        let expected = if name == "delete" || name == "shutdown" {
            (0, 0)
        } else {
            (KEYSPACE, KEYSPACE * report.payload_bytes)
        };
        if logical != expected {
            return Err(format!("{name} logical drift"));
        }
        report.phases.push(Phase {
            name,
            workload_calls,
            allocations,
            process_rss_bytes: process_rss()?,
            logical_entries: logical.0,
            logical_value_bytes: logical.1,
        });
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let (surface, payload) = match arguments(&args) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let mut report = Report {
        schema_version: 1,
        profile_id: "secure-observer-memory-checks-074-v1",
        surface: surface.to_owned(),
        payload_bytes: payload,
        seed: SEED,
        keyspace: KEYSPACE,
        connections: if surface == "direct" { 0 } else { 8 },
        pipeline_depth: if surface.starts_with("resp") { 10 } else { 0 },
        dataset_sha256: Dataset::new(KEYSPACE, payload)
            .expect("validated dataset")
            .digest(),
        get_owner_feature: cfg!(feature = "get-owner"),
        runtime: "current-thread",
        allocator: "System-with-requested-layout-counters",
        rss_scope: RSS_SCOPE,
        allocator_active_resident_retained: None,
        allocator_retention_status: "unavailable-not-a-pass",
        product_numeric_claims_allowed: false,
        cross_surface_numeric_comparison_allowed: false,
        admission_allowed: false,
        transport_security: None,
        phases: Vec::with_capacity(7),
        error: None,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("diagnostic runtime");
    let result = runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(60), run(&mut report))
            .await
            .map_err(|_| "diagnostic process deadline".to_owned())?
    });
    if let Err(error) = result {
        report.error = Some(error);
    }
    // No JSON allocation is inside a measured epoch.
    println!(
        "{}",
        serde_json::to_string(&report).expect("diagnostic receipt")
    );
    if report.error.is_some() {
        std::process::ExitCode::FAILURE
    } else {
        std::process::ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arguments_refuse_unsealed_shapes_and_unknown_surfaces() {
        for args in [
            vec![],
            vec!["direct"],
            vec!["unknown", "256"],
            vec!["direct", "1048576"],
            vec!["direct", "256", "retry"],
        ] {
            assert!(arguments(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err());
        }
        for surface in ["direct", "hc1", "hc2-mtls", "resp2-mtls", "resp3-mtls"] {
            for payload in ["256", "65536"] {
                assert!(arguments(&[surface.to_owned(), payload.to_owned()]).is_ok());
            }
        }
    }
    #[test]
    fn rss_has_real_units_and_is_not_reported_as_allocator_retention() {
        assert!(process_rss().unwrap() > 0);
        assert!(RSS_SCOPE.contains("whole-process"));
        assert!(RSS_SCOPE.contains("not-heap-resident"));
    }
}
