use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("bench-budget") => xtask::bench_budget::run(args.collect())?,
        Some("borrowed-suite-check") => xtask::migration_conformance::run_borrowed(args.collect())?,
        Some("canary-check") => xtask::canary_check::run(args.collect())?,
        Some("canary-sweep") => xtask::canary_sweep::run(args.collect())?,
        Some("client-plane-java-sdk-check") => xtask::client_plane_java::run(args.collect())?,
        Some("client-plane-bakeoff-check") => xtask::client_plane_bakeoff::run(args.collect())?,
        Some("client-plane-ci-check") => xtask::client_plane_ci::run_check(args.collect())?,
        Some("client-plane-ci-receipt") => xtask::client_plane_ci::run_receipt(args.collect())?,
        Some("client-plane-ci-admission") => xtask::client_plane_ci::run_admission(args.collect())?,
        Some("client-plane-docker-interop-check") => {
            xtask::client_plane_spike::run_docker(args.collect())?
        }
        Some("client-plane-compat-check") => xtask::client_plane_compat::run(args.collect())?,
        Some("client-plane-fault-check") => xtask::client_plane_fault::run(args.collect())?,
        Some("client-plane-generation-check") => {
            xtask::client_plane_generation::run(args.collect())?
        }
        Some("client-plane-spike-check") => xtask::client_plane_spike::run(args.collect())?,
        Some("client-plane-python-check") => xtask::client_plane_python::run_check(args.collect())?,
        Some("client-plane-python-generate") => {
            xtask::client_plane_python::run_generate(args.collect())?
        }
        Some("client-plane-rust-sdk-check") => xtask::client_plane_rust::run(args.collect())?,
        Some("client-conformance") => xtask::client_conformance::run(args.collect())?,
        Some("client-package-check") => xtask::client_package::run(args.collect())?,
        Some("client-schema-check") => xtask::client_schema::run(args.collect())?,
        Some("ci-admission-status") => {
            let code = xtask::ci_admission::run(args.collect())?;
            if code != 0 {
                std::process::exit(code);
            }
        }
        Some("ci-topology-check") => xtask::ci_topology::run(args.collect())?,
        Some("compat-check") => xtask::compat_check::run(args.collect())?,
        Some("coverage-ratchet-check") => xtask::coverage_ratchet::run(args.collect())?,
        Some("determinism-sweep") => xtask::determinism_sweep::run(args.collect())?,
        Some("doc-check") => xtask::doc_check::run(args.collect())?,
        Some("durable-inspect") => xtask::durable_inspect::run(args.collect())?,
        Some("evidence-run") => {
            let code = xtask::evidence_run::run(args.collect())?;
            if code != 0 {
                std::process::exit(code);
            }
        }
        Some("fast-suite-check") => xtask::fast_suite::run(args.collect())?,
        Some("gated-test-check") => xtask::gated_tests::run(args.collect())?,
        Some("legacy-client-check") => xtask::migration_conformance::run_legacy(args.collect())?,
        Some("miri-check") => xtask::miri_check::run(args.collect())?,
        Some("migration-conformance-check") => xtask::migration_conformance::run(args.collect())?,
        Some("memory-owner-inventory") => xtask::memory_ownership::run_inventory(args.collect())?,
        Some("memory-ownership-check") => xtask::memory_ownership::run_check(args.collect())?,
        Some("memory-decision-check") => xtask::memory_contracts::run_decisions(args.collect())?,
        Some("memory-statistics-check") => xtask::memory_contracts::run_statistics(args.collect())?,
        Some("allocator-capability-check") => {
            xtask::memory_contracts::run_allocator(args.collect())?
        }
        Some("perf-memory-preflight") => {
            xtask::memory_contracts::run_host_preflight(args.collect())?
        }
        Some("memory-compat-check") => xtask::memory_contracts::run_compat(args.collect())?,
        Some("memory-release-policy-check") => {
            xtask::memory_contracts::run_release_policy(args.collect())?
        }
        Some("memory-baseline-check") => xtask::memory_baseline::run_check(args.collect())?,
        Some("memory-baseline-status") => xtask::memory_baseline::run_status(args.collect())?,
        Some("postgres-conformance-check") => {
            xtask::migration_conformance::run_postgres(args.collect())?
        }
        Some("mutants") => xtask::mutants::run(args.collect())?,
        Some("perf-prebuild") => xtask::perf::run(args.collect())?,
        Some("perf-bootstrap") => xtask::perf_bootstrap::run(args.collect())?,
        Some("perf-full-dress") => xtask::perf_full_dress::run(args.collect())?,
        Some("perf-qualification") => xtask::perf_qualification::run(args.collect())?,
        Some("perf-reference") => xtask::perf_reference::run(args.collect())?,
        Some("perf-runner-preflight") => xtask::perf::run_preflight(args.collect())?,
        Some("perf-budget-check") => xtask::perf_budget::run(args.collect())?,
        Some("quarantine-check") => xtask::quarantine::run(args.collect())?,
        Some("raft-spec-check") => xtask::raft_spec_check::run(args.collect())?,
        Some("release-evidence") => xtask::release_evidence::run(args.collect())?,
        Some("release-governance-check") => xtask::release_governance::run(args.collect())?,
        Some("tsan-check") => xtask::tsan_check::run(args.collect())?,
        Some("verify") => xtask::verify::run(args.collect())?,
        Some("verify-no-test-features") => xtask::feature_leak::run(args.collect())?,
        Some("--help") | Some("-h") | None => print_usage(),
        Some(command) => return Err(format!("unsupported xtask command: {command}").into()),
    }
    Ok(())
}

fn print_usage() {
    println!(
        "Usage:\n  \
         cargo xtask borrowed-suite-check --suite hazelcast  # execute the 0.69 adapted Hazelcast expectation manifest\n  \
         cargo xtask verify        # run the fast release gates (see docs/GATES.md)\n  \
         cargo xtask verify-no-test-features  # ensure test-only features/deps are absent from release graphs\n  \
         cargo xtask canary-check  # validate the 0.64 Raft canary registry\n  \
         cargo xtask canary-sweep --release 0.64 --tier <fast|all>  # execute expected-red canary proofs\n  \
         cargo xtask client-plane-spike-check  # run 0.68 Rust/Java SDK/Python client-plane evidence\n  \
         cargo xtask client-plane-bakeoff-check  # validate the accepted HC/2 transport decision and evidence manifest\n  \
         cargo xtask client-plane-ci-check  # validate the H22 Linux/Docker/fuzz/soak workflow contract\n  \
         cargo xtask client-plane-ci-receipt --lane <lane> --output <path> [--seed <u64>] [--iterations <n>] [--image <digest>]  # retain one H22 lane result\n  \
         cargo xtask client-plane-ci-admission --receipts <dir> [--scope <hosted|full>] [--commit <sha>] [--output <path>]  # require the selected exact-commit HC/2 evidence scope\n  \
         cargo xtask client-plane-docker-interop-check  # run the bounded H22 process-interoperability subset in the pinned container\n  \
         cargo xtask client-plane-java-sdk-check  # build/test/install Java SDK and external consumer\n  \
         cargo xtask client-plane-compat-check [--manifest-only|--require-complete]  # verify retained HC/2 artifacts and compatibility matrix\n  \
         cargo xtask client-plane-fault-check [--replay <path>|[--case <id>] --seed <u64> --output <path>]  # verify or generate deterministic HC/2 fault traces\n  \
         cargo xtask client-plane-generation-check  # compare two clean Rust and Java HC/2 generations byte-for-byte\n  \
         cargo xtask client-plane-python-check  # verify generated Python and test it from the offline wheelhouse\n  \
         cargo xtask client-plane-python-generate --write  # regenerate checked-in Python messages/stubs/metadata\n  \
         cargo xtask client-plane-rust-sdk-check  # prove the native HC/2 Rust SDK and unchanged HC/1 client\n  \
         cargo xtask client-conformance --all-sdks  # validate and execute the shared Rust/Java/Python HC/2 semantics\n  \
         cargo xtask client-package-check  # freeze and consume Rust/Java/Python HC/2 packages\n  \
         cargo xtask client-schema-check  # prove generation-6 schema/API metadata and deterministic SDK generation\n  \
         cargo xtask ci-admission-status --release <release> --source <sha> --head <sha> --base <sha> [--require <lane=result>|--lane-status <lane=path>] --output <path>  # retain and validate fail-loud CI admission status\n  \
         cargo xtask ci-topology-check --release 0.71  # validate workflow topology, timeouts, artifacts, and publication ownership\n  \
         cargo xtask compat-check [--preflight-only|--manifest-only]  # validate previous-release compatibility\n  \
         cargo xtask coverage-ratchet-check [--structural|--run]  # validate or execute the pinned coverage floor\n  \
         cargo xtask determinism-sweep --release 0.64  # compare canonical logical evidence across repeated/serial runs\n  \
         cargo xtask doc-check     # validate docs/plans/releases.toml (RULES R-11)\n  \
         cargo xtask durable-inspect <store-dir>  # dump verified durable value records as JSON\n  \
         cargo xtask evidence-run --release 0.64 --gate <id>  # execute a registered gate and write a receipt\n  \
         cargo xtask fast-suite-check --release 0.64  # validate fast-suite budgets and receipts\n  \
         cargo xtask gated-test-check  # validate every ignored/cfg/env-gated test registration\n  \
         cargo xtask legacy-client-check --matrix hc1  # build shipped HC/1 libraries into consumers and run them against the current server\n  \
         cargo xtask miri-check  # run pinned Miri-safe snapshot proofs (skip loud when unavailable)\n  \
         cargo xtask migration-conformance-check <--structural|--upstream>  # validate 0.69 manifests or resolve selectors at pinned upstream commits\n  \
         cargo xtask memory-owner-inventory --release 0.71  # generate conservative source ownership candidates\n  \
         cargo xtask memory-ownership-check --release 0.71  # require reviewed closure for every ownership candidate\n  \
         cargo xtask memory-decision-check --release 0.71  # validate immutable D0-D4 proposal transitions\n  \
         cargo xtask memory-statistics-check --release 0.71  # validate the preregistered numerical decision contract\n  \
         cargo xtask allocator-capability-check --release 0.71  # validate allocator capability and portability claims\n  \
         cargo xtask perf-memory-preflight --release 0.71 --profile memory-reference-071-v1  # fingerprint or block the numerical evidence host\n  \
         cargo xtask memory-compat-check --release 0.71  # validate the 0.70/0.71 runtime and durable compatibility matrix\n  \
         cargo xtask memory-release-policy-check --release 0.71  # validate mandatory/no-win/deferred release dispositions\n  \
         cargo xtask memory-baseline-check --release 0.71 [--require-d0]  # validate B0/B1, scenario, archive, and D0 admission\n  \
         cargo xtask memory-baseline-status --release 0.71 --output <path>  # emit a non-promotable local baseline status\n  \
         cargo xtask postgres-conformance-check --mode <happy|canary>  # execute the real PostgreSQL differential or expected-red sentinel\n  \
         cargo xtask mutants       # validate the Raft mutation-testing baseline, optionally run cargo-mutants\n  \
         cargo xtask perf-runner-preflight --release 0.67 --profile reference-v1  # reject an unstable reference runner before build/measurement\n  \
         cargo xtask perf-prebuild --release 0.67 --profile reference-v1  # build and bind exact performance binaries\n  \
         cargo xtask perf-bootstrap --release 0.67.1 --profile reference-v1 --phase <context|authorize|sample|sample-set>  # retain an admitted, chained non-ship bootstrap sample\n  \
         cargo xtask perf-full-dress --release 0.67.1 --profile reference-v1 --phase <context|receipt|admission>  # prove the complete workload twice before bootstrap\n  \
         cargo xtask perf-qualification --release 0.67.1 --profile reference-v1 --phase <context|finalize>  # validate a non-promotable dedicated host\n  \
         cargo xtask perf-reference --release 0.67.1 --profile reference-v1 --phase <propose|review|reviewed|activate|frozen-candidate>  # derive, review, activate, and prove the reference contract\n  \
         cargo xtask perf-budget-check --release <0.67|0.67.1> --profile <reference-v1|ci-shared>  # validate receipt-bound macro budgets\n  \
         cargo xtask quarantine-check --release 0.64  # validate temporary test quarantines\n  \
         cargo xtask raft-spec-check --structural|--scope <fast|canary|nightly>  # validate/run the pinned TLA+ model\n  \
         cargo xtask release-evidence --release 0.64  # derive the per-W release evidence matrix\n  \
         cargo xtask release-governance-check --release 0.64  # run structural release meta-gates\n  \
         cargo xtask tsan-check --scope <suites|canary>  # run pinned Linux ThreadSanitizer proof\n  \
         cargo xtask bench-budget [--budget benches/budget.toml] [--baseline benches/baseline/0_37.json] [--current target/criterion]\n\n\
         (The `cargo xtask` alias is defined in .cargo/config.toml; `cargo run -p xtask -- <cmd>` also works.)"
    );
}
