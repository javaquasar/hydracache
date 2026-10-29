"""Fixed-source manual loop proof joined to retained verdicts, not a profiler.

Only verified packet data is read. No Git, Cargo, input code, clocks or network
are executed. The ledger is deliberately not a general Rust control-flow parser.
"""
import hashlib
import json
from pathlib import Path
import sys
import tomllib

import review

PACKET = 'docs/testing/performance/0.74/local-runs/transport-package-20261025'
PACKET_SHA256 = 'e9b8a923723360bce644c9de558113bf084630d1c4a72916dd0bfd3d418fc553'
COMMIT = '3e9bcc06ebce3439495200162cb0c1900b8eda53'
SOURCE = 'crates/xtask/tests/diagnostic_artifact_fixture_074.rs'
SOURCE_SHA256 = '5819f89ec58a899e03d7c0f14c6af91f4be6248a26f8578186cfc47b5900a805'
TARGET = 'tests/diagnostic_artifact_fixture_074.rs'
PHASE = 'checks/transport-audit-20261025-package-first/'
TAIL = ('Cargo.lock', 'docs/COMPAT.md', 'docs/testing/gated-test-registry.toml',
        'docs/testing/performance/0.74/qualification-manifest.toml')
# Each reviewed body asserts this lines().count(), then unconditionally visits
# every line and tail item. Exactly one read and one git(show) occur in its
# assert_eq! operands; there is no break/return, filter or retry in that loop.
SWEEPS = (
    ('actual_consuming_fixture_preserves_all_446_preceding_sources_and_formats',
     'previous', 446, TAIL, 'held_live::normalize'),
    ('immutable_fixture_preserves_all_437_runtime_sources_and_production_inputs',
     'old', 437, (*TAIL, 'tools/long-run-supervisor-074/tests/owned_root_creator_sigkill.py'), 'namespace::normalize'),
    ('launcher_preserves_all_438_preceding_sources_and_exact_old_formats',
     'old', 438, TAIL, 'namespace::held_build::launcher::normalize'),
    ('role_files_preserve_all_440_previous_runtime_sources_and_inputs',
     'old', 440, TAIL, 'roles::normalize'),
    ('role_binding_preserves_all_441_preceding_runtime_sources_and_inputs',
     'old', 441, TAIL, 'binding::normalize'),
    ('held_trust_preserves_all_442_preceding_sources_and_production_inputs',
     'old', 442, TAIL, 'trust::normalize'),
    ('consuming_held_join_preserves_all_443_old_sources_and_formats',
     'old', 443, TAIL, 'consuming::normalize'),
)


def load_snapshot(root):
    folder = root / PACKET
    raw = (folder / 'index.json').read_bytes()
    review.require(review.sha(raw) == PACKET_SHA256, 'retained packet index changed')
    index = review.decode_json(raw)
    review.require(index['code_commit'] == COMMIT, 'packet commit differs')
    names = [PHASE + name for name in ('intent.json', 'result.json', 'stdout.log', 'stderr.log')]
    names += ['reviewed-source/' + SOURCE, 'reviewed-source/crates/xtask/Cargo.toml',
              'syntax/final-closure/result.json']
    values = {name: review.member(folder, index, name) for name in names}
    return values


def source_proof(raw, syntax):
    review.require(review.sha(raw) == SOURCE_SHA256, 'unreviewed source bytes')
    review.require(syntax['executed_input_code'] is False and syntax['compiler_expansion'] is False,
                   'not the retained syntax-only scope')
    matches = [row for row in syntax['sources'] if row['source'] == SOURCE]
    review.require(len(matches) == 1 and matches[0]['sha256'] == SOURCE_SHA256, 'syntax/source identity')
    sites = matches[0]['sites']
    review.require(sum(site == dict(scope='git', kind='command-constructor', detail='literal:git') for site in sites) == 1,
                   'direct Git constructor evidence')
    review.require(sum(site == dict(scope='git', kind='process-method-candidate', detail='output') for site in sites) == 1,
                   'synchronous output evidence')
    text = raw.decode('utf-8')
    rows = []
    for name, variable, count, tail, normalizer in SWEEPS:
        marker = f'fn {name}() {{'
        review.require(text.count(marker) == 1, 'reviewed body identity')
        start = text.index(marker)
        end = text.find('\n#[test]', start)
        body = text[start:] if end < 0 else text[start:end]
        review.require(f'assert_eq!({variable}.lines().count(), {count});' in body, 'asserted loop bound')
        review.require(f'for path in {variable}.lines().chain(' in body, 'reviewed iteration shape')
        review.require(f'{normalizer}(path, &read(path))' in body and 'git(&["show", &format!(' in body,
                       'reviewed per-iteration operands')
        review.require(all('"' + path + '"' in body for path in tail), 'reviewed tail inputs')
        review.require(dict(scope=name, kind='function-attribute', detail='test') in sites, 'real test item')
        review.require(dict(scope=name + '::macro:assert_eq', kind='function-call', detail=normalizer) in sites,
                       'normalizer syntax evidence')
        rows.append(dict(case=name, runtime_paths=count, tail_paths=list(tail),
            loop_iterations=count + len(tail), direct_show_calls_per_iteration=1,
            direct_read_calls_per_iteration=1, normalizer=normalizer,
            body_sha256=review.sha(body.encode()), source_line=text.count('\n', 0, start) + 1,
            proof='manually reviewed exact pinned body plus syntax inventory; success exhausts loop'))
    return rows


def analyze_snapshot(values):
    intent, result = (review.decode_json(values[PHASE + name]) for name in ('intent.json', 'result.json'))
    review.require(intent['head'] == COMMIT and intent['argv'] == review.ARGV, 'source/command identity')
    review.require(intent['retry'] is False and intent['bound_seconds'] == 780, 'retry or original bound differs')
    review.require(result['source_unchanged'] is True and result['head_unchanged'] is True,
                   'source changed during retained run')
    review.require(intent['source'] == result['source_after'], 'before/after source inventory differs')
    review.require(result['admission'] is False and result['qualification'] is False, 'unexpected admission')
    source = values['reviewed-source/' + SOURCE]
    review.require(intent['source'][SOURCE] == SOURCE_SHA256, 'run/source identity')
    manifest = values['reviewed-source/crates/xtask/Cargo.toml']
    review.require(review.sha(manifest) == intent['source']['crates/xtask/Cargo.toml'], 'captured manifest differs')
    catalog = review.catalog(intent['source'], tomllib.loads(manifest.decode()))
    report = review.parse_transcript(values[PHASE + 'stdout.log'].decode(), values[PHASE + 'stderr.log'].decode(),
        catalog, result['exit'], result['outer_refusal'], intent['bound_seconds'])
    targets = [row for row in report['targets'] if row['target'] == TARGET]
    review.require(len(targets) == 1, 'selected target not scheduled exactly once')
    target = targets[0]
    review.require(target['reported_ns'] is not None and target['failed'] == target['ignored'] == 0,
                   'selected target incomplete or not wholly successful')
    rows = source_proof(source, review.decode_json(values['syntax/final-closure/result.json']))
    passed = {row['name'] for row in target['cases'] if row['verdict'] == 'ok'}
    review.require(all(row['case'] in passed for row in rows), 'reviewed sweep lacks passing verdict')
    count = sum(row['loop_iterations'] for row in rows)
    return dict(schema='hc074-retained-preservation-sweeps-v1', source_commit=COMMIT,
        packet_sha256=PACKET_SHA256, source=SOURCE, source_sha256=SOURCE_SHA256,
        target=TARGET, target_reported_ns=target['reported_ns'], target_cases=target['declared'],
        sweeps=rows, proven_direct_show_launches_lower_bound=count,
        proven_direct_reads_lower_bound=count, excludes_other_cases_helpers_and_inventory_work=True,
        proof_kind='source/verdict lower bound, not dynamic process telemetry or general control-flow analysis',
        input_sha256={name: review.sha(raw) for name, raw in sorted(values.items())},
        package_complete=report['complete'], package_pass=report['package_pass'],
        completed_targets=report['completed_targets'], expected_targets=report['expected_targets'],
        unstarted_targets=len(report['unstarted_targets']),
        incomplete_target=None if report['incomplete_target'] is None else report['incomplete_target']['target'],
        package_timeout_explained=False, elapsed_time_attributed=False, per_case_timing=False,
        multiplied_by_other_cohort_timing=False, cache_or_gate_change=False,
        workload_rerun=False, subprocesses_executed=False, server_or_release_admission=False)


def analyze(root):
    return analyze_snapshot(load_snapshot(root))


def main():
    try:
        result = analyze(Path(__file__).resolve().parents[2])
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f'Offline preservation-sweep review refused: {error}', file=sys.stderr)
        return 2
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
