"""Prepare Windows/package and workspace contracts, never dispatch either gate."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib

from storage import now, write

ROOT = Path(__file__).resolve().parents[2]
FLOOR = 4 * 1024**3
PINS = {
    'tools/package-frontier-gate-074/review.py': '97f17d0b0fe7453197eb3e61770db95ea9e055fe3d0e260c091043701c101c4c',
    'tools/package-frontier-gate-074/inputs.json': 'f4931bed92a446c07171c40baa566ac8c66f08374487ef931ad3276855391656',
    'crates/xtask/src/verify.rs': '8275b2093464899319bb5de2246ec8893e872ebf67e91d1bb4e2d65ca3a41918',
    'target/run-xtask-package-780-20261024.py': '52504f2b489d16f344b477df0568f33ac224c5cadacc552f32f427f39c09d04d',
    'target/run-source-step-20261024.py': 'a92482cabcfcf8f6440e95972bee8d204bb9c59c8805c45c7b57363dcc3ab430',
    'target/run-full-verify-bounded-20261024.py': 'ddc6a8ead0e70d3cacd8a247c9d2cc7a96d5348f9ab5242b3bfd8fd6f4e1854b',
}
PACKET = 'docs/testing/performance/0.74/local-runs/frontier-gnu-package-20261027'
INDEX_SHA = 'bdadfcc8c8c59f9bd49c79608307f76fe7cd0accee453fcb456aeeb818da0ef9'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def checked(root, name, digest):
    path = root / name
    require(path.resolve().is_relative_to(root.resolve()) and not path.is_symlink()
            and not path.is_junction(), 'review path custody: ' + name)
    raw = path.read_bytes()
    require(sha(raw) == digest, 'fixed input drift: ' + name)
    return raw


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def contract():
    return dict(
        package=dict(argv=['cargo', '+1.94.0', 'test', '--offline', '--locked',
            '--no-fail-fast', '-p', 'xtask', '--lib', '--tests', '--', '--test-threads=4'],
            bound_seconds=780, includes_compilation=True, test_threads=4,
            physical_floor_bytes=FLOOR, target_catalog_count=156,
            filter=None, retry=False, old_v9_result='red-incomplete-immutable',
            all_default_targets_required=True, concatenate_prior_results=False),
        workspace=dict(argv=['cargo', 'xtask', 'verify'],
            toolchain_precondition='actual default Rust/Cargo 1.94.0; do not silently switch',
            original_captured_outer_bound_seconds=4800,
            original_start_free_bytes=16 * 1024**3, original_running_floor_bytes=FLOOR,
            original_capacity_poll_seconds=5, original_retry=False,
            expired_20261024_schedule_reused=False,
            implementation='crates/xtask/src/verify.rs',
            implementation_raw_sha256=PINS['crates/xtask/src/verify.rs'],
            windows_tests=[['cargo', 'test', '--workspace', '--exclude', 'xtask', '--locked', '-j', '1'],
                ['cargo', 'test', '-p', 'xtask', '--lib', '--tests', '--locked', '-j', '1']],
            isolated_target_for=['tests (xtask lib/integration)', 'performance budget contract'],
            isolated_target_pattern='target/xtask-verify-<verify-process-id>',
            format='original per-manifest Windows format gate',
            remaining_stages=['doc-check', 'management-center-check 0.72',
                'performance-contract-check 0.73', 'release feature leak',
                'complete original allocator/Clippy matrix', 'dependency bans', 'dependency policy',
                'DST fast budget', 'soak fast budget', 'raft failpoint crash-safety',
                'rustdoc -D warnings', 'performance budget contract', 'original management console gate'],
            console_skip='only original Node/npm absence rule; record actual outcome',
            package_780_is_not_a_whole_verify_budget=True,
            bounded_outer_execution_envelope_prepared=False,
            result=False),
        live_contract=dict(check_count_order_frequency='unchanged', origins_and_windows=[300, 60, 10]),
        dispatch_supported=False, release_admitted=False)


def capacity(free):
    require(type(free) is int and free >= 0, 'invalid free-space observation')
    return dict(observed_free_bytes=free, original_floor_bytes=FLOOR,
        original_floor_satisfied=free >= FLOOR, headroom_above_floor_bytes=max(0, free-FLOOR),
        original_workspace_start_bytes=16 * 1024**3,
        original_workspace_start_satisfied=free >= 16 * 1024**3,
        build_growth_reservation_established=False, pagefile_growth_reservation_established=False,
        launch_ready=False,
        reason='A sampled floor is not a build/pagefile reservation; no heavy gate dispatched.')


def workspace_members(root):
    manifest = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))
    rows = {}
    for member in manifest['workspace']['members']:
        require('*' not in member and '..' not in Path(member).parts, 'review wildcard member explicitly')
        name = member + '/Cargo.toml'
        path = root / name
        require(path.resolve().is_relative_to(root.resolve()) and not path.is_symlink()
                and not path.is_junction(), 'workspace member custody')
        raw = path.read_bytes()
        package = tomllib.loads(raw.decode())['package']
        rows[name] = dict(package=package['name'], raw_sha256=sha(raw))
    require(rows and len({r['package'] for r in rows.values()}) == len(rows), 'duplicate workspace package')
    return rows


def source_review(root):
    for name, digest in PINS.items():
        checked(root, name, digest)
    index = json.loads(checked(root, PACKET + '/index.json', INDEX_SHA))
    row = index['members']['attempt/syntax-current/result.json']
    raw = checked(root, PACKET + '/objects/' + row['sha256'], row['sha256'])
    require(len(raw) == row['bytes'], 'syntax member size')
    review = load(root / 'tools/package-frontier-gate-074/review.py', 'hc074_prepared_source_review')
    data = review.inputs()
    context = review.load(root / 'tools/historical-blob-batch-074/module_context_v5.py',
                          'hc074_prepared_context', review.CONTEXT_SHA)
    result = review.source_review(root, data, json.loads(raw), context)
    # This is a source-only closure; GNU execution success is not Windows admission.
    result.update(windows_execution_cleared=False, workspace_effect_review_complete=False)
    return result


def environment_review(root):
    # The full source review already checks all three imported legacy bodies.
    sys.path.insert(0, str(root / 'tools/historical-blob-batch-074'))
    import package_effects_review_v2
    return package_effects_review_v2.environment_preflight(root)


def command_capture(out, name, argv, root):
    with (out / (name + '.stdout.log')).open('xb') as stdout, \
            (out / (name + '.stderr.log')).open('xb') as stderr:
        result = subprocess.run(argv, cwd=root, stdout=stdout, stderr=stderr, timeout=10, check=False)
    write(out, name + '.json', dict(argv=argv, exit=result.returncode))
    require(result.returncode == 0, 'read-only probe refused: ' + name)
    return (out / (name + '.stdout.log')).read_bytes()


def prepare(out, root=ROOT):
    out.mkdir(parents=True, exist_ok=False)
    write(out, 'intent.json', dict(utc=now(), preparation_only=True,
        dispatch_supported=False, process_stops=False, settings_changes=False,
        source_sha256=sha(Path(__file__).read_bytes())))
    write(out, 'contract.json', contract())
    # Receipts exist even if the very first platform/pin/source preflight refuses.
    try:
        require(os.name == 'nt', 'Windows preparation only')
        head = command_capture(out, 'head-before', ['git', 'rev-parse', 'HEAD'], root).decode().strip()
        status = command_capture(out, 'source-status', ['git', 'status', '--porcelain',
            '--untracked-files=all', '--', 'Cargo.toml', 'Cargo.lock', 'deny.toml',
            '.cargo', 'crates', 'tools', 'scripts/ci', '.gitattributes'], root)
        write(out, 'source-review.json', source_review(root))
        write(out, 'environment-review.json', environment_review(root))
        write(out, 'workspace-members.json', workspace_members(root))
        write(out, 'capacity.json', capacity(shutil.disk_usage(root).free))
        write(out, 'tool-resolution.json', {name: shutil.which(name) for name in
            ('cargo', 'rustc', 'git', 'python', 'python3', 'node', 'npm', 'cargo-deny')})
        after = command_capture(out, 'head-after', ['git', 'rev-parse', 'HEAD'], root).decode().strip()
        require(head == after, 'HEAD changed during preparation')
        write(out, 'result.json', dict(utc=now(), head=head, contract_prepared=True,
            source_worktree_clean=not status, source_only_review_complete=True,
            windows_package_pass=False, workspace_pass=False, gate_dispatched=False,
            launch_ready=False, release_admitted=False,
            remaining=['fresh clean source/environment and owned Windows containment gate',
                'establish build and pagefile headroom without changing settings or foreign processes',
                'review owned whole-verify envelope at original captured 4800s, 16 GiB start, 4 GiB floor',
                'one distinct full Windows command, then complete workspace verify; retain first outcomes']))
    except BaseException as error:
        write(out, 'refusal.json', dict(type=type(error).__name__, reason=str(error),
            gate_dispatched=False, package_pass=False, workspace_pass=False))
        raise


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('out', type=Path)
    args = parser.parse_args()
    prepare(args.out)
    print(json.dumps(dict(prepared=True, gate_dispatched=False, launch_ready=False)))
