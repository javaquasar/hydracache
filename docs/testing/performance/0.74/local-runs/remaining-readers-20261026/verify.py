"""Offline exact caller and first-outcome replay; never executes captured source."""
import hashlib
import json
from pathlib import Path
import re

INDEX_SHA256 = '83156b61416eeee2cf07afc191b1bd2570534ee710488bf875db64b934c82fe1'
RULES_SHA256 = 'b8803a5f12460fa2278ffef033dc0738c4dc925a65a661f701ea09ae9bb6883e'
PREFORMAT_SHA256 = 'a1cad8de5aa96602623cc50af59debe82ef7c30d8d582233eebeab5cb84d77a1'
FIRST_RULES_SHA256 = '70e398f840925e2d703b726b07f7e81c189f2e6f8e8733bc15fbc00351405a95'
SUCCESSOR_SHA256 = '61d3e1941cdae497600592a49af0487e4cf545573ba3f091add173110b560cda'
BASE = '3e7e0aa343ed51f99651761f9bb7b0b10efd359a'
EXITS = {label: 0 for label in [
    'remaining-readers-candidate-first', 'remaining-readers-candidate-final',
    'remaining-readers-candidate-second', 'remaining-readers-current-controls-second',
    'remaining-readers-msrv-first', 'remaining-readers-producer-first',
    'remaining-readers-check-first', 'remaining-readers-clippy-first',
    'remaining-readers-fmt-second', 'remaining-readers-doc-check-first', 'remaining-readers-contract-check-first',
    'remaining-readers-postformat-current', 'remaining-readers-postformat-msrv',
    'remaining-readers-producer-final', 'remaining-readers-check-final',
    'remaining-readers-clippy-final', 'remaining-readers-fmt-final']}
EXITS.update({'remaining-readers-baseline-first': 101, 'remaining-readers-current-controls': 101})
EXITS['remaining-readers-fmt-first'] = 1


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
    require(index['schema'] == 'hc074-remaining-readers-evidence-v1' and
            index['baseline_revision'] == BASE and index['expected_exit'] == EXITS, 'identity differs')
    require([index[key] for key in ['baseline_cases', 'baseline_failed', 'original_cases', 'new_cases',
            'each_compiler_distinct_cases', 'original_cases_in_modified_files']] == [27, 2, 57, 4, 61, 18], 'case scope differs')
    require(all(index[key] is False for key in ['full_package_rerun', 'runtime_or_deadline_changes',
                                              'server', 'qualification']), 'scope differs')
    members, intents = {}, {}
    for name, row in index['members'].items():
        require(re.fullmatch('[a-f0-9]{64}', row['sha256']), 'invalid object digest')
        body = (folder / 'objects' / row['sha256']).read_bytes()
        require(sha(body) == row['sha256'] and len(body) == row['bytes'], 'raw member differs')
        members[name] = body
    for label, expected in EXITS.items():
        prefix = 'checks/' + label + '/'
        intent = json.loads(members[prefix + 'intent.json'])
        result = json.loads(members[prefix + 'result.json'])
        require(result['exit'] == expected and result['outer_refusal'] is None and
                result['admission'] is False and result['qualification'] is False, 'first outcome differs')
        require(result['source_unchanged'] is True and result['head_unchanged'] is True and
                result['source_after'] == intent['source'] and intent['head'] == BASE, 'source changed inside check')
        require(intent['bound_seconds'] == 1200 and intent['disk_floor_bytes'] == 4 * 1024**3 and
                all(intent[key] is False for key in ['dependency_changes', 'observation_frequency_changed',
                    'services_or_clocks', 'server', 'original_owner', 'qualification', 'retry']), 'check bounds differ')
        require(intent['source']['Cargo.toml'] == '18a8a27cf140a1d6eb31c8316ee39bb7d5832866a990346eee16cd0c30de344b' and
                intent['source']['Cargo.lock'] == 'd2c7736ebaa8381cece9162bbbf384db60c46111051e4e78e51d4bae12feaf5c', 'dependency baseline differs')
        intents[label] = intent
    passing = [(value, 0) for value in [6, 6, 7, 1, 4, 2, 4, 4, 4]]
    cohorts = {label: passing for label in ['remaining-readers-candidate-first',
               'remaining-readers-candidate-final', 'remaining-readers-candidate-second']}
    full = [(value, 0) for value in [6, 6, 7, 1, 4, 2, 3, 2, 2, 4, 4, 16, 4]]
    cohorts.update({
        'remaining-readers-baseline-first': [(2, 0), (3, 0), (1, 1), (3, 1), (16, 0)],
        'remaining-readers-current-controls': [(2, 0), (2, 1), (16, 0)],
        'remaining-readers-current-controls-second': [(2, 0), (3, 0), (2, 0), (16, 0)],
        'remaining-readers-msrv-first': full,
        'remaining-readers-postformat-current': full,
        'remaining-readers-postformat-msrv': full})
    for label, expected in cohorts.items():
        log = members['checks/' + label + '/stdout.log'].decode()
        actual = [(int(passed), int(failed)) for passed, failed in re.findall(
            r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; 0 ignored; 0 measured; 0 filtered out;', log)]
        require(actual == expected, 'cohort accounting differs')
    require(b'never used' in members['checks/remaining-readers-candidate-first/stderr.log'], 'initial warnings missing')
    require(b'unreviewed document scratch reader: crates/xtask/tests/fresh_kernel_cost_074.rs' in
            members['checks/remaining-readers-current-controls/stdout.log'], 'first scratch integration refusal missing')
    require(b'remaining_phase::previous(path, &raw).expect' in
            members['checks/remaining-readers-fmt-first/stdout.log'], 'first formatting refusal missing')
    final_labels = ['remaining-readers-postformat-current', 'remaining-readers-postformat-msrv',
                    'remaining-readers-clippy-final']
    for name, body in members.items():
        if name.startswith('source-final/'):
            path = name.removeprefix('source-final/')
            for label in final_labels:
                require(sha(body) == intents[label]['source'][path], 'final source binding differs')
        for prefix, label in [('initial-source/', 'remaining-readers-candidate-first'),
                              ('interim-source/', 'remaining-readers-candidate-final')]:
            if name.startswith(prefix):
                path = name.removeprefix(prefix)
                require(sha(body) == intents[label]['source'][path], 'retained intermediate source differs')
        if name.startswith('preformat-source/'):
            path = name.removeprefix('preformat-source/')
            for label in ['remaining-readers-candidate-second', 'remaining-readers-current-controls-second',
                          'remaining-readers-msrv-first']:
                require(sha(body) == intents[label]['source'][path], 'pre-format source binding differs')
    require(sha(members['initial-six-caller-rules.json']) == FIRST_RULES_SHA256, 'initial descriptor differs')
    rules_raw = members['source-final/tools/remaining-readers-074/rules.json']
    require(sha(rules_raw) == RULES_SHA256, 'fixed final descriptor differs')
    rules = json.loads(rules_raw)
    require(rules['schema'] == 'hc074-remaining-readers-v1' and rules['baseline'] == BASE and
            len(rules['rows']) == len({row['path'] for row in rules['rows']}) == 7, 'caller scope differs')
    preformat_raw = members['preformat-source/tools/remaining-readers-074/rules.json']
    require(sha(preformat_raw) == PREFORMAT_SHA256, 'pre-format descriptor differs')
    preformat = json.loads(preformat_raw)
    initial = json.loads(members['initial-six-caller-rules.json'])
    require(initial['baseline'] == BASE and initial['rows'] == preformat['rows'][:6], 'first caller description changed')
    for row in initial['rows']:
        prefix = 'preformat-source/' if row['path'] == 'tools/bootstrap-doc-reader-074/reader.rs' else 'source-final/'
        body = members[prefix + row['path']]
        for label in ['remaining-readers-candidate-first', 'remaining-readers-candidate-final']:
            require(sha(body) == intents[label]['source'][row['path']], 'first caller source binding differs')
    old_layout = 'remaining_phase::previous(path, &raw).expect'
    new_layout = 'remaining_phase::previous(path, &raw)\n            .expect'
    before = canonical(members['preformat-source/tools/bootstrap-doc-reader-074/reader.rs']).decode()
    after = canonical(members['source-final/tools/bootstrap-doc-reader-074/reader.rs']).decode()
    require(before.count(old_layout) == 1 and before.replace(old_layout, new_layout) == after, 'formatting body delta differs')
    require(members['preformat-source/tools/remaining-readers-074/contract.rs'].replace(
            PREFORMAT_SHA256.encode(), RULES_SHA256.encode()) == members['source-final/tools/remaining-readers-074/contract.rs'],
            'formatting contract delta differs')
    changed_row = preformat['rows'][5]
    changed_row['after_sha256'] = sha(after.encode())
    changed_row['edits'][0]['after'] = changed_row['edits'][0]['after'].replace(old_layout, new_layout)
    require(preformat == rules, 'formatting descriptor delta differs')
    reconstructed = members['interim-source/crates/xtask/tests/remaining_readers_074.rs']
    for before, after in [(b'fn six_exact_caller_constructions_preserve_all_eighteen_cases_and_assertions()',
                           b'fn seven_exact_caller_constructions_preserve_all_eighteen_cases_and_assertions()'),
                          (b'let counts = [2, 4, 4, 1, 7, 0];', b'let counts = [2, 4, 4, 1, 7, 0, 0];')]:
        require(reconstructed.count(before) == 1, 'pre-format reconstruction anchor differs')
        reconstructed = reconstructed.replace(before, after)
    require(reconstructed == members['preformat-source/crates/xtask/tests/remaining_readers_074.rs'] and
            json.loads(members['preformat-test-reconstruction.json'])['derived_data_not_new_run'] is True,
            'pre-format control reconstruction differs')
    for row in rules['rows']:
        old = canonical(members['baseline/' + row['path']])
        require(sha(old) == row['before_sha256'], 'fixed predecessor differs')
        current = old.decode()
        for edit in row['edits']:
            require(edit['before'] and current.count(edit['before']) == edit['count'] == 1, 'caller edit differs')
            current = current.replace(edit['before'], edit['after'])
        current = (current + row['append']).encode()
        require(sha(current) == row['after_sha256'] and current ==
                canonical(members['source-final/' + row['path']]), 'whole caller successor differs')
    for path in ['crates/xtask/tests/support/current_tree_input.rs', 'crates/xtask/tests/support/current_tree_input_rules_074.json']:
        body = members['source-final/' + path]
        require(sha(body) == intents['remaining-readers-baseline-first']['source'][path], 'immutable legacy validator changed')
    legacy = json.loads(members['source-final/crates/xtask/tests/support/current_tree_input_rules_074.json'])
    require(len(legacy['patches']) == 9 and len(legacy['additions']) == 81, 'legacy counts differ')
    raw = members['source-final/tools/source-successor-view-074/rules.json']
    require(sha(raw) == SUCCESSOR_SHA256, 'independent successor descriptor differs')
    rows = [row for row in json.loads(raw)['rows'] if row['path'] in [
        'docs/COMPAT.md', 'tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs']]
    require(len(rows) == 2, 'input projection scope differs')
    for row in rows:
        current = canonical(members['producer/' + row['revision'] + '/' + row['path']])
        previous = canonical(members['producer/' + row['previous_revision'] + '/' + row['path']])
        require(sha(current) == row['current_sha256'] and sha(previous) == row['previous_sha256'], 'input producer differs')
        limit = len(current)
        for edit in reversed(row['patches']):
            at = edit['offset']
            exact = edit['current'].encode()
            end = at + len(exact)
            require(0 <= at <= end <= limit and current[at:end] == exact, 'input inverse edit differs')
            current = current[:at] + edit['previous'].encode() + current[end:]
            limit = at
        require(current == previous, 'whole input predecessor differs')
    return dict(cases_per_compiler=61, original_cases=57, new_cases=4,
                full_package_rerun=False, code_executed=False, qualification=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
