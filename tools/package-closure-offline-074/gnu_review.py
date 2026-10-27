"""Separate default-GNU source/effect/environment review, without a dispatcher.

Fixed V9 source data covers cfg-disabled branches too. This manual entry-point
review is not compiler expansion, full-test success or a production capability.
"""
import importlib.util
import json
from pathlib import Path
import shutil

import analysis as a

FRONTIER_SHA = '8cb4397761714aee264ba52c17f8d7d2579358a8995d8f7ea5ba5e4c9a0eb631'
CONTEXT_SHA = 'c4bbc2d6b69e416ac3fcb30a1a64fb3bb54473b70977a1875ff90ca264b0cd44'
ROOT = Path('/home/artur/.cache/hydracache-074-frontier-native/636feb166b273b5c3d6fc0b93c4fccf57560cf03/repo')
CONTROL = Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
TARGET = Path('/home/artur/.cache/hydracache-074-194-check')

# Complete fixed source bodies, not token discovery -> automatic approval.
# Each group records the separately reviewed Linux/default entry-point boundary.
GROUPS = {
    'unix_metadata': dict(paths=['crates/xtask/src/host_attestation.rs', 'crates/xtask/src/long_run_campaign.rs',
        'crates/xtask/src/perf.rs', 'crates/xtask/tests/support/artifact_inventory_input.rs',
        'crates/xtask/tests/support/current_tree_input.rs', 'tools/current-frontier-074/additions.rs',
        'tools/current-frontier-074/alarm_addition.rs', 'crates/xtask/tests/frontier_current_view_074.rs'],
        review='Unix UID/mode/nlink/directory-fsync and owned tempfile symlink controls. Windows reparse branches are absent, never substituted for Unix symlink/regular-file checks. No fixed-host directory installation or enrollment.'),
    'evidence_children': dict(paths=['crates/xtask/src/evidence_run.rs', 'crates/xtask/tests/evidence_run.rs',
        'crates/xtask/src/canary_sweep.rs', 'crates/xtask/tests/canary_check.rs'],
        review='Owned temporary synthetic Git repositories and exact self-executable child argv. Unix process_group(0), wait/capture and timeout kill apply only to these fixtures. Windows descendant tests are cfg-excluded, not declared covered on Linux. The full outer CI subreaper remains separately required. No systemd, SSH, product runner or generic registered real campaign is dispatched.'),
    'ci_containment': dict(paths=['crates/xtask/tests/ci_reliability_071.rs', 'crates/xtask/tests/cancellation_safety.rs'],
        review='Pinned Python heartbeat runs bounded synthetic success/failure/timeout/output/descendant children in owned scratch. Cancellation controls are deterministic models. These are CI containment checks, not original all-writer retirement.'),
    'supervisor_constants': dict(paths=['crates/xtask/tests/performance_contract_074.rs'],
        review='Four Linux blocks import local_files, local_context, intent_projection and intent_journal constants only. They do not invoke account/context enrollment, publish journal records, launch a manager or touch a host fence. Remaining privileged/root fixture tokens are source strings, not #[path] execution edges.'),
    'python_transport': dict(paths=['crates/xtask/tests/historical_blob_caller_074.rs',
        'crates/xtask/tests/historical_blob_fault_074.rs', 'crates/xtask/tests/historical_blob_transport_074.rs',
        'crates/xtask/tests/perf_exploratory_harness.rs', 'crates/xtask/tests/perf_ci_instruction.rs',
        'crates/xtask/tests/census_cost_074.rs'],
        review='Linux selects python3 for the already fixed oracle/collision/caller/report scripts and self-only children. Git config/ref changes are confined to owned synthetic repositories. Deliberate trace/config refusals do not weaken caller pins or panic API. No attribution child opt-in is inherited.'),
    'default_off': dict(paths=['crates/xtask/tests/direct_output_cost_074.rs',
        'crates/xtask/tests/git_descendant_cpu_074.rs', 'crates/xtask/tests/package_diagnostics_074.rs',
        'tools/normalization-attribution-074/history_probe.rs', 'tools/normalization-attribution-074/probe.rs',
        'tools/xtask-package-attribution-074/pipe_capture.rs'],
        review='Windows-only CPU/pipe implementations are not compiled for GNU. Non-Windows diagnostic panic routes require explicit HC074 opt-ins or exact --exact/.../--test-threads=1 argv; the default full four-thread argv and absence of inherited HC074/HYDRACACHE/RUST_TEST overrides keep them off. No diagnostic is counted as an executed benchmark.'),
    'generation': dict(paths=['crates/xtask/src/client_plane_python.rs', 'crates/xtask/src/client_plane_generation.rs',
        'crates/xtask/src/client_plane_compat.rs', 'crates/xtask/src/client_plane_rust.rs',
        'crates/xtask/src/client_plane_java.rs', 'crates/xtask/src/client_plane_spike.rs',
        'crates/xtask/src/client_package.rs', 'crates/xtask/src/migration_conformance.rs'],
        review='Unit paths validate fixed files/models and vendored-protoc Python generation into the owned target/hc2-python-generation scratch. Real Maven/venv/wheel installation, clean Rust generation, daemon extraction/listeners and compatibility CLI drivers are not called by unit roots. Locked build scripts write OUT_DIR only; offline build uses the existing owned GNU target.'),
    'models_not_measurements': dict(paths=['crates/xtask/src/memory_contracts.rs', 'crates/xtask/src/performance_local.rs',
        'crates/xtask/src/coverage_ratchet.rs', 'crates/xtask/src/verify.rs',
        'crates/xtask/tests/coverage_ratchet.rs'],
        review='Platform/model/receipt/plan assertions do not invoke real host-profile, measurement, coverage or verify CLI entry points. Linux eligibility does not grant a lease or launch product load. No full verify, qualification or thresholds are selected by this review.'),
}


def command():
    # The only context option relative to V9 Windows is the existing GNU build
    # target directory. No libtest filter, feature, ignore or retry option exists.
    return ['/home/artur/.cargo/bin/cargo', '+1.94.0', 'test', '--offline', '--locked', '--no-fail-fast',
        '--target-dir', str(TARGET), '-p', 'xtask', '--lib', '--tests', '--', '--test-threads=4']


def source_review(gate, document, texts, module_additions, unresolved):
    # Reuse only pure data assessment, never a default-Windows environment gate.
    a.require(a.digest(document) == a.AST and len(gate['sources']) == 386, 'GNU fixed source inventory drift')
    a.require(set(texts) == set(gate['sources']) and
              all(a.sha(texts[name]) == value for name, value in gate['sources'].items()), 'GNU source body drift')
    a.require(not module_additions and not unresolved, 'GNU module closure unresolved')
    a.require(len(gate['targets']) == len(set(gate['targets'])) == 153, 'GNU target census drift')
    for group in GROUPS.values():
        a.require(set(group['paths']) <= set(texts), 'GNU manual entry-point body missing')
    # Existing V9 census traverses cfg branches without evaluating them.
    constructors = gate['constructors']
    a.require(not any(s['detail'] in {'literal:ssh', 'literal:systemctl', 'literal:systemd-run',
        'literal:wsl', 'literal:timedatectl'} for s in constructors), 'GNU host/service constructor')
    return dict(schema='hc074-default-gnu-package-source-review-v1', source_review_scope='default GNU --lib --tests, no --ignored or all-features',
        fixed_source_data_sha256=a.AST, sources=gate['sources'], auxiliary=gate['auxiliary'], targets=gate['targets'],
        manual_groups=GROUPS, constructor_sites=len(constructors), module_closure_complete=True,
        source_effect_review_complete=True, compiler_expansion=False, formal_whole_program_proof=False,
        planned_argv=command(), package_bound_seconds=780, test_threads=4,
        build_context_delta='native cwd/independent Git database, LF source representation and explicit existing GNU target-dir; no suite/filter/deadline change',
        windows_v9_reinterpreted=False, tests_dispatched=False, package_pass=False, release_admitted=False)


def checked_module(path, name, expected):
    a.require(a.sha(path.read_bytes()) == expected, 'current review helper drift')
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def review():
    frontier = checked_module(CONTROL / 'tools/native-frontier-074/gnu_frontier.py',
        'checked_current_native_frontier_review', FRONTIER_SHA)
    native, request, inherited = frontier.preflight()
    accounting, gate, document, texts = a.inputs(CONTROL)
    # The immutable first receipt is independently assessed without rerunning it.
    a.analyze(accounting, gate, document, texts)
    context = checked_module(CONTROL / 'tools/historical-blob-batch-074/module_context_v5.py',
        'checked_current_native_module_context', CONTEXT_SHA)
    additions, unresolved = context.module_edges(ROOT, document)
    actual_targets = ['unittests src/lib.rs', 'unittests src/main.rs'] + sorted(
        'tests/' + p.name for p in (ROOT / 'crates/xtask/tests').glob('*.rs'))
    a.require(actual_targets == gate['targets'], 'actual native GNU default targets differ')
    for name in ('python3', 'git', 'kill'):
        a.require(shutil.which(name) == '/usr/bin/' + name, 'unreviewed GNU fixture executable: ' + name)
    result = source_review(gate, document, texts, additions, unresolved)
    result['native_preflight'] = inherited
    result['environment_preflight_complete'] = True
    result['physical_free_bytes'] = shutil.disk_usage('/mnt/c').free
    result['native_free_bytes'] = shutil.disk_usage(ROOT).free
    result['capacity_is_not_build_admission'] = True
    result['full_dispatcher_implemented'] = False
    # Preflight already observes the complete file/ref census; no effects follow.
    a.require(native.git(ROOT, 'rev-parse', 'HEAD').decode().strip() == request['source'], 'native source changed during review')
    return result


if __name__ == '__main__':
    print(json.dumps(review(), indent=2, sort_keys=True))
