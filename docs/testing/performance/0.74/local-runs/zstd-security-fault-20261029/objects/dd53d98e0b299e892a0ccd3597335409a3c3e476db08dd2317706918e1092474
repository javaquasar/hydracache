"""Fixed V9 log/source attribution as data, without Git or test execution.

Static loop contracts identify logical work, not measured calls or CPU cost.
The frozen AST inventory is source-only, not compiler expansion.
"""
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
PACKET = 'docs/testing/performance/0.74/local-runs/package-closure-v9-20261027'
INDEX = 'eea96bfe5da0766983b1bbbceff0ee55b46ec67fd49cfe049e465c8fbedc8161'
SOURCE_PACKET = 'docs/testing/performance/0.74/local-runs/package-effect-syntax-v9-20261027'
SOURCE_INDEX = 'e2c54681e571204fc927d7df3fe99a08303474c52af7a945d1a2764bb601bfc9'
ACCOUNTING = '49709b4cb75f127625bd4d77328127ccbc16c035a20dd126494c51089cf167b4'
GATE = 'edacd9736d494ec6b11335c93320676510bd09d99a8bb367b2feae66b6d80d4d'
AST = '556959c6a9dc136f3ef2f6b61f0b8c52b2e6f5d9015327c872b9ef6af4105ad7'
PREFIX = 'crates/xtask/tests/'
# Manually reviewed loop contracts in whole pinned source bodies. These are
# selected paths, never a discovered all-loop count or a performance estimate.
LOOPS = [
    ('diagnostic_artifact_fixture_074', 'actual_consuming_fixture_preserves_all_446_preceding_sources_and_formats', 446, 4),
    ('diagnostic_artifact_fixture_074', 'immutable_fixture_preserves_all_437_runtime_sources_and_production_inputs', 437, 5),
    ('diagnostic_artifact_fixture_074', 'held_trust_preserves_all_442_preceding_sources_and_production_inputs', 442, 4),
    ('diagnostic_artifact_fixture_074', 'consuming_held_join_preserves_all_443_old_sources_and_formats', 443, 4),
    ('custody_preparation_cost_074', 'custody_observation_hooks_reconstruct_all_preceding_469_bodies', 469, 4),
    ('document_scratch_cost_074', 'document_scratch_adoption_is_the_frozen_helper_and_reconstructs_all_473_old_bodies', 473, 0),
    ('held_preparation_cost_074', 'held_preparation_and_prefix_reconstruct_all_old_runtime_and_fixture_assertions', 462, 4),
]
GIT_READS = {'git', 'old', 'read', 'runtime_io::read_to_string', 'history::read_to_string',
             'history::command', 'std::process::Command::new', 'std::fs::read_to_string'}


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    def constant(_):
        raise ValueError('nonfinite JSON')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=constant)


def digest(value):
    return sha(json.dumps(value, sort_keys=True, separators=(',', ':')).encode())


def packet(root, name, expected):
    raw = (root / name / 'index.json').read_bytes()
    require(sha(raw) == expected, 'fixed packet index drift')
    return decode(raw)


def member(root, name, index, key, expected=None):
    row = index['members'][key]
    value = row['sha256']
    require(isinstance(value, str) and re.fullmatch('[0-9a-f]{64}', value), 'unsafe object identity')
    require(expected is None or value == expected, 'fixed member pointer drift')
    raw = (root / name / 'objects' / value).read_bytes()
    require(sha(raw) == value and len(raw) == row['bytes'], 'fixed object drift')
    return raw


def inputs(root=ROOT):
    receipt = packet(root, PACKET, INDEX)
    source = packet(root, SOURCE_PACKET, SOURCE_INDEX)
    accounting = decode(member(root, PACKET, receipt, 'derived/accounting.json', ACCOUNTING))
    gate = decode(member(root, PACKET, receipt, 'coordinator/full-first/gate.json', GATE))
    document = decode(member(root, SOURCE_PACKET, source, 'capture/v9-syntax-first/result.json'))
    texts = {name: member(root, SOURCE_PACKET, source, 'fixture/' + name, expected)
             for name, expected in gate['sources'].items()}
    return accounting, gate, document, texts


def analyze(accounting, gate, document, texts):
    require(sha((json.dumps(accounting, indent=2, sort_keys=True) + '\n').encode()) == ACCOUNTING,
            'first accounting drift')
    require(digest(document) == AST and not document['executed_input_code']
            and not document['compiler_expansion'], 'fixed source/AST drift')
    rows = {r['source']: r for r in document['sources']}
    require(len(rows) == len(document['sources']) == 386 and len(gate['auxiliary']) == 56,
            'source census drift')
    require({name: row['sha256'] for name, row in rows.items()} == gate['sources'], 'source/AST association drift')
    require(set(texts) == set(rows), 'source coverage drift')
    for name, raw in texts.items():
        require(sha(raw) == rows[name]['sha256'], 'reviewed source drift: ' + name)
    require(len(gate['targets']) == 153 and accounting['expected_targets'] == 153
            and accounting['completed_targets'] == 80 and accounting['passed'] == 488
            and accounting['failed'] == 0 and not accounting['package_pass']
            and not accounting['complete'], 'first incomplete result cannot change')
    targets = {r['target']: r for r in accounting['targets']}
    selected = []
    for target, case, paths, extra in LOOPS:
        source = PREFIX + target + '.rs'
        sites = rows[source]['sites']
        require(sum(s['kind'] == 'function-attribute' and s['scope'] == case
                    and s['detail'] == 'test' for s in sites) == 1, 'selected test identity drift')
        body = texts[source].decode('utf-8').replace('\r\n', '\n')
        require(f'lines().count(), {paths}' in body, 'reviewed loop cardinality witness absent')
        calls = [s for s in sites if s['kind'] == 'function-call'
                 and (s['scope'] == case or s['scope'].startswith(case + '::'))]
        require(any(s['detail'] == 'read' for s in calls) and any(s['detail'] in ('git', 'old') for s in calls),
                'read/old-output comparison absent')
        actual = targets['tests/' + target + '.rs']
        verdict = next((r['verdict'] for r in actual['cases'] if r['name'] == case), None)
        selected.append(dict(target=actual['target'], case=case, verdict=verdict,
            target_complete=actual['reported_seconds'] is not None, historical_paths=paths,
            explicit_extra_paths=extra, static_loop_members=paths + extra,
            logical_operations='one retained read helper, predecessor projection and old-output comparison per member; nested helper work is additional',
            syntax_calls=calls, source=source, source_sha256=rows[source]['sha256'],
            per_case_elapsed_measured=False, physical_io_or_process_count_measured=False))
    witnesses = {}
    for name in [PREFIX + t + '.rs' for t in ('authority_rust_history_074', 'current_tree_input_audit_074')]:
        witnesses[name] = dict(sha256=rows[name]['sha256'],
            source_sites=[s for s in rows[name]['sites'] if s['kind'] == 'function-call' and s['detail'] in GIT_READS])
    return dict(schema='hc074-package-v9-offline-attribution-v1', source_commit='0c60d80d097b15f7d4bf59203d4455eb8d537441',
        packet_index_sha256=INDEX, source_index_sha256=SOURCE_INDEX,
        completed_targets=80, complete_summary_passes=488, printed_passes=491,
        incomplete_target=accounting['incomplete_target'], unstarted_targets=accounting['unstarted_targets'],
        reported_build_span_ns=accounting['reported_build_span_ns'],
        completed_target_spans_ns=accounting['reported_completed_target_spans_ns'],
        ranking=accounting['ranking'], selected_loop_contracts=selected, frontier_witnesses=witnesses,
        attribution_level='printed target spans plus exact source-path contracts; not measured leaf causality',
        measured_cpu_attribution=False, measured_git_or_filesystem_attribution=False,
        summed_case_times=False, product_performance_claim=False, new_hypothesis=False,
        input_code_executed=False, tests_dispatched=False, git_dispatched=False,
        windows_package_pass=False, release_admitted=False)


if __name__ == '__main__':
    print(json.dumps(analyze(*inputs()), indent=2, sort_keys=True))
