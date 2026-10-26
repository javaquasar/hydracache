"""Offline source and first-outcome replay; no retained code is executed."""
import hashlib
import json
from pathlib import Path
import re

INDEX_SHA256 = '685401b61760909eb6539a17f4f7ff3b4d1f567be3bbd808863a80697038e211'
RULES_SHA256 = '4ef20240b953381c2e4dbfe7892b0a9205dbdefba3012e89a113030372d0d389'
PHASE_SHA256 = 'fec2195d38bd7784e9c584704b90d5851f5a5c228a900513aa11cf91b42fad8f'
OBSERVED_SHA256 = 'e6ba645e84f7e462844085fd0c34297866fe1349e631468dbf68f845e33a14a1'
BASE = 'bf352a1dfbe3040af9ce058a5d42efe55202db62'
EXITS = {name: 0 for name in ['current-frontier-candidate-first', 'current-frontier-msrv-first',
    'current-frontier-producer-first', 'current-frontier-check-first', 'current-frontier-clippy-first',
    'current-frontier-fmt-first', 'current-frontier-doc-check-first', 'current-frontier-contract-check-first']}
EXITS['current-frontier-baseline-first'] = 101


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(raw):
    return raw.decode().replace('\r\n', '\n').encode()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def verify(folder):
    raw = (folder / 'index.json').read_bytes()
    require(sha(raw) == INDEX_SHA256, 'fixed index differs')
    index = json.loads(raw)
    require(index['schema'] == 'hc074-current-frontier-evidence-v1' and
            index['baseline_revision'] == BASE and index['expected_exit'] == EXITS, 'identity differs')
    require(all(index[key] is False for key in ['full_package_rerun', 'runtime_or_deadline_changes',
                                              'server', 'qualification']), 'scope differs')
    require([index[key] for key in ['baseline_cases', 'old_cases', 'new_cases',
            'accounted_existing', 'accounted_additions']] == [5, 14, 7, 12, 14], 'case scope differs')
    members, intents = {}, {}
    for name, row in index['members'].items():
        require(re.fullmatch('[a-f0-9]{64}', row['sha256']), 'invalid digest')
        body = (folder / 'objects' / row['sha256']).read_bytes()
        require(sha(body) == row['sha256'] and len(body) == row['bytes'], 'raw member differs')
        members[name] = body
    supplement_raw = (folder / 'observed-bindings.json').read_bytes()
    require(sha(supplement_raw) == OBSERVED_SHA256, 'fixed observed index differs')
    supplement = json.loads(supplement_raw)
    require(supplement['schema'] == 'hc074-current-frontier-observed-bindings-v1' and
            supplement['original_index_sha256'] == INDEX_SHA256 and supplement['raw_inputs'] == 25 and
            supplement['later_reconstruction_of_original_raw_inputs'] is True and
            supplement['tests_rerun'] is False, 'observed reconstruction scope differs')
    for name, row in supplement['members'].items():
        require(name not in members and re.fullmatch('[a-f0-9]{64}', row['sha256']), 'observed member identity differs')
        body = (folder / 'objects' / row['sha256']).read_bytes()
        require(sha(body) == row['sha256'] and len(body) == row['bytes'], 'raw observed member differs')
        members[name] = body
    for label, expected in EXITS.items():
        prefix = 'checks/' + label + '/'
        intent = json.loads(members[prefix + 'intent.json'])
        result = json.loads(members[prefix + 'result.json'])
        require(result['exit'] == expected and result['outer_refusal'] is None and
                result['admission'] is False and result['qualification'] is False, 'check outcome differs')
        require(result['source_unchanged'] is True and result['head_unchanged'] is True and
                result['source_after'] == intent['source'] and intent['head'] == BASE, 'check source differs')
        require(intent['bound_seconds'] == 1200 and intent['disk_floor_bytes'] == 4 * 1024**3 and
                all(intent[key] is False for key in ['dependency_changes', 'observation_frequency_changed',
                    'services_or_clocks', 'server', 'original_owner', 'qualification', 'retry']), 'check bounds differ')
        require(intent['source']['Cargo.toml'] == '18a8a27cf140a1d6eb31c8316ee39bb7d5832866a990346eee16cd0c30de344b' and
                intent['source']['Cargo.lock'] == 'd2c7736ebaa8381cece9162bbbf384db60c46111051e4e78e51d4bae12feaf5c', 'dependency baseline differs')
        intents[label] = intent
    for label, expected in {'current-frontier-baseline-first': [(0, 1), (4, 0)],
                            'current-frontier-candidate-first': [(9, 0), (7, 0), (1, 0), (4, 0)],
                            'current-frontier-msrv-first': [(9, 0), (7, 0), (1, 0), (4, 0)]}.items():
        log = members['checks/' + label + '/stdout.log'].decode()
        observed = [(int(passed), int(failed)) for passed, failed in re.findall(
            r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; 0 ignored; 0 measured; 0 filtered out;', log)]
        require(observed == expected, 'cohort accounting differs')
    log = members['checks/current-frontier-baseline-first/stdout.log'].decode()
    require(len(re.findall(r'^CURRENT_TREE_FRONTIER ', log, re.M)) == 26, 'first frontier differs')
    for label in ['current-frontier-candidate-first', 'current-frontier-msrv-first']:
        require(b'CURRENT_TREE_FRONTIER ' not in members['checks/' + label + '/stdout.log'], 'final audit refusal')
    for name, body in members.items():
        if name.startswith('source-final/'):
            path = name.removeprefix('source-final/')
            for label in ['current-frontier-msrv-first', 'current-frontier-clippy-first']:
                require(sha(body) == intents[label]['source'][path], 'final source binding differs')
            if not path.endswith('/README.md'):
                require(sha(body) == intents['current-frontier-candidate-first']['source'][path], 'current source binding differs')
    raw = members['source-final/tools/current-frontier-074/rules.json']
    require(sha(raw) == RULES_SHA256, 'independent source rules differ')
    rules = json.loads(raw)
    require(rules['baseline'] == BASE and rules['source'] == '33c54b0cda8c330b115009cdf092ad002077cedf' and
            len(rules['rows']) == len({row['path'] for row in rules['rows']}) == 12, 'source scope differs')
    for row in rules['rows']:
        current = canonical(members['producer/' + row['revision'] + '/' + row['path']])
        previous = canonical(members['producer/' + row['previous_revision'] + '/' + row['path']])
        require(sha(current) == row['current_sha256'] and sha(previous) == row['previous_sha256'], 'producer differs')
        if row['path'] in intents['current-frontier-candidate-first']['source']:
            observed = members['observed/' + row['path']]
            require(canonical(observed) == current, 'observed producer canonical bytes differ')
            for label in ['current-frontier-candidate-first', 'current-frontier-msrv-first', 'current-frontier-clippy-first']:
                require(intents[label]['source'][row['path']] == sha(observed), 'observed producer binding differs')
        limit = len(current)
        for edit in reversed(row['patches']):
            at = edit['offset']
            exact = edit['current'].encode()
            end = at + len(exact)
            require(0 <= at <= end <= limit and current[at:end] == exact, 'inverse edit differs')
            current = current[:at] + edit['previous'].encode() + current[end:]
            limit = at
        require(current == previous, 'whole predecessor differs')
        expected = '37db65f68e67ed7c0ee0f1cc84fed1c30bf9822a' if row['path'] == 'crates/xtask/tests/performance_contract_074.rs' else rules['source']
        require(row['previous_revision'] == expected, 'main gate predecessor differs')
    audit = rules['audit']
    old = canonical(members['baseline/' + audit['path']])
    require(sha(old) == audit['before_sha256'] and len(audit['edits']) == 3, 'audit predecessor differs')
    current = old.decode()
    for edit in audit['edits']:
        require(edit['before'] and current.count(edit['before']) == edit['count'] == 1, 'audit edit differs')
        current = current.replace(edit['before'], edit['after'])
    current = (current + audit['append']).encode()
    require(sha(current) == audit['after_sha256'] and current == canonical(members['source-final/' + audit['path']]), 'whole audit differs')
    for path in ['crates/xtask/tests/support/current_tree_input.rs', 'crates/xtask/tests/support/current_tree_input_rules_074.json']:
        require(canonical(members['baseline/' + path]) == canonical(members['source-final/' + path]), 'old reader/rules changed')
    old = json.loads(members['source-final/crates/xtask/tests/support/current_tree_input_rules_074.json'])
    require(len(old['patches']) == 9 and len(old['additions']) == 81, 'old rule counts differ')
    raw = members['independent-inventory544.json']
    require(sha(raw) == PHASE_SHA256, 'independent inventory differs')
    phase = json.loads(raw)
    require(phase['revision'] == '958b3135a15c00b9f6aad41522cef9d1f6608d3a' and len(phase['added']) == 14, 'addition scope differs')
    for path, row in phase['added'].items():
        body = members['addition-producer/' + path]
        blob = hashlib.sha1(b'blob ' + str(len(body)).encode() + b'\0' + body).hexdigest()
        require(row['mode'] == '100644' and row['blob'] == blob and
                row['canonical_sha256'] == sha(canonical(body)) and path not in old['additions'], 'addition producer differs')
        observed = members['observed/' + path]
        require(canonical(observed) == canonical(body), 'observed addition canonical bytes differ')
        for label in ['current-frontier-candidate-first', 'current-frontier-msrv-first', 'current-frontier-clippy-first']:
            require(intents[label]['source'][path] == sha(observed), 'observed addition binding differs')
    refusal = json.loads(members['offline-enrichment-first-refusal.json'])
    require(refusal['verifier_exit'] == 1 and refusal['controls_passed'] == 5 and
            refusal['controls_errored'] == 1 and refusal['raw_stream_capture_available'] is False and
            refusal['error'] == 'ValueError: observed producer binding differs', 'enrichment first refusal differs')
    return dict(cases_per_compiler=21, accounted_inputs=26, complete_audit=True,
                full_package_rerun=False, code_executed=False, qualification=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
