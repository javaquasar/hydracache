"""Offline first-outcome join; no retained implementation or workload execution."""
import hashlib
import importlib.util
import json
from pathlib import Path

INDEX = 'd194f84a6258a060156271d6704af3f38f3f5d7590e0f9f8ad672b531abacbd7'
SOURCE = 'fa9e708868b22999d60c9ccd97b694662910a095'
PARSER = '0229d4067fffd0d0b5acee7d864efc64aadfb201de1ea3f17944a2f8dddf3c4d'
ROOT = Path(__file__).resolve().parents[6]
ARGV = ['cargo', '+1.94.0', 'test', '--offline', '--locked', '--no-fail-fast',
        '-p', 'xtask', '--lib', '--tests', '--', '--test-threads=4']
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def verify(folder, root=ROOT):
    raw = (folder / 'index.json').read_bytes()
    require(sha(raw) == INDEX, 'packet index differs')
    index = json.loads(raw)
    require(index['source_commit'] == SOURCE and all(index[key] is False for key in
            ('package_pass', 'qualification', 'server', 'automatic_retry')), 'packet scope differs')
    members = {}
    for name, row in index['members'].items():
        body = (folder / 'objects' / row['sha256']).read_bytes()
        require(len(body) == row['bytes'] and sha(body) == row['sha256'], 'raw member differs: ' + name)
        members[name] = body
    require(len(members) == 156, 'member count differs')
    gate = json.loads(members['coordinator/first/gate.json'])
    pre = json.loads(members['coordinator/preflight-first/gate.json'])
    post = json.loads(members['checks/package-v6-postflight-first/stdout.log'])
    require(gate == pre and {k: v for k, v in gate.items() if k != 'head'} == post, 'pre/post review differs')
    require(gate['head'] == SOURCE and len(gate['sources']) == 368 and len(gate['auxiliary']) == 45
            and len(gate['targets']) == 145 and gate['source_effect_review_complete'] is True,
            'review scope differs')
    require(gate['reviewed_ast_sha256'] == '2552f3a6d96786433ea27b111a3f9f3b7274e82446634d68ca7d49090e24aa54'
            and all(gate[key] is False for key in ('formal_whole_program_proof', 'package_pass',
                'package_dispatched', 'linux_or_all_features_cleared',
                'original_execution_or_product_or_qualification_admission')), 'review admission differs')
    labels = sorted(name.split('/')[1] for name in members
                    if name.startswith('checks/') and name.endswith('/result.json'))
    require(len(labels) == 22, 'check catalog differs')
    bad = {'package-v6-controls-first', 'package-v6-dispatch-controls-first',
           'package-v6-cost-evidence-first', 'package-v6-contract-check-first', 'package-v6-first'}
    for label in labels:
        prefix = 'checks/' + label + '/'
        intent = json.loads(members[prefix + 'intent.json'])
        result = json.loads(members[prefix + 'result.json'])
        full, changed_head = label == 'package-v6-first', label == 'package-v6-doc-check-first'
        require(result['exit'] == (1 if label in bad else 0) and
                result['outer_refusal'] == ('outer-timeout' if full else None), 'first check result differs')
        require(result['source_unchanged'] is True and result['head_unchanged'] is (not changed_head) and
                result['source_after'] == intent['source'], 'check source/head receipt differs')
        require(intent['bound_seconds'] == (780 if full else 1200) and
                intent['disk_floor_bytes'] == 4 * 1024**3 and
                all(intent[key] is False for key in ('dependency_changes', 'observation_frequency_changed',
                    'services_or_clocks', 'server', 'original_owner', 'qualification', 'retry')), 'check scope differs')
    require('Ran 14 tests' in members['checks/package-v6-controls-final/stderr.log'].decode() and
            members['checks/package-v6-controls-final/stderr.log'].decode().rstrip().endswith('OK'), 'final cohort differs')
    require('FAILED (failures=1)' in members['checks/package-v6-controls-first/stderr.log'].decode() and
            'FAILED (errors=1)' in members['checks/package-v6-dispatch-controls-first/stderr.log'].decode() and
            "KeyError: 'reported_ns'" in members['checks/package-v6-cost-evidence-first/stderr.log'].decode() and
            'unsupported performance contract argument: --manifest' in
                members['checks/package-v6-contract-check-first/stderr.log'].decode(), 'first refusals differ')
    for suffix, label in (('test_package_replay_v6.py', 'package-v6-dispatch-controls-first'),
                          ('test_package_effects_review_v6.py', 'package-v6-controls-first')):
        name = 'tools/historical-blob-batch-074/' + suffix
        intent = json.loads(members['checks/' + label + '/intent.json'])
        require(sha(members['reconstructed-first-source/' + name]) == intent['source'][name],
                'inverse-edit reconstruction differs from original hash')
    first = json.loads(members['checks/package-v6-first/intent.json'])
    result = json.loads(members['checks/package-v6-first/result.json'])
    require(first['head'] == SOURCE and first['argv'] == ARGV and
            all(first['source'][name] == digest for name, digest in gate['sources'].items()), 'full command/source differs')
    for label in ('preflight-first', 'first'):
        intent = json.loads(members['coordinator/' + label + '/intent.json'])
        require(intent['argv'] == ARGV and intent['execute'] is (label == 'first') and
                intent['package_bound_seconds'] == 780 and intent['case_threads'] == 4 and
                all(intent[key] is False for key in ('retry', 'observer', 'services', 'clocks', 'server', 'qualification')),
                'coordinator scope differs')
    wrappers = {'target/run-xtask-package-780-20261024.py': '52504f2b489d16f344b477df0568f33ac224c5cadacc552f32f427f39c09d04d',
                'target/run-source-step-20261024.py': 'a92482cabcfcf8f6440e95972bee8d204bb9c59c8805c45c7b57363dcc3ab430'}
    require(json.loads(members['coordinator/first/coordinator.json'])['wrappers'] == wrappers, 'wrapper receipt differs')
    for name, digest in wrappers.items():
        require(sha(members['wrapper/' + Path(name).name]) == digest, 'original wrapper differs')
    for name, digest in gate['auxiliary'].items():
        require(sha(members['auxiliary/' + name]) == digest, 'auxiliary differs')
    for name in ('package_effects_review_v6.py', 'package_replay_v6.py',
                 'test_package_effects_review_v6.py', 'test_package_replay_v6.py'):
        name = 'tools/historical-blob-batch-074/' + name
        require(sha(members['implementation/' + name]) == first['source'][name], 'exact implementation differs')
    path = root / 'tools/xtask-package-attribution-074/review.py'
    require(sha(path.read_bytes()) == sha(members['parser/tools/xtask-package-attribution-074/review.py']) == PARSER,
            'fixed local parser differs')
    spec = importlib.util.spec_from_file_location('checked_v6_local_parser', path)
    parser = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(parser)
    accounting = parser.parse_transcript(members['checks/package-v6-first/stdout.log'].decode(),
        members['checks/package-v6-first/stderr.log'].decode(), gate['targets'], result['exit'], result['outer_refusal'], 780)
    require(accounting == json.loads(members['derived/accounting.json']) and
            (accounting['completed_targets'], accounting['passed'], accounting['failed']) == (72, 448, 0)
            and len(accounting['unstarted_targets']) == 72 and accounting['package_pass'] is False,
            'first package accounting differs')
    partial = accounting['incomplete_target']
    require(partial['target'] == 'tests/held_complete_role_census_074.rs' and partial['declared'] == 4 and
            len(partial['cases']) == 3 and partial['reported_seconds'] is None and
            all(row['verdict'] == 'ok' for row in partial['cases']), 'censored frontier differs')
    targets = {row['target']: row for row in accounting['targets']}
    for name, count in (('authority_rust_history_074', 10), ('ci_reliability_071', 12), ('fresh_status_reader_074', 2)):
        row = targets['tests/' + name + '.rs']
        require(row['passed'] == count and row['failed'] == row['ignored'] == 0, 'old failure repair differs')
    require(members['checks/package-v6-first/owned-cleanup.log'].count(b'could not be terminated') == 1,
            'cleanup error accounting differs')
    joined = json.loads(members['checks/package-v6-cost-evidence-corrected/stdout.log'])
    require(joined['v5_helper_bytes_identical_to_old_profile_baseline'] is True and
            all(joined[key] is False for key in ('old_profile_times_transferred_to_v5',
                'current_transitive_read_graph_attested', 'cost_saving_estimated', 'runtime_cache_or_batch_admitted',
                'observations_changed', 'package_replayed', 'workload_or_git_launched', 'server', 'package_pass')),
            'offline evidence join scope differs')
    return dict(completed=72, expected=145, passed_completed=448, failed_completed=0,
                partial_passed=3, partial_without_verdict=1, unstarted=72,
                package_pass=False, workload_replayed=False, all_writer_retirement_proven=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
