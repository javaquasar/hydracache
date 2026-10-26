"""Offline first-outcome accounting; no retained implementation or workload execution."""
import hashlib
import importlib.util
import json
from pathlib import Path

INDEX = 'd6e157fecd2749e29122dfd95f6029e864284c407ab5a6aad91880f41bb4b011'
SOURCE = '9cc82116af63c7eac2e6ca4a6ff4c4f5e815a8c3'
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
        require(sha(body) == row['sha256'] and len(body) == row['bytes'], 'raw member differs: ' + name)
        members[name] = body
    require(len(members) == 112, 'member count differs')
    gate = json.loads(members['coordinator/first/gate.json'])
    before_gate = json.loads(members['coordinator/preflight-first/gate.json'])
    after_gate = json.loads(members['checks/package-v5-postflight-review-first/stdout.log'])
    require(gate == before_gate and {k: v for k, v in gate.items() if k != 'head'} == after_gate,
            'preflight/postflight review differs')
    require(gate['head'] == SOURCE and gate['source_effect_review_complete'] is True and
            len(gate['sources']) == 365 and len(gate['auxiliary']) == 42 and len(gate['targets']) == 144,
            'review scope differs')
    require(gate['reviewed_ast_sha256'] == '074a38382393cf290329a5018a71a5946348b024da3ed7e7e0ccd502bbac8733',
            'reviewed syntax identity differs')
    require(all(gate[key] is False for key in ('formal_whole_program_proof', 'package_pass',
                'package_dispatched', 'linux_or_all_features_cleared',
                'original_execution_or_product_or_qualification_admission')), 'review is not product admission')
    labels = ['package-effect-v5-controls-first', 'package-replay-v5-controls-first', 'package-v5-focused-final',
              'package-v5-review-first', 'package-v5-check-first', 'package-v5-clippy-first', 'package-v5-fmt-first',
              'package-v5-contract-doc-first', 'package-v5-preflight-first', 'package-v5-cleanup-observation-first',
              'package-v5-postflight-review-first', 'package-v5-first']
    intents = {}
    results = {}
    for label in labels:
        prefix = 'checks/' + label + '/'
        intent = json.loads(members[prefix + 'intent.json'])
        result = json.loads(members[prefix + 'result.json'])
        intents[label], results[label] = intent, result
        full = label == 'package-v5-first'
        unstable = label == 'package-effect-v5-controls-first'
        require(result['exit'] == (1 if full else 0) and result['outer_refusal'] == ('outer-timeout' if full else None)
                and result['head_unchanged'] is True and result['source_unchanged'] is (not unstable),
                'first check outcome differs')
        require(intent['bound_seconds'] == (780 if full else 1200) and
                intent['disk_floor_bytes'] == 4 * 1024**3, 'check bound differs')
        require(all(intent[key] is False for key in ('dependency_changes', 'observation_frequency_changed',
                'services_or_clocks', 'server', 'original_owner', 'qualification', 'retry')), 'check scope differs')
        if not unstable:
            require(intent['source'] == result['source_after'], 'source changed')
    first = intents['package-effect-v5-controls-first']['source']
    later = results['package-effect-v5-controls-first']['source_after']
    require(set(later) - set(first) == {'tools/historical-blob-batch-074/package_replay_v5.py',
                'tools/historical-blob-batch-074/test_package_replay_v5.py'} and
            all(later.get(name) == digest for name, digest in first.items()), 'initial source-change refusal differs')
    require('Ran 42 tests' in members['checks/package-v5-focused-final/stderr.log'].decode() and
            members['checks/package-v5-focused-final/stderr.log'].decode().rstrip().endswith('OK'), 'focused cohort differs')
    full = intents['package-v5-first']
    result = results['package-v5-first']
    require(full['head'] == SOURCE and full['argv'] == ARGV and
            all(full['source'].get(name) == digest for name, digest in gate['sources'].items()),
            'package source or command differs')
    for label in ['preflight-first', 'first']:
        intent = json.loads(members['coordinator/' + label + '/intent.json'])
        require(intent['argv'] == ARGV and intent['execute'] is (label == 'first') and
                intent['package_bound_seconds'] == 780 and intent['case_threads'] == 4 and
                all(intent[key] is False for key in ('retry', 'observer', 'services', 'clocks', 'server', 'qualification')),
                'coordinator intent differs')
    wrappers = {
        'target/run-xtask-package-780-20261024.py': '52504f2b489d16f344b477df0568f33ac224c5cadacc552f32f427f39c09d04d',
        'target/run-source-step-20261024.py': 'a92482cabcfcf8f6440e95972bee8d204bb9c59c8805c45c7b57363dcc3ab430',
    }
    for name, digest in wrappers.items():
        require(sha(members['wrapper/' + Path(name).name]) == digest, 'original wrapper differs')
    require(json.loads(members['coordinator/first/coordinator.json'])['wrappers'] == wrappers, 'wrapper receipt differs')
    for name, digest in gate['auxiliary'].items():
        require(sha(members['auxiliary/' + name]) == digest, 'auxiliary body differs')
    for name in ('package_effects_review_v5.py', 'package_replay_v5.py',
                 'test_package_effects_review_v5.py', 'test_package_replay_v5.py'):
        path = 'tools/historical-blob-batch-074/' + name
        require(sha(members['implementation/' + path]) == full['source'][path], 'implementation differs')
    path = root / 'tools/xtask-package-attribution-074/review.py'
    require(sha(path.read_bytes()) == sha(members['parser/tools/xtask-package-attribution-074/review.py']) == PARSER,
            'reviewed local parser differs')
    spec = importlib.util.spec_from_file_location('checked_local_package_parser', path)
    parser = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(parser)
    analysis = parser.parse_transcript(members['checks/package-v5-first/stdout.log'].decode(),
        members['checks/package-v5-first/stderr.log'].decode(), gate['targets'], result['exit'], result['outer_refusal'], 780)
    require(analysis == json.loads(members['derived/accounting.json']), 'derived accounting differs')
    require((analysis['completed_targets'], analysis['passed'], analysis['failed']) == (76, 457, 3) and
            len(analysis['unstarted_targets']) == 67 and analysis['package_pass'] is False, 'first package outcome differs')
    partial = analysis['incomplete_target']
    require(partial['target'] == 'tests/held_preparation_cost_074.rs' and partial['declared'] == 4 and
            len(partial['cases']) == 3 and all(case['verdict'] == 'ok' for case in partial['cases']), 'partial coverage differs')
    cleanup = members['checks/package-v5-first/owned-cleanup.log']
    observation = json.loads(members['checks/package-v5-cleanup-observation-first/stdout.log'])
    require(cleanup.count(b'could not be terminated') == 2 and observation['cleanup_log_sha256'] == sha(cleanup)
            and observation['named_pid_observations'] == [] and observation['watchdog_marker_process_observations'] == []
            and observation['original_all_writer_retirement_proven'] is False
            and observation['terminated_anything'] is False, 'cleanup observations differ')
    return dict(completed_targets=76, expected_targets=144, passed_completed=457, failed_completed=3,
                partial_passed=3, partial_without_verdict=1, unstarted_targets=67,
                package_pass=False, workload_replayed=False, original_all_writer_retirement_proven=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
