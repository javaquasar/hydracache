use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{self, Read};
use std::path::PathBuf;
use std::time::Duration;

use hydracache_loadgen::profile::{PerformanceProfile, RunnerFingerprint};
use hydracache_loadgen::report::{BuildIdentity, SourceIdentity};
use hydracache_loadgen::resp_external::{
    parse_redis_benchmark_csv, read_stream_bounded, run_redis_benchmark, sha256,
    ExternalRepeatStateEvidence, ExternalToolError, ExternalToolExecutor,
    ExternalToolPrebuildReceipt, ExternalToolPrebuildReceiptPayload,
    ExternalToolProvenanceRegistry, ExternalToolRunOutcome, LaunchError, MissingToolPolicy,
    ProcessCapture, ProcessLimits, RedisBenchmarkCase, RedisBenchmarkContract,
    RedisBenchmarkCsvError, RedisBenchmarkEndpoint, RedisBenchmarkRunContext, ResolvedExternalTool,
    RespOpenLoopEndpointCapability, SelectedDaemonReceipt, SelectedDaemonReceiptPayload,
    StreamCaptureError, CLOSED_LOOP_METHODOLOGY, EXTERNAL_PREBUILD_RECEIPT_VERSION,
    NODE_LOCAL_STATE_SCOPE, PINNED_REDIS_BENCHMARK_VERSION, PINNED_TOOL_IDENTITY_POLICY,
    REDIS_BENCHMARK_CSV_HEADER, REDIS_BENCHMARK_MEASUREMENT_ID, SELECTED_DAEMON_RECEIPT_VERSION,
    SELECTED_RESP_BOUNDARY, SUPPLEMENTAL_CLAIM_SCOPE,
};

const CONTRACT: &str =
    include_str!("../../../docs/testing/perf-scenarios/0.67/resp-external-redis-benchmark-v1.toml");
const PROVENANCE: &str =
    include_str!("../../../docs/testing/perf-scenarios/0.67/redis-benchmark-provenance-v1.toml");
const CSV_HEADER: &str = "\"test\",\"rps\",\"avg_latency_ms\",\"min_latency_ms\",\"p50_latency_ms\",\"p95_latency_ms\",\"p99_latency_ms\",\"max_latency_ms\"\n";
const VALID_CSV: &[u8] = concat!(
    "\"test\",\"rps\",\"avg_latency_ms\",\"min_latency_ms\",\"p50_latency_ms\",\"p95_latency_ms\",\"p99_latency_ms\",\"max_latency_ms\"\n",
    "\"SET\",\"11234.50\",\"0.091\",\"0.032\",\"0.087\",\"0.119\",\"0.135\",\"0.311\"\n",
    "\"GET\",\"12345.67\",\"0.081\",\"0.024\",\"0.079\",\"0.103\",\"0.119\",\"0.287\"\n",
    "\"MSET (10 keys)\",\"5432.10\",\"0.184\",\"0.071\",\"0.175\",\"0.247\",\"0.279\",\"0.615\"\n",
)
.as_bytes();
type CsvErrorPredicate = fn(&RedisBenchmarkCsvError) -> bool;

fn contract() -> RedisBenchmarkContract {
    RedisBenchmarkContract::parse_toml(CONTRACT).expect("committed W3 external-tool contract")
}

fn provenance_registry() -> ExternalToolProvenanceRegistry {
    ExternalToolProvenanceRegistry::parse_toml(PROVENANCE)
        .expect("committed W3 external-tool provenance registry")
}

fn capture(exit_code: i32, stdout: &[u8], stderr: &[u8]) -> ProcessCapture {
    ProcessCapture {
        exit_code: Some(exit_code),
        timed_out: false,
        stdout: stdout.to_vec(),
        stderr: stderr.to_vec(),
    }
}

struct ScriptedExecutor {
    results: RefCell<VecDeque<Result<ProcessCapture, LaunchError>>>,
    resolutions: RefCell<VecDeque<Result<ResolvedExternalTool, LaunchError>>>,
    calls: RefCell<Vec<(ResolvedExternalTool, Vec<String>, ProcessLimits)>>,
    resolve_calls: RefCell<usize>,
    platform_key: String,
}

impl ScriptedExecutor {
    fn new(results: impl IntoIterator<Item = Result<ProcessCapture, LaunchError>>) -> Self {
        Self {
            results: RefCell::new(results.into_iter().collect()),
            resolutions: RefCell::new(VecDeque::new()),
            calls: RefCell::new(Vec::new()),
            resolve_calls: RefCell::new(0),
            platform_key: "linux-x86_64-gnu".to_owned(),
        }
    }

    fn with_resolutions(
        results: impl IntoIterator<Item = Result<ProcessCapture, LaunchError>>,
        resolutions: impl IntoIterator<Item = Result<ResolvedExternalTool, LaunchError>>,
    ) -> Self {
        Self {
            results: RefCell::new(results.into_iter().collect()),
            resolutions: RefCell::new(resolutions.into_iter().collect()),
            calls: RefCell::new(Vec::new()),
            resolve_calls: RefCell::new(0),
            platform_key: "linux-x86_64-gnu".to_owned(),
        }
    }

    fn on_platform(mut self, platform_key: &str) -> Self {
        self.platform_key = platform_key.to_owned();
        self
    }
}

impl ExternalToolExecutor for ScriptedExecutor {
    fn platform_key(&self) -> String {
        self.platform_key.clone()
    }

    fn resolve(&self, program: &str) -> Result<ResolvedExternalTool, LaunchError> {
        assert_eq!(program, "redis-benchmark");
        *self.resolve_calls.borrow_mut() += 1;
        self.resolutions
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| Ok(tool_identity()))
    }

    fn resolve_exact(
        &self,
        logical_program: &str,
        canonical_path: &std::path::Path,
    ) -> Result<ResolvedExternalTool, LaunchError> {
        assert_eq!(logical_program, "redis-benchmark");
        assert_eq!(canonical_path, tool_identity().canonical_path);
        *self.resolve_calls.borrow_mut() += 1;
        self.resolutions
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| Ok(tool_identity()))
    }

    fn execute(
        &self,
        tool: &ResolvedExternalTool,
        argv: &[String],
        limits: ProcessLimits,
    ) -> Result<ProcessCapture, LaunchError> {
        self.calls
            .borrow_mut()
            .push((tool.clone(), argv.to_vec(), limits));
        self.results
            .borrow_mut()
            .pop_front()
            .expect("scripted executor exhausted")
    }

    fn prepare_repeat_state(
        &self,
        _endpoint: &RedisBenchmarkEndpoint,
        case: &RedisBenchmarkCase,
        _repeat: u8,
    ) -> Result<ExternalRepeatStateEvidence, LaunchError> {
        Ok(ExternalRepeatStateEvidence::deterministic_fixture(case))
    }
}

fn tool_identity() -> ResolvedExternalTool {
    let canonical_path = if cfg!(windows) {
        PathBuf::from(r"C:\perf-tools\redis-benchmark-7.2.5.exe")
    } else {
        PathBuf::from("/opt/perf-tools/redis-benchmark-7.2.5")
    };
    ResolvedExternalTool {
        requested_program: "redis-benchmark".to_owned(),
        canonical_path,
        binary_sha256: sha256(b"scripted redis-benchmark 7.2.5 fixture binary"),
    }
}

fn run_context(
    contract: &RedisBenchmarkContract,
    registry: &ExternalToolProvenanceRegistry,
) -> RedisBenchmarkRunContext {
    let runner_fingerprint = "reference-runner-fixture-v1".to_owned();
    let runner_profile = PerformanceProfile {
        name: contract.tool.required_runner_profile.clone(),
        required_runner_class: "dedicated-linux-perf".to_owned(),
        allowed_fingerprints: vec![runner_fingerprint.clone()],
        minimum_logical_cores: 8,
        required_cpu_affinity: "0-7".to_owned(),
        required_cgroup_cpu_quota: "max".to_owned(),
        require_dedicated: true,
        maximum_calibration_score: 1.0,
    };
    let observed_runner = RunnerFingerprint {
        runner_class: "dedicated-linux-perf".to_owned(),
        fingerprint: runner_fingerprint,
        cpu_model: "fixture-cpu".to_owned(),
        logical_cores: 8,
        ram_bytes: 16 * 1024 * 1024 * 1024,
        os: "linux".to_owned(),
        kernel: "fixture-kernel".to_owned(),
        cpu_affinity: "0-7".to_owned(),
        cgroup_cpu_quota: "max".to_owned(),
        governor: "performance".to_owned(),
        turbo: "disabled".to_owned(),
        shared_hardware: false,
        calibration_score: 0.5,
    };
    let prebuild_manifest_sha256 = sha256(b"fixture prebuild manifest");
    let daemon_binary_sha256 = sha256(b"fixture hydracache-server binary");
    let tool = tool_identity();
    let build = BuildIdentity {
        prebuild_contract_digest: sha256(b"fixture prebuild contract"),
        prebuild_manifest_sha256: prebuild_manifest_sha256.clone(),
        binary_sha256: vec![
            ("hydracache-server".to_owned(), daemon_binary_sha256.clone()),
            ("redis-benchmark".to_owned(), tool.binary_sha256.clone()),
        ],
    };
    let provenance = registry
        .approved_entry("linux-x86_64-gnu", &contract.tool.required_provenance_id)
        .expect("approved fixture provenance");
    let external_tool_prebuild =
        ExternalToolPrebuildReceipt::seal(ExternalToolPrebuildReceiptPayload {
            schema_version: EXTERNAL_PREBUILD_RECEIPT_VERSION,
            platform_key: provenance.platform_key.clone(),
            provenance_id: provenance.provenance_id.clone(),
            provenance_registry_sha256: registry.digest(),
            source_archive_sha256: provenance
                .provenance
                .source_archive_sha256()
                .map(str::to_owned),
            tool_binary_id: "redis-benchmark".to_owned(),
            tool_canonical_path: tool.canonical_path,
            tool_binary_sha256: tool.binary_sha256,
            prebuild_manifest_sha256: prebuild_manifest_sha256.clone(),
        });
    let endpoint_capability_sha256 = sha256(
        format!(
            "real-daemon-resp-readiness:{}:{}",
            contract.endpoint.host, contract.endpoint.port
        )
        .as_bytes(),
    );
    let selected_daemon = SelectedDaemonReceipt::seal(SelectedDaemonReceiptPayload {
        schema_version: SELECTED_DAEMON_RECEIPT_VERSION,
        node_id: "node-0".to_owned(),
        endpoint: contract.endpoint.clone(),
        daemon_binary_id: "hydracache-server".to_owned(),
        daemon_binary_sha256,
        prebuild_manifest_sha256,
        open_loop_endpoint_capability_sha256: endpoint_capability_sha256.clone(),
        capability_source: "real-daemon-resp-readiness".to_owned(),
        daemon_processes: true,
        resp_listener_capability: true,
        state_scope: NODE_LOCAL_STATE_SCOPE.to_owned(),
        selected_endpoint_only: true,
        automatic_failover: false,
    });
    RedisBenchmarkRunContext {
        runner_profile,
        observed_runner,
        source: SourceIdentity {
            git_commit: sha256(b"fixture source commit"),
            cargo_lock_sha256: sha256(b"fixture Cargo.lock"),
            toolchain: "rustc-fixture".to_owned(),
            build_flags: vec!["--release".to_owned(), "--locked".to_owned()],
        },
        build,
        open_loop_endpoint: RespOpenLoopEndpointCapability {
            endpoint: contract.endpoint.clone(),
            endpoint_capability_sha256,
        },
        selected_daemon,
        external_tool_prebuild,
        committed_contract_sha256: contract.committed_digest(),
    }
}

fn run_scripted<E: ExternalToolExecutor>(
    contract: &RedisBenchmarkContract,
    policy: MissingToolPolicy,
    executor: &E,
) -> Result<ExternalToolRunOutcome, ExternalToolError> {
    let registry = provenance_registry();
    let context = run_context(contract, &registry);
    run_redis_benchmark(contract, policy, executor, &registry, &context)
}

fn csv_row(name: &str, rps: &str, latency: &str) -> String {
    format!(
        "\"{name}\",\"{rps}\",\"{latency}\",\"{latency}\",\"{latency}\",\"{latency}\",\"{latency}\",\"{latency}\"\n"
    )
}

fn one_row_csv(name: &str, rps: &str, latency: &str) -> String {
    format!("{CSV_HEADER}{}", csv_row(name, rps, latency))
}

fn complete_csv(get_rps: &str, set_rps: &str, mset_rps: &str) -> Vec<u8> {
    format!(
        "{CSV_HEADER}{}{}{}",
        csv_row("SET", set_rps, "0.1"),
        csv_row("GET", get_rps, "0.1"),
        csv_row("MSET (10 keys)", mset_rps, "0.2")
    )
    .into_bytes()
}

fn successful_script(
    contract: &RedisBenchmarkContract,
) -> Vec<Result<ProcessCapture, LaunchError>> {
    let version = format!("{PINNED_REDIS_BENCHMARK_VERSION}\n");
    let mut results = vec![Ok(capture(0, version.as_bytes(), b""))];
    results.extend(
        contract.cases.iter().flat_map(|_| {
            (0..contract.tool.repeats_per_case).map(|_| Ok(capture(0, VALID_CSV, b"")))
        }),
    );
    results
}

#[test]
fn redis_benchmark_contract_builds_exact_argv_and_preserves_closed_loop_identity() {
    let contract = contract();
    assert_eq!(contract.tool.program, "redis-benchmark");
    assert_eq!(contract.version_argv(), ["--version"]);
    assert_eq!(
        contract.tool.expected_version,
        PINNED_REDIS_BENCHMARK_VERSION
    );
    assert_eq!(contract.tool.identity_policy, PINNED_TOOL_IDENTITY_POLICY);
    assert_eq!(contract.tool.version_timeout_seconds, 10);
    assert_eq!(contract.tool.case_timeout_seconds, 300);
    assert_eq!(contract.tool.max_stdout_bytes, 262_144);
    assert_eq!(contract.tool.max_stderr_bytes, 65_536);
    assert_eq!(contract.tool.stderr_policy, "must-be-empty");
    assert_eq!(contract.tool.repeats_per_case, 3);
    assert_eq!(contract.tool.max_robust_spread_ratio, 0.15);
    assert_eq!(contract.tool.required_runner_profile, "reference-v1");
    assert_eq!(contract.cases.len(), 4);

    assert_eq!(
        contract.benchmark_argv(&contract.cases[0]),
        [
            "--csv",
            "-h",
            "127.0.0.1",
            "-p",
            "6379",
            "-c",
            "1",
            "-n",
            "100000",
            "-P",
            "1",
            "-d",
            "256",
            "-t",
            "get,set,mset",
        ]
    );
    assert_eq!(
        contract.identity.measurement_id,
        REDIS_BENCHMARK_MEASUREMENT_ID
    );
    assert_eq!(contract.identity.methodology, CLOSED_LOOP_METHODOLOGY);
    assert_eq!(contract.identity.claim_scope, SUPPLEMENTAL_CLAIM_SCOPE);
    assert_eq!(contract.identity.state_scope, NODE_LOCAL_STATE_SCOPE);
    assert_eq!(contract.identity.network_boundary, SELECTED_RESP_BOUNDARY);
    assert!(!contract.identity.scheduled_send_latency);
    assert!(!contract.identity.capacity_knee_eligible);
}

#[test]
fn redis_benchmark_provenance_pins_the_real_725_source_archive_and_recipe() {
    let contract = contract();
    let registry = provenance_registry();
    assert_eq!(registry.entries.len(), 1);
    assert_eq!(
        contract.tool.provenance_registry_path,
        "docs/testing/perf-scenarios/0.67/redis-benchmark-provenance-v1.toml"
    );
    let entry = registry
        .approved_entry("linux-x86_64-gnu", &contract.tool.required_provenance_id)
        .unwrap();
    assert_eq!(
        entry.provenance.source_archive_sha256(),
        Some("5981179706f8391f03be91d951acafaeda91af7fac56beffb2701963103e423d")
    );
    let serialized = serde_json::to_string(entry).unwrap();
    assert!(serialized.contains("redis-7.2.5.tar.gz"));
    assert!(serialized.contains("3386454"));
    assert!(!serialized.contains("latest"));
}

#[test]
fn redis_benchmark_process_limits_and_identity_policy_are_bounded_and_digest_bound() {
    let contract = contract();
    let original_digest = contract.digest();
    for broken in [
        CONTRACT.replace(
            "version_timeout_seconds = 10",
            "version_timeout_seconds = 0",
        ),
        CONTRACT.replace("case_timeout_seconds = 300", "case_timeout_seconds = 1801"),
        CONTRACT.replace("max_stdout_bytes = 262144", "max_stdout_bytes = 0"),
        CONTRACT.replace("max_stderr_bytes = 65536", "max_stderr_bytes = 1048577"),
        CONTRACT.replace(
            "identity_policy = \"canonical-path-sha256-pinned-per-run\"",
            "identity_policy = \"version-string-only\"",
        ),
        CONTRACT.replace(
            "stderr_policy = \"must-be-empty\"",
            "stderr_policy = \"ignore\"",
        ),
    ] {
        assert!(matches!(
            RedisBenchmarkContract::parse_toml(&broken),
            Err(ExternalToolError::Contract(_))
        ));
    }

    let changed = CONTRACT.replace("case_timeout_seconds = 300", "case_timeout_seconds = 301");
    let changed = RedisBenchmarkContract::parse_toml(&changed).unwrap();
    assert_ne!(changed.digest(), original_digest);
}

#[test]
fn redis_benchmark_contract_rejects_version_argv_and_claim_relabeling() {
    for broken in [
        CONTRACT.replace(
            "expected_version = \"redis-benchmark 7.2.5\"",
            "expected_version = \"redis-benchmark latest\"",
        ),
        CONTRACT.replace(
            "version_args = [\"--version\"]",
            "version_args = [\"--help\"]",
        ),
        CONTRACT.replace(
            "claim_scope = \"supplemental-interop-throughput-no-slo-knee\"",
            "claim_scope = \"capacity-knee\"",
        ),
        CONTRACT.replace(
            "scheduled_send_latency = false",
            "scheduled_send_latency = true",
        ),
        CONTRACT.replace("state_scope = \"node-local\"", "state_scope = \"cluster\""),
    ] {
        let error = RedisBenchmarkContract::parse_toml(&broken).unwrap_err();
        assert!(matches!(error, ExternalToolError::Contract(_)), "{error}");
    }
}

#[test]
fn redis_benchmark_csv_parser_accepts_only_the_exact_complete_row_set() {
    let expected = vec![
        "GET".to_owned(),
        "SET".to_owned(),
        "MSET (10 keys)".to_owned(),
    ];
    let rows = parse_redis_benchmark_csv(VALID_CSV, &expected).unwrap();
    assert_eq!(
        rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(),
        ["GET", "SET", "MSET (10 keys)"]
    );
    assert_eq!(rows[0].requests_per_second, "12345.67");
    assert_eq!(rows[0].requests_per_second_f64(), 12345.67);
    assert_eq!(rows[0].average_latency_ms, "0.081");
    assert_eq!(rows[0].p99_latency_ms, "0.119");
    assert_eq!(REDIS_BENCHMARK_CSV_HEADER[0], "test");
    assert_eq!(REDIS_BENCHMARK_CSV_HEADER.len(), 8);
}

#[test]
fn redis_benchmark_csv_parser_rejects_truncated_duplicate_missing_unknown_and_swallowed_output() {
    let expected = vec![
        "GET".to_owned(),
        "SET".to_owned(),
        "MSET (10 keys)".to_owned(),
    ];
    let cases: Vec<(Vec<u8>, CsvErrorPredicate)> = vec![
        (Vec::new(), |error| {
            matches!(error, RedisBenchmarkCsvError::Empty)
        }),
        (VALID_CSV[..VALID_CSV.len() - 1].to_vec(), |error| {
            matches!(error, RedisBenchmarkCsvError::Truncated)
        }),
        (
            format!(
                "{CSV_HEADER}{}{}{}{}",
                csv_row("GET", "1", "0.1"),
                csv_row("GET", "2", "0.1"),
                csv_row("SET", "1", "0.1"),
                csv_row("MSET (10 keys)", "1", "0.1")
            )
            .into_bytes(),
            |error| matches!(error, RedisBenchmarkCsvError::Duplicate(row) if row == "GET"),
        ),
        (
            format!(
                "{CSV_HEADER}{}{}",
                csv_row("GET", "1", "0.1"),
                csv_row("SET", "1", "0.1")
            )
            .into_bytes(),
            |error| matches!(error, RedisBenchmarkCsvError::Missing(rows) if rows == &["MSET (10 keys)".to_owned()]),
        ),
        (
            format!(
                "{CSV_HEADER}{}{}{}",
                csv_row("GET", "1", "0.1"),
                csv_row("SET", "1", "0.1"),
                csv_row("PING_INLINE", "1", "0.1")
            )
            .into_bytes(),
            |error| matches!(error, RedisBenchmarkCsvError::Unknown(row) if row == "PING_INLINE"),
        ),
        (
            one_row_csv("GET", "1", "0.1")
                .replace("\"max_latency_ms\"", "\"p100_latency_ms\"")
                .into_bytes(),
            |error| matches!(error, RedisBenchmarkCsvError::InvalidRow { line: 1, .. }),
        ),
    ];
    for (csv, predicate) in cases {
        let error = parse_redis_benchmark_csv(&csv, &expected).unwrap_err();
        assert!(predicate(&error), "unexpected error: {error}");
    }
}

#[test]
fn redis_benchmark_csv_parser_rejects_nan_inf_zero_and_malformed_rows() {
    let expected = vec!["GET".to_owned()];
    for value in ["NaN", "inf", "-inf"] {
        let csv = one_row_csv("GET", value, "0.1");
        assert!(matches!(
            parse_redis_benchmark_csv(csv.as_bytes(), &expected),
            Err(RedisBenchmarkCsvError::NonFinite { .. })
        ));
    }
    for value in ["0", "-1", "not-a-number"] {
        let csv = one_row_csv("GET", value, "0.1");
        assert!(matches!(
            parse_redis_benchmark_csv(csv.as_bytes(), &expected),
            Err(RedisBenchmarkCsvError::InvalidThroughput { .. })
        ));
    }
    for latency in ["NaN", "inf", "-1", "not-a-number"] {
        let csv = one_row_csv("GET", "1", latency);
        assert!(matches!(
            parse_redis_benchmark_csv(csv.as_bytes(), &expected),
            Err(RedisBenchmarkCsvError::InvalidLatency { .. })
        ));
    }
    for csv in [
        format!("{CSV_HEADER}GET,1\n"),
        format!("{CSV_HEADER}\"GET\",\"1\",\"extra\"\n"),
        format!("{CSV_HEADER}\"GET\",\"1\n"),
        "\n".to_owned(),
    ] {
        assert!(matches!(
            parse_redis_benchmark_csv(csv.as_bytes(), &expected),
            Err(RedisBenchmarkCsvError::InvalidRow { .. })
        ));
    }
}

struct PrefixThenError {
    prefix: &'static [u8],
    offset: usize,
}

impl Read for PrefixThenError {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.offset == self.prefix.len() {
            return Err(io::Error::other("injected pipe read failure"));
        }
        let count = buffer.len().min(self.prefix.len() - self.offset);
        buffer[..count].copy_from_slice(&self.prefix[self.offset..self.offset + count]);
        self.offset += count;
        Ok(count)
    }
}

#[test]
fn external_tool_pipe_capture_rejects_read_errors_and_bytes_beyond_the_committed_cap() {
    let error = read_stream_bounded(
        PrefixThenError {
            prefix: VALID_CSV,
            offset: 0,
        },
        262_144,
    )
    .unwrap_err();
    assert!(matches!(error, StreamCaptureError::Read(message) if message.contains("injected")));

    let error = read_stream_bounded(io::Cursor::new(b"12345"), 4).unwrap_err();
    assert_eq!(error, StreamCaptureError::LimitExceeded { limit: 4 });
    assert_eq!(
        read_stream_bounded(io::Cursor::new(b"1234"), 4).unwrap(),
        b"1234"
    );
}

#[test]
fn redis_benchmark_evidence_binds_version_exact_argv_and_raw_stream_hashes() {
    let contract = contract();
    let version_stdout = format!("{PINNED_REDIS_BENCHMARK_VERSION}\n");
    let mut results = vec![Ok(capture(0, version_stdout.as_bytes(), b""))];
    results.extend(
        contract.cases.iter().flat_map(|_| {
            (0..contract.tool.repeats_per_case).map(|_| Ok(capture(0, VALID_CSV, b"")))
        }),
    );
    let executor = ScriptedExecutor::new(results);
    let outcome =
        run_scripted(&contract, MissingToolPolicy::MandatoryFailClosed, &executor).unwrap();
    let ExternalToolRunOutcome::Completed(evidence) = outcome else {
        panic!("mandatory run unexpectedly skipped")
    };

    assert_eq!(evidence.tool_version, PINNED_REDIS_BENCHMARK_VERSION);
    assert_eq!(evidence.tool_identity, tool_identity());
    assert_eq!(evidence.contract_sha256, contract.digest());
    assert_eq!(evidence.effective_contract_sha256, contract.digest());
    assert_eq!(evidence.cases.len(), contract.cases.len());
    assert!(evidence.cases.iter().all(|case| {
        let expected = &case.repeats[0].initial_state.state_digest;
        case.repeats
            .iter()
            .all(|repeat| &repeat.initial_state.state_digest == expected)
    }));
    assert_eq!(
        evidence.version_probe.stdout_sha256,
        sha256(version_stdout.as_bytes())
    );
    assert_eq!(evidence.version_probe.stderr_sha256, sha256(b""));
    assert_eq!(
        evidence.cases[0].repeats[0].process.stdout,
        String::from_utf8(VALID_CSV.to_vec()).unwrap()
    );
    assert_eq!(
        evidence.cases[0].repeats[0].process.stdout_sha256,
        sha256(VALID_CSV)
    );
    assert_eq!(
        evidence.cases[0].repeats[0].process.argv,
        contract.benchmark_argv(&contract.cases[0])
    );
    assert_eq!(
        evidence.version_probe.program,
        tool_identity().canonical_path.to_string_lossy()
    );
    assert_eq!(
        executor.calls.borrow().len(),
        1 + contract.cases.len() * usize::from(contract.tool.repeats_per_case)
    );
    assert_eq!(
        executor.calls.borrow()[0].2.timeout,
        Duration::from_secs(10)
    );
    assert_eq!(executor.calls.borrow()[0].2.max_stdout_bytes, 262_144);
    assert_eq!(executor.calls.borrow()[0].2.max_stderr_bytes, 65_536);
    assert!(executor
        .calls
        .borrow()
        .iter()
        .skip(1)
        .all(|call| call.2.timeout == Duration::from_secs(300)));
    assert!(executor
        .calls
        .borrow()
        .iter()
        .all(|call| call.0 == tool_identity()));
    assert_eq!(
        *executor.resolve_calls.borrow(),
        2 + contract.cases.len() * usize::from(contract.tool.repeats_per_case) * 2
    );
    assert!(executor.results.borrow().is_empty());
}

#[test]
fn committed_external_contract_digest_is_stable_when_the_selected_daemon_port_changes() {
    let committed = contract();
    let committed_digest = committed.committed_digest();
    let mut effective = committed.clone();
    effective.endpoint.port = effective.endpoint.port.saturating_add(1);
    assert_ne!(effective.digest(), committed_digest);

    let registry = provenance_registry();
    let context = run_context(&effective, &registry);
    let executor = ScriptedExecutor::new(successful_script(&effective));
    let outcome = run_redis_benchmark(
        &effective,
        MissingToolPolicy::MandatoryFailClosed,
        &executor,
        &registry,
        &context,
    )
    .unwrap();
    let ExternalToolRunOutcome::Completed(evidence) = outcome else {
        panic!("mandatory run unexpectedly skipped")
    };

    assert_eq!(evidence.contract_sha256, committed_digest);
    assert_eq!(evidence.effective_contract_sha256, effective.digest());
    assert_ne!(evidence.contract_sha256, evidence.effective_contract_sha256);
    assert_eq!(evidence.endpoint, effective.endpoint);
}

#[test]
fn redis_benchmark_missing_tool_skips_loud_locally_and_fails_closed_when_mandatory() {
    let contract = contract();
    let local = ScriptedExecutor::with_resolutions([], [Err(LaunchError::missing("not found"))]);
    let outcome = run_scripted(&contract, MissingToolPolicy::LocalSkipLoud, &local).unwrap();
    let ExternalToolRunOutcome::SkippedLoud(skip) = outcome else {
        panic!("missing local tool unexpectedly produced evidence")
    };
    assert_eq!(skip.code, "external-tool-missing-local-skip-loud");
    assert!(skip.message.contains("evidence was not produced"));
    assert_eq!(skip.argv, ["--version"]);

    let mandatory =
        ScriptedExecutor::with_resolutions([], [Err(LaunchError::missing("not found"))]);
    let error = run_scripted(
        &contract,
        MissingToolPolicy::MandatoryFailClosed,
        &mandatory,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ExternalToolError::RequiredToolMissing { .. }
    ));
}

#[test]
fn redis_benchmark_rejects_nonzero_stderr_version_drift_and_truncated_csv() {
    let contract = contract();
    let valid_version = format!("{PINNED_REDIS_BENCHMARK_VERSION}\n");
    let scripts = [
        vec![Ok(capture(2, b"partial", b"failed"))],
        vec![Ok(capture(0, valid_version.as_bytes(), b"warning"))],
        vec![Ok(capture(0, b"redis-benchmark 7.2.4\n", b""))],
        vec![
            Ok(capture(0, valid_version.as_bytes(), b"")),
            Ok(capture(0, &VALID_CSV[..VALID_CSV.len() - 1], b"")),
        ],
    ];
    for script in scripts {
        let executor = ScriptedExecutor::new(script);
        let error =
            run_scripted(&contract, MissingToolPolicy::MandatoryFailClosed, &executor).unwrap_err();
        match error {
            ExternalToolError::OutputRejected {
                stdout_sha256,
                stderr_sha256,
                ..
            } => {
                assert_eq!(stdout_sha256.len(), 64);
                assert_eq!(stderr_sha256.len(), 64);
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}

#[test]
fn redis_benchmark_timeout_is_rejected_with_raw_stream_hashes() {
    let contract = contract();
    let executor = ScriptedExecutor::new([Ok(ProcessCapture {
        exit_code: None,
        timed_out: true,
        stdout: b"partial-version".to_vec(),
        stderr: b"timeout diagnostics".to_vec(),
    })]);
    let error =
        run_scripted(&contract, MissingToolPolicy::MandatoryFailClosed, &executor).unwrap_err();
    match error {
        ExternalToolError::OutputRejected {
            reason,
            stdout_sha256,
            stderr_sha256,
            ..
        } => {
            assert!(reason.contains("timeout"));
            assert_eq!(stdout_sha256, sha256(b"partial-version"));
            assert_eq!(stderr_sha256, sha256(b"timeout diagnostics"));
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn redis_benchmark_tool_disappearing_after_version_probe_is_never_a_local_skip() {
    let contract = contract();
    let version = format!("{PINNED_REDIS_BENCHMARK_VERSION}\n");
    let executor = ScriptedExecutor::with_resolutions(
        [Ok(capture(0, version.as_bytes(), b""))],
        [
            Ok(tool_identity()),
            Ok(tool_identity()),
            Err(LaunchError::missing("removed during run")),
        ],
    );
    let error = run_scripted(&contract, MissingToolPolicy::LocalSkipLoud, &executor).unwrap_err();
    assert!(matches!(error, ExternalToolError::Launch { .. }));
    assert!(error.to_string().contains("removed during run"));
}

#[test]
fn redis_benchmark_rejects_canonical_path_or_binary_sha_drift_after_the_version_probe() {
    let contract = contract();
    let version = format!("{PINNED_REDIS_BENCHMARK_VERSION}\n");
    for changed in [
        ResolvedExternalTool {
            canonical_path: if cfg!(windows) {
                PathBuf::from(r"C:\other\redis-benchmark.exe")
            } else {
                PathBuf::from("/other/redis-benchmark")
            },
            ..tool_identity()
        },
        ResolvedExternalTool {
            binary_sha256: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
                .to_owned(),
            ..tool_identity()
        },
    ] {
        let executor = ScriptedExecutor::with_resolutions(
            [Ok(capture(0, version.as_bytes(), b""))],
            [Ok(tool_identity()), Ok(changed)],
        );
        let error =
            run_scripted(&contract, MissingToolPolicy::MandatoryFailClosed, &executor).unwrap_err();
        assert!(matches!(error, ExternalToolError::ToolIdentity(_)));
    }
}

#[test]
fn unsupported_platform_skips_loud_locally_and_fails_before_resolution_when_mandatory() {
    let contract = contract();
    let local = ScriptedExecutor::new([]).on_platform("windows-x86_64-msvc");
    let outcome = run_scripted(&contract, MissingToolPolicy::LocalSkipLoud, &local).unwrap();
    let ExternalToolRunOutcome::SkippedLoud(skip) = outcome else {
        panic!("unsupported platform unexpectedly produced evidence")
    };
    assert_eq!(
        skip.code,
        "external-tool-platform-unsupported-local-skip-loud"
    );
    assert_eq!(skip.platform_key, "windows-x86_64-msvc");
    assert_eq!(*local.resolve_calls.borrow(), 0);

    let mandatory = ScriptedExecutor::new([]).on_platform("windows-x86_64-msvc");
    let error = run_scripted(
        &contract,
        MissingToolPolicy::MandatoryFailClosed,
        &mandatory,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ExternalToolError::UnsupportedMandatoryPlatform { platform_key }
            if platform_key == "windows-x86_64-msvc"
    ));
    assert_eq!(*mandatory.resolve_calls.borrow(), 0);
}

#[test]
fn mandatory_run_cross_binds_actual_tool_sha_and_prebuild_receipt() {
    let contract = contract();
    let registry = provenance_registry();
    let mut context = run_context(&contract, &registry);
    let receipt_tool_sha256 = sha256(b"different prebuilt redis-benchmark fixture");
    context.external_tool_prebuild.payload.tool_binary_sha256 = receipt_tool_sha256.clone();
    context.external_tool_prebuild =
        ExternalToolPrebuildReceipt::seal(context.external_tool_prebuild.payload.clone());
    context
        .build
        .binary_sha256
        .iter_mut()
        .find(|(id, _)| id == "redis-benchmark")
        .unwrap()
        .1 = receipt_tool_sha256;
    let executor = ScriptedExecutor::new([]);
    let error = run_redis_benchmark(
        &contract,
        MissingToolPolicy::MandatoryFailClosed,
        &executor,
        &registry,
        &context,
    )
    .unwrap_err();
    assert!(matches!(error, ExternalToolError::PrebuildReceipt(_)));
    assert_eq!(*executor.resolve_calls.borrow(), 1);
    assert!(executor.calls.borrow().is_empty());
}

#[test]
fn external_report_fails_closed_on_open_loop_endpoint_or_capability_mismatch() {
    let contract = contract();
    let registry = provenance_registry();
    for mutate in [
        |context: &mut RedisBenchmarkRunContext| context.open_loop_endpoint.endpoint.port += 1,
        |context: &mut RedisBenchmarkRunContext| {
            context.open_loop_endpoint.endpoint_capability_sha256 =
                sha256(b"different real-daemon endpoint capability")
        },
        |context: &mut RedisBenchmarkRunContext| {
            context.committed_contract_sha256 = sha256(b"arbitrary stable contract")
        },
    ] {
        let mut context = run_context(&contract, &registry);
        mutate(&mut context);
        let executor = ScriptedExecutor::new([]);
        let error = run_redis_benchmark(
            &contract,
            MissingToolPolicy::MandatoryFailClosed,
            &executor,
            &registry,
            &context,
        )
        .unwrap_err();
        assert!(matches!(error, ExternalToolError::PrebuildReceipt(_)));
        assert_eq!(*executor.resolve_calls.borrow(), 0);
    }
}

#[test]
fn external_report_recomputes_repeat_statistics_and_rejects_tampering() {
    let mut contract = contract();
    contract.cases.truncate(1);
    let csv_a = complete_csv("100", "200", "50");
    let csv_b = complete_csv("105", "210", "52");
    let csv_c = complete_csv("110", "220", "54");
    let version = format!("{PINNED_REDIS_BENCHMARK_VERSION}\n");
    let executor = ScriptedExecutor::new([
        Ok(capture(0, version.as_bytes(), b"")),
        Ok(capture(0, &csv_a, b"")),
        Ok(capture(0, &csv_b, b"")),
        Ok(capture(0, &csv_c, b"")),
    ]);
    let registry = provenance_registry();
    let context = run_context(&contract, &registry);
    let outcome = run_redis_benchmark(
        &contract,
        MissingToolPolicy::MandatoryFailClosed,
        &executor,
        &registry,
        &context,
    )
    .unwrap();
    let ExternalToolRunOutcome::Completed(evidence) = outcome else {
        panic!("mandatory run unexpectedly skipped")
    };
    let get = evidence.cases[0]
        .operations
        .iter()
        .find(|operation| operation.operation == "GET")
        .unwrap();
    assert_eq!(get.repeat_count, 3);
    assert_eq!(get.requests_per_second_samples, [100.0, 105.0, 110.0]);
    assert_eq!(get.minimum_requests_per_second, 100.0);
    assert_eq!(get.median_requests_per_second, 105.0);
    assert_eq!(get.maximum_requests_per_second, 110.0);
    assert!((get.robust_spread_ratio - (10.0 / 105.0)).abs() < f64::EPSILON);
    assert!(get.stable);
    assert!(evidence.measurements_stable);
    assert!(evidence.ship_evidence_eligible);
    assert_eq!(evidence.provenance_registry_sha256, registry.digest());
    assert_eq!(
        evidence.run_context.open_loop_endpoint.endpoint,
        evidence.run_context.selected_daemon.payload.endpoint
    );
    assert_eq!(
        evidence
            .run_context
            .open_loop_endpoint
            .endpoint_capability_sha256,
        evidence
            .run_context
            .selected_daemon
            .payload
            .open_loop_endpoint_capability_sha256
    );
    evidence.validate(&contract, &registry).unwrap();

    let mut tampered_aggregate = (*evidence).clone();
    tampered_aggregate.cases[0].operations[0].median_requests_per_second += 1.0;
    assert!(matches!(
        tampered_aggregate.validate(&contract, &registry),
        Err(ExternalToolError::EvidenceValidation(_))
    ));

    let mut tampered_raw = (*evidence).clone();
    tampered_raw.cases[0].repeats[0].process.stdout.push(' ');
    assert!(matches!(
        tampered_raw.validate(&contract, &registry),
        Err(ExternalToolError::EvidenceValidation(_))
    ));
}

#[test]
fn completed_high_spread_external_run_is_not_suite_ship_evidence() {
    let mut contract = contract();
    contract.cases.truncate(1);
    let version = format!("{PINNED_REDIS_BENCHMARK_VERSION}\n");
    let low = complete_csv("100", "100", "100");
    let high = complete_csv("1000", "1000", "1000");
    let executor = ScriptedExecutor::new([
        Ok(capture(0, version.as_bytes(), b"")),
        Ok(capture(0, &low, b"")),
        Ok(capture(0, &high, b"")),
        Ok(capture(0, &low, b"")),
    ]);
    let registry = provenance_registry();
    let context = run_context(&contract, &registry);
    let outcome = run_redis_benchmark(
        &contract,
        MissingToolPolicy::MandatoryFailClosed,
        &executor,
        &registry,
        &context,
    )
    .unwrap();
    let ExternalToolRunOutcome::Completed(evidence) = outcome else {
        panic!("mandatory run unexpectedly skipped")
    };

    assert!(!evidence.measurements_stable);
    assert!(!evidence.ship_evidence_eligible);
    assert!(!evidence.stability_reasons.is_empty());
    assert!(
        !hydracache_loadgen::tiers::resp::external_evidence_is_ship_eligible(&evidence),
        "a completed but noisy external run must not reach a suite receipt"
    );
}

#[test]
fn mandatory_external_run_rejects_an_ineligible_runner_before_tool_resolution() {
    let contract = contract();
    let registry = provenance_registry();
    let mut context = run_context(&contract, &registry);
    context.observed_runner.shared_hardware = true;
    let executor = ScriptedExecutor::new(successful_script(&contract));
    let error = run_redis_benchmark(
        &contract,
        MissingToolPolicy::MandatoryFailClosed,
        &executor,
        &registry,
        &context,
    )
    .unwrap_err();
    assert!(matches!(error, ExternalToolError::RunnerIdentity(_)));
    assert_eq!(*executor.resolve_calls.borrow(), 0);
    assert!(executor.calls.borrow().is_empty());
}
