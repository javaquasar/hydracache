//! D1 ownership attribution only. No product feature, candidate or timing claim.
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::hint::black_box;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use hydracache_client_protocol::{ClientResponse, ClientResponseEnvelope};
use hydracache_client_transport_axum::{ClientIdentity, ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{
    translate_redis_command, RedisCommand, RedisExecutionPlan, RedisTranslatedCommand,
    RedisTranslationContext, RespValue,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

// Reuse the audited tool-only counter, not the old scratch workload/candidate.
#[path = "../../resp-scratch-screen-074/src/memory.rs"]
mod memory;
#[global_allocator]
static ALLOCATOR: memory::Allocator = memory::Allocator;
const PROFILE: &str = "response-reduction-owner-d1-074-v1";
const SEED: u64 = 740074;
const WARMUP: u64 = 100;
const CLOCK: u64 = 1_000_000;

#[derive(Clone, Copy)]
struct Cell {
    id: &'static str,
    set: bool,
    hit: bool,
    bytes: usize,
    iterations: u64,
}
const CELLS: [Cell; 6] = [
    Cell {
        id: "get-empty",
        set: false,
        hit: true,
        bytes: 0,
        iterations: 10000,
    },
    Cell {
        id: "get-64",
        set: false,
        hit: true,
        bytes: 64,
        iterations: 10000,
    },
    Cell {
        id: "get-4096",
        set: false,
        hit: true,
        bytes: 4096,
        iterations: 10000,
    },
    Cell {
        id: "get-1048576",
        set: false,
        hit: true,
        bytes: 1048576,
        iterations: 500,
    },
    Cell {
        id: "get-miss",
        set: false,
        hit: false,
        bytes: 4096,
        iterations: 10000,
    },
    Cell {
        id: "set-4096",
        set: true,
        hit: true,
        bytes: 4096,
        iterations: 10000,
    },
];

fn plan(set: bool, key: &[u8], payload: &[u8]) -> Result<RedisExecutionPlan, Box<dyn Error>> {
    let command = if set {
        RedisCommand::Set {
            key: key.to_vec(),
            value: payload.to_vec(),
            options: Vec::new(),
        }
    } else {
        RedisCommand::Get { key: key.to_vec() }
    };
    let context = RedisTranslationContext::new("default", "response-owner-074")?;
    let RedisTranslatedCommand::Execute(plan) = translate_redis_command(command, &context)? else {
        return Err("expected executable plan".into());
    };
    if plan.initial_requests().len() != 1 {
        return Err("expected single initial request".into());
    }
    Ok(plan)
}

struct Fixture {
    state: ClientSurfaceState,
    identity: ClientIdentity,
}
impl Fixture {
    fn new(cell: Cell, key: &[u8], payload: &[u8]) -> Result<Self, Box<dyn Error>> {
        let state = ClientSurfaceState::new(ClientSurfaceLimits::default())?;
        // Explicit deterministic owner lane; this does not measure production clocks.
        state.set_cache_time_for_tests(Some(CLOCK));
        let fixture = Self {
            state,
            identity: ClientIdentity::new("response-owner-074", "default")?,
        };
        if cell.hit {
            let put = plan(true, key, payload)?;
            let responses = fixture.dispatch(&put)?;
            validate_native(Cell { set: true, ..cell }, &responses, payload)?;
        }
        Ok(fixture)
    }
    fn dispatch(
        &self,
        plan: &RedisExecutionPlan,
    ) -> Result<Vec<ClientResponseEnvelope>, Box<dyn Error>> {
        // Same public plan/dispatch/vector/followup chain as canonical execute_plan.
        let mut responses = plan
            .initial_requests()
            .iter()
            .cloned()
            .map(|request| {
                self.state
                    .dispatch_verified_request(&self.identity, request)
            })
            .collect::<Vec<_>>();
        let followups = plan.followup_requests(&responses)?;
        if !followups.is_empty() {
            return Err("single-key fixture unexpectedly requires followup".into());
        }
        responses.extend(followups.into_iter().map(|request| {
            self.state
                .dispatch_verified_request(&self.identity, request)
        }));
        Ok(responses)
    }
    fn reconcile(
        &self,
        cell: Cell,
        key: &[u8],
        payload: &[u8],
        operations: u64,
    ) -> Result<(), Box<dyn Error>> {
        let preload = u64::from(cell.hit);
        let mutations = if cell.set {
            preload + operations
        } else {
            preload
        };
        if self.state.dispatch_attempts() != preload + operations
            || self.state.state_mutations() != mutations
            || self.state.retained_state_for_diagnostics().store_entries != usize::from(cell.hit)
        {
            return Err("dispatch/mutation/cardinality reconciliation failed".into());
        }
        let get = plan(false, key, payload)?;
        validate_native(Cell { set: false, ..cell }, &self.dispatch(&get)?, payload)
    }
}

fn validate_native(
    cell: Cell,
    responses: &[ClientResponseEnvelope],
    payload: &[u8],
) -> Result<(), Box<dyn Error>> {
    if responses.len() != 1 {
        return Err("response count differs".into());
    }
    let valid = match &responses[0].result {
        Ok(ClientResponse::Stored) => cell.set,
        Ok(ClientResponse::Value { value: Some(value) }) => {
            !cell.set && cell.hit && value == payload
        }
        Ok(ClientResponse::Value { value: None }) => !cell.set && !cell.hit,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err("native result differs".into())
    }
}
fn validate_reduced(
    cell: Cell,
    responses: &[ClientResponseEnvelope],
    value: &RespValue,
    payload: &[u8],
) -> Result<(), Box<dyn Error>> {
    if responses.len() != 1 {
        return Err("response count differs".into());
    }
    let valid = match value {
        RespValue::SimpleString("OK") => cell.set,
        RespValue::Null => !cell.set && !cell.hit,
        RespValue::BulkString(value) => {
            if cell.set || !cell.hit || value != payload {
                return Err("bulk result differs".into());
            }
            let Ok(ClientResponse::Value {
                value: Some(original),
            }) = &responses[0].result
            else {
                return Err("missing original payload owner".into());
            };
            // Nonempty Vec clone has a distinct owner while source remains alive.
            value.is_empty() || value.as_ptr() != original.as_ptr()
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err("reduced result/ownership differs".into())
    }
}

#[derive(Serialize)]
struct Stage {
    memory: memory::Measurement,
    maximum_response_live_increment_bytes: u64,
    maximum_response_and_reduced_live_increment_bytes: u64,
}
fn warm(
    fixture: &Fixture,
    cell: Cell,
    request: &RedisExecutionPlan,
    payload: &[u8],
    reduce: bool,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..WARMUP {
        let responses = fixture.dispatch(request)?;
        validate_native(cell, &responses, payload)?;
        if reduce {
            validate_reduced(cell, &responses, &request.reduce(&responses)?, payload)?;
        }
    }
    Ok(())
}
fn measure_dispatch(
    fixture: &Fixture,
    cell: Cell,
    request: &RedisExecutionPlan,
    payload: &[u8],
    reduce: bool,
) -> Result<Stage, Box<dyn Error>> {
    let scope = memory::Scope::start();
    let before = scope.before;
    let mut response_live = 0;
    let mut reduced_live = 0;
    for _ in 0..cell.iterations {
        let responses = fixture.dispatch(request)?;
        validate_native(cell, &responses, payload)?;
        response_live = response_live.max(memory::live().saturating_sub(before));
        if reduce {
            let value = request.reduce(&responses)?;
            validate_reduced(cell, &responses, &value, payload)?;
            reduced_live = reduced_live.max(memory::live().saturating_sub(before));
            black_box(&value);
        }
        black_box(&responses);
    }
    Ok(Stage {
        memory: scope.finish(),
        maximum_response_live_increment_bytes: response_live,
        maximum_response_and_reduced_live_increment_bytes: reduced_live,
    })
}
fn measure_reducer(
    cell: Cell,
    request: &RedisExecutionPlan,
    responses: &[ClientResponseEnvelope],
    payload: &[u8],
) -> Result<Stage, Box<dyn Error>> {
    let scope = memory::Scope::start();
    let before = scope.before;
    let mut reduced_live = 0;
    for _ in 0..cell.iterations {
        let value = request.reduce(responses)?;
        validate_reduced(cell, responses, &value, payload)?;
        reduced_live = reduced_live.max(memory::live().saturating_sub(before));
        black_box(&value);
    }
    Ok(Stage {
        memory: scope.finish(),
        maximum_response_live_increment_bytes: 0,
        maximum_response_and_reduced_live_increment_bytes: reduced_live,
    })
}
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn git(root: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git").current_dir(root).args(args).output()?;
    if !output.status.success() {
        return Err("Git identity unavailable".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
fn identity(root: &Path, source: &str) -> Result<(), Box<dyn Error>> {
    if source != env!("OWNER_SOURCE_SHA")
        || git(root, &["rev-parse", "HEAD"])? != source
        || !git(root, &["status", "--porcelain"])?.is_empty()
    {
        return Err("clean runtime/build/source identity must match".into());
    }
    Ok(())
}

#[derive(Serialize)]
struct Receipt<'a> {
    schema_version: u32,
    profile_id: &'static str,
    tier: &'static str,
    promotable: bool,
    product_mutation: bool,
    source_commit: &'a str,
    binary_sha256: String,
    tool_lock_sha256: String,
    contract_sha256: String,
    cell_id: &'static str,
    operation: &'static str,
    hit: bool,
    payload_bytes: usize,
    iterations: u64,
    seed: u64,
    warmup_operations: u64,
    cache_time_ms: u64,
    key_sha256: String,
    payload_sha256: String,
    request_plan_sha256: String,
    exact_result_pointer_and_state_validation: bool,
    dispatch_only: Stage,
    reducer_only: Stage,
    dispatch_and_reduce: Stage,
    limitations: &'static [&'static str],
}
fn run<'a>(cell: Cell, source: &'a str, root: &Path) -> Result<Receipt<'a>, Box<dyn Error>> {
    let mut key = b"hc074-response-owner:\0\xff:".to_vec();
    key.extend_from_slice(&SEED.to_le_bytes());
    let payload = (0..cell.bytes)
        .map(|index| SEED.to_le_bytes()[index % 8])
        .collect::<Vec<_>>();
    let request = plan(cell.set, &key, &payload)?;
    let direct = Fixture::new(cell, &key, &payload)?;
    let combined = Fixture::new(cell, &key, &payload)?;
    warm(&direct, cell, &request, &payload, false)?;
    warm(&combined, cell, &request, &payload, true)?;
    let original = if cell.set {
        ClientResponse::Stored
    } else {
        ClientResponse::Value {
            value: cell.hit.then(|| payload.clone()),
        }
    };
    let responses = [ClientResponseEnvelope::ok("response-owner-074", original)];
    validate_native(cell, &responses, &payload)?;
    let dispatch_only = measure_dispatch(&direct, cell, &request, &payload, false)?;
    let reducer_only = measure_reducer(cell, &request, &responses, &payload)?;
    let dispatch_and_reduce = measure_dispatch(&combined, cell, &request, &payload, true)?;
    direct.reconcile(cell, &key, &payload, WARMUP + cell.iterations)?;
    combined.reconcile(cell, &key, &payload, WARMUP + cell.iterations)?;
    Ok(Receipt {
        schema_version: 1, profile_id: PROFILE, tier: "local-d1-owner-attribution", promotable: false,
        product_mutation: false, source_commit: source,
        binary_sha256: digest(&fs::read(std::env::current_exe()?)?),
        tool_lock_sha256: digest(&fs::read(root.join("tools/resp-response-owner-074/Cargo.lock"))?),
        contract_sha256: digest(&fs::read(root.join("docs/testing/performance/0.74/response-reduction-attribution-contract.toml"))?),
        cell_id: cell.id, operation: if cell.set { "set" } else { "get" }, hit: cell.hit,
        payload_bytes: cell.bytes, iterations: cell.iterations, seed: SEED, warmup_operations: WARMUP,
        cache_time_ms: CLOCK, key_sha256: digest(&key), payload_sha256: digest(&payload),
        request_plan_sha256: digest(&serde_json::to_vec(request.initial_requests())?),
        exact_result_pointer_and_state_validation: true, dispatch_only, reducer_only, dispatch_and_reduce,
        limitations: &[
            "fixed public execution plan; excludes decode, command translation, server request IDs, encoding and IO",
            "deterministic test clock; not production clock/expiry CPU attribution",
            "tool-only process-wide requested-layout allocator; no concurrent tasks",
            "reducer source response prebuilt outside window; live peaks are increments, not additive stage totals",
            "distinct pointer plus source Vec clone attributes payload duplication, not hardware memcpy counts",
            "no candidate, native nonregression, CPU, latency, goodput, RSS or allocator-retention claim",
        ],
    })
}
fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(
            "usage: resp-response-owner-074 <source-sha> <cell-id> <new-output.json>".into(),
        );
    }
    let cell = CELLS
        .iter()
        .find(|cell| cell.id == args[1])
        .copied()
        .ok_or("unknown cell")?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    identity(&root, &args[0])?;
    let output = Path::new(&args[2]);
    // Reserve before measuring; no overwritten or silently retried receipt.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    let receipt = run(cell, &args[0], &root)?;
    identity(&root, &args[0])?;
    serde_json::to_writer_pretty(&mut file, &receipt)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_cells_reconcile_dispatch_reduce_and_owner_release() {
        for mut cell in CELLS {
            cell.iterations = 3;
            let payload = vec![0xff; cell.bytes];
            let request = plan(cell.set, b"binary\0\xffkey", &payload).unwrap();
            let fixture = Fixture::new(cell, b"binary\0\xffkey", &payload).unwrap();
            let stage = measure_dispatch(&fixture, cell, &request, &payload, true).unwrap();
            assert_eq!(
                stage.memory.live_before_bytes,
                stage.memory.live_after_bytes
            );
            fixture
                .reconcile(cell, b"binary\0\xffkey", &payload, 3)
                .unwrap();
        }
    }
    #[test]
    fn isolated_reducer_has_exact_one_payload_copy_and_no_retained_increment() {
        for cell in CELLS {
            let payload = vec![0x5a; cell.bytes];
            let request = plan(cell.set, b"key", &payload).unwrap();
            let result = if cell.set {
                ClientResponse::Stored
            } else {
                ClientResponse::Value {
                    value: cell.hit.then(|| payload.clone()),
                }
            };
            let responses = [ClientResponseEnvelope::ok("id", result)];
            let stage = measure_reducer(cell, &request, &responses, &payload).unwrap();
            let expected = if !cell.set && cell.hit {
                cell.bytes as u64
            } else {
                0
            };
            assert_eq!(
                stage.memory.gross_allocated_bytes,
                expected * cell.iterations
            );
            assert_eq!(stage.memory.peak_live_above_start_bytes, expected);
            assert_eq!(
                stage.memory.live_after_bytes,
                stage.memory.live_before_bytes
            );
        }
    }
    #[test]
    fn validators_reject_changed_payload_shape_count_and_state() {
        let cell = CELLS[1];
        let payload = vec![1; cell.bytes];
        let responses = [ClientResponseEnvelope::ok(
            "id",
            ClientResponse::Value {
                value: Some(payload.clone()),
            },
        )];
        assert!(validate_native(cell, &[], &payload).is_err());
        assert!(validate_native(cell, &responses, &[2]).is_err());
        assert!(validate_reduced(cell, &responses, &RespValue::Null, &payload).is_err());
        let request = plan(false, b"key", &payload).unwrap();
        let fixture = Fixture::new(cell, b"key", &payload).unwrap();
        assert!(fixture.reconcile(cell, b"key", &payload, 1).is_err());
        assert!(request.reduce(&[]).is_err());
        assert!(request
            .reduce(&[ClientResponseEnvelope::ok("id", ClientResponse::Stored)])
            .is_err());
    }
    #[test]
    fn validation_and_pointer_observation_allocate_nothing() {
        let cell = CELLS[1];
        let payload = vec![1; cell.bytes];
        let responses = [ClientResponseEnvelope::ok(
            "id",
            ClientResponse::Value {
                value: Some(payload.clone()),
            },
        )];
        let value = RespValue::BulkString(payload.clone());
        let scope = memory::Scope::start();
        for _ in 0..100 {
            validate_native(cell, &responses, &payload).unwrap();
            validate_reduced(cell, &responses, &value, &payload).unwrap();
        }
        assert_eq!(scope.finish().gross_allocated_bytes, 0);
    }
    #[test]
    fn bad_identity_fails_before_workload() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert!(identity(&root, "not-the-build-sha").is_err());
    }
}
