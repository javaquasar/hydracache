"""Offline evidence replay only: never execute retained code or start a fixture."""
import hashlib
import json
from pathlib import Path
import re

INDEX_SHA256 = 'd0ce29b1044d27e41c7342a20865034bf107fbfc2e094fbeb9123eaa9e995331'
RULES_SHA256 = '188763c292af6f6f603414ace264926009c59d7795cea1047084867609c77023'
BASE = '52abf7ab0efd35557eb3117a45c253e0ebe76469'
EXITS = {name: 0 for name in ['history-status-candidate-first',
    'history-status-current-final', 'history-status-msrv-final',
    'history-status-producer-final', 'history-status-check-final',
    'history-status-current-shared', 'history-status-msrv-shared',
    'history-status-producer-shared', 'history-status-check-shared',
    'history-status-clippy-shared', 'history-status-fmt-shared', 'watchdog-taskkill-held-first',
    'history-status-doc-check-first', 'history-status-contract-check-first']}
EXITS.update({'history-status-red-first': 101, 'history-status-fmt-first': 1,
              'history-status-clippy-final': 101})
COUNTS = [6, 5, 10, 6, 7, 1, 4, 2, 3, 2, 2, 2, 4, 4, 4, 16, 4]


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def check(index, members):
    require(index['schema'] == 'hc074-history-status-cleanup-evidence-v1' and
            index['baseline_revision'] == BASE and index['expected_exit'] == EXITS, 'fixed scope')
    require([index[k] for k in ['each_compiler_cases', 'original_cases', 'new_cases',
            'modified_old_cases']] == [82, 78, 4, 12], 'case scope')
    require(all(index[k] is False for k in ['original_watchdog_rerun', 'full_package_rerun',
            'server', 'qualification', 'runtime_changes']), 'admission scope')
    require(index['previous_package_index_sha256'] ==
            'd6e157fecd2749e29122dfd95f6029e864284c407ab5a6aad91880f41bb4b011', 'old package reference')
    require(set(members) == set(index['members']), 'member scope')
    intents = {}
    for label, expected in EXITS.items():
        prefix = 'checks/' + label + '/'
        intent = json.loads(members[prefix + 'intent.json'])
        result = json.loads(members[prefix + 'result.json'])
        require(result['exit'] == expected and result['outer_refusal'] is None and
                result['source_unchanged'] is True and result['head_unchanged'] is True and
                result['source_after'] == intent['source'] and intent['head'] == BASE, 'check outcome/source')
        require(result['admission'] is False and result['qualification'] is False, 'check admission')
        require(intent['bound_seconds'] == 1200 and intent['disk_floor_bytes'] == 4 * 1024**3 and
                all(intent[k] is False for k in ['dependency_changes', 'observation_frequency_changed',
                    'services_or_clocks', 'server', 'original_owner', 'qualification', 'retry']), 'check limits')
        intents[label] = intent
    for label in ['history-status-current-final', 'history-status-msrv-final',
                  'history-status-current-shared', 'history-status-msrv-shared']:
        counts = re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;',
                            members['checks/' + label + '/stdout.log'].decode())
        require(list(map(int, counts)) == COUNTS, 'complete cohort counts')
    require(b'2 passed; 2 failed;' in members['checks/history-status-red-first/stdout.log'], 'first red result')
    counts = re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;',
                        members['checks/history-status-candidate-first/stdout.log'].decode())
    require(list(map(int, counts)) == [5, 10, 1, 4, 2, 4], 'first candidate counts')
    require(b'duplicate_mod' in members['checks/history-status-clippy-final/stderr.log'], 'first strict refusal')
    require(b'never used' in members['checks/history-status-candidate-first/stderr.log'], 'first warnings')
    bindings = [('source-final/', ['history-status-current-shared', 'history-status-msrv-shared',
                                  'history-status-clippy-shared']),
                ('initial-candidate/', ['history-status-candidate-first']),
                ('preformat/', ['history-status-fmt-first']),
                ('preclippy/', ['history-status-current-final', 'history-status-msrv-final',
                               'history-status-clippy-final'])]
    for prefix, labels in bindings:
        for name, body in members.items():
            if name.startswith(prefix):
                path = name.removeprefix(prefix)
                for label in labels:
                    require(sha(body) == intents[label]['source'][path], 'source receipt binding')
    require(sha(members['initial-red-test.rs']) == intents['history-status-red-first']['source'][
        'crates/xtask/tests/history_status_readers_074.rs'], 'first test source')
    rules_raw = members['source-final/tools/history-status-readers-074/rules.json']
    require(sha(rules_raw) == RULES_SHA256, 'fixed constructor descriptor')
    rules = json.loads(rules_raw)
    require(rules['baseline'] == BASE and len(rules['rows']) == 3, 'constructor scope')
    for row in rules['rows']:
        old = members['baseline/' + row['path']].decode().replace('\r\n', '\n')
        current = members['source-final/' + row['path']].decode().replace('\r\n', '\n')
        require(sha(old.encode()) == row['before_sha256'] and old.count(row['before']) == 1, 'fixed predecessor')
        require(old.replace(row['before'], row['after']) + row['append'] == current and
                sha(current.encode()) == row['after_sha256'], 'exact successor')
    source = members['source-final/crates/xtask/tests/history_status_readers_074.rs']
    require(b'use original::history_reader::frontier_phase as phase;' in source and
            b'mod phase;' not in source, 'single module reuse')
    for path, digest in [('Cargo.toml', '18a8a27cf140a1d6eb31c8316ee39bb7d5832866a990346eee16cd0c30de344b'),
                         ('Cargo.lock', 'd2c7736ebaa8381cece9162bbbf384db60c46111051e4e78e51d4bae12feaf5c')]:
        require(sha(members['source-final/' + path]) == digest, 'dependency baseline')
    intent = json.loads(members['cleanup/intent.json'])
    result = json.loads(members['cleanup/result.json'])
    require(sha(members['coordinator/watchdog-taskkill-handles-20261026.py']) == intent['source_sha256'] and
            sha(members['cleanup-source/docs/testing/performance/0.74/watchdog-held-taskkill-contract-20261026.md']) ==
            intent['contract_sha256'] and sha(members['cleanup-source/scripts/ci/run-with-heartbeat.py']) ==
            intent['original_watchdog_sha256'], 'cleanup fixed source')
    require(intent['taskkill_attempt_limit'] == 1 and intent['post_enrollment_delay_seconds'] == 0.35 and
            intent['marker_delay_seconds'] == 1.5 and intent['node_self_exit_seconds'] == 8, 'cleanup bounds')
    require(sha(members['protected/docs/testing/performance/0.74/qualification-manifest.toml']) ==
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification unchanged')
    require(result['captured'] is True and result['refusal'] is None and result['cleanup_errors'] == [] and
            result['termination_calls'] == 1 and result['cleanup_terminal'] is True and
            result['taskkill_exit'] == 0 and result['marker_after_cut'] is True, 'cleanup first outcome')
    require(result['original_test_pass'] is False and result['package_pass'] is False and
            intent['original_test_rerun'] is False and intent['production_retirement_admission'] is False,
            'cleanup is not admission')
    require(len(result['identities']) == len(result['after_cut']) == len(result['terminal_objects']) == 2,
            'held object scope')
    for identity, first, terminal in zip(result['identities'], result['after_cut'], result['terminal_objects']):
        require(first['terminal'] is True and terminal['terminal'] is True and
                identity['creation_100ns'] == first['creation_100ns'] == terminal['creation_100ns'] and
                first['exit_100ns'] == terminal['exit_100ns'] > identity['creation_100ns'], 'original held object')


def verify(folder):
    raw = (folder / 'index.json').read_bytes()
    require(sha(raw) == INDEX_SHA256, 'fixed index')
    index = json.loads(raw)
    members = {}
    for name, row in index['members'].items():
        require(re.fullmatch('[a-f0-9]{64}', row['sha256']), 'object digest')
        raw = (folder / 'objects' / row['sha256']).read_bytes()
        require(sha(raw) == row['sha256'] and len(raw) == row['bytes'], 'raw object')
        members[name] = raw
    check(index, members)
    return index, members


if __name__ == '__main__':
    index, _ = verify(Path(__file__).resolve().parent)
    print('Verified: two separate 82-case final compiler cohorts; first refusals retained; cleanup diagnostic only.')
