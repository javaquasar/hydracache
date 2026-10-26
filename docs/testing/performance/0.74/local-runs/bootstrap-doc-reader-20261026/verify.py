"""Offline raw receipt and exact delta replay; never executes retained source."""
import hashlib
import json
from pathlib import Path
import re

INDEX_SHA256 = '7031d30ae5ab7f78609c5a96f96923b682cdeb7be600b6d1cd4c6e8b119a763f'
RULES_SHA256 = 'f780c166ff6e708eb2b09013075e414426bad8ae443f537a5ad0a08dcebfb3ce'
SUCCESSOR_SHA256 = '61d3e1941cdae497600592a49af0487e4cf545573ba3f091add173110b560cda'
BASE = '181c077abdfbae5a8c4dfe67cee6f1c95519d529'
EXITS = {name: 0 for name in [
    'bootstrap-doc-reader-candidate-first', 'bootstrap-doc-reader-msrv-first',
    'bootstrap-doc-reader-producer-first', 'bootstrap-doc-reader-fmt-first',
    'bootstrap-doc-reader-check-first', 'bootstrap-doc-reader-candidate-third',
    'bootstrap-doc-reader-msrv-third', 'bootstrap-doc-reader-producer-final',
    'bootstrap-doc-reader-fmt-final', 'bootstrap-doc-reader-check-final',
    'bootstrap-doc-reader-clippy-second', 'bootstrap-doc-reader-doc-check-first',
    'bootstrap-doc-reader-contract-check-first']}
EXITS.update({'bootstrap-continuation-first-targets': 101,
              'bootstrap-doc-reader-clippy-first': 101,
              'bootstrap-doc-reader-candidate-second': 101})


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
    require(index['schema'] == 'hc074-bootstrap-doc-reader-evidence-v1' and
            index['baseline_revision'] == BASE and index['expected_exit'] == EXITS, 'identity differs')
    require(all(index[key] is False for key in [
        'full_package_rerun', 'server', 'runtime_or_deadline_changes', 'qualification']), 'scope differs')
    require([index[key] for key in ['bootstrap_original_cases', 'preceding_proof_cases',
                                   'new_cases', 'unresolved_audit_inputs']] == [53, 10, 6, 26], 'case scope differs')
    members, intents = {}, {}
    for name, row in index['members'].items():
        require(re.fullmatch('[a-f0-9]{64}', row['sha256']), 'invalid digest')
        body = (folder / 'objects' / row['sha256']).read_bytes()
        require(sha(body) == row['sha256'] and len(body) == row['bytes'], 'raw member differs')
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
                intent['source']['Cargo.lock'] == 'd2c7736ebaa8381cece9162bbbf384db60c46111051e4e78e51d4bae12feaf5c',
                'dependency baseline differs')
        intents[label] = intent
    passing = [(value, 0) for value in [6, 4, 4, 4, 4, 4, 6, 3, 4, 4, 22, 4]]
    cohorts = {label: passing for label in ['bootstrap-doc-reader-candidate-first',
        'bootstrap-doc-reader-msrv-first', 'bootstrap-doc-reader-candidate-third', 'bootstrap-doc-reader-msrv-third']}
    cohorts['bootstrap-continuation-first-targets'] = [
        (2, 2), (3, 1), (3, 1), (2, 2), (3, 1), (2, 1), (3, 1), (3, 1), (17, 5), (0, 1)]
    for label, expected in cohorts.items():
        log = members['checks/' + label + '/stdout.log'].decode()
        observed = [(int(passed), int(failed)) for passed, failed in re.findall(
            r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; 0 ignored; 0 measured; 0 filtered out;', log)]
        require(observed == expected, 'cohort accounting differs')
    baseline = members['checks/bootstrap-continuation-first-targets/stdout.log'].decode()
    require(len(re.findall(r'^CURRENT_TREE_FRONTIER ', baseline, re.M)) == 26, 'audit frontier differs')
    require(b'file is loaded as a module multiple times' in
            members['checks/bootstrap-doc-reader-clippy-first/stderr.log'], 'first lint refusal missing')
    require(b'could not find `runtime` in the crate root' in
            members['checks/bootstrap-doc-reader-candidate-second/stderr.log'], 'first correction refusal missing')
    for name, body in members.items():
        if name.startswith('source-final/'):
            path = name.removeprefix('source-final/')
            for label in ['bootstrap-doc-reader-candidate-third', 'bootstrap-doc-reader-msrv-third',
                          'bootstrap-doc-reader-clippy-second']:
                require(sha(body) == intents[label]['source'][path], 'final source binding differs')
    rules_raw = members['source-final/tools/bootstrap-doc-reader-074/rules.json']
    require(sha(rules_raw) == RULES_SHA256, 'fixed caller rules differ')
    rules = json.loads(rules_raw)
    require(rules['baseline'] == BASE and len(rules['rows']) == 11 and
            len({row['path'] for row in rules['rows']}) == 11, 'caller scope differs')
    for row in rules['rows']:
        old = canonical(members['baseline/' + row['path']])
        require(sha(old) == row['before_sha256'], 'fixed caller predecessor differs')
        current = old.decode()
        for edit in row['edits']:
            require(edit['before'] and current.count(edit['before']) == edit['count'] > 0, 'caller edit differs')
            current = current.replace(edit['before'], edit['after'])
        current = (current + row['append']).encode()
        require(sha(current) == row['after_sha256'] and current ==
                canonical(members['source-final/' + row['path']]), 'whole caller differs')
    raw = members['source-final/tools/source-successor-view-074/rules.json']
    require(sha(raw) == SUCCESSOR_SHA256, 'independent successor rules differ')
    rows = json.loads(raw)['rows'][:2]
    require([row['path'] for row in rows] == ['docs/COMPAT.md', 'docs/testing/gated-test-registry.toml'], 'document scope differs')
    for row in rows:
        current = canonical(members['producer/' + row['revision'] + '/' + row['path']])
        previous = canonical(members['producer/' + row['previous_revision'] + '/' + row['path']])
        require(sha(current) == row['current_sha256'] and sha(previous) == row['previous_sha256'], 'producer differs')
        limit = len(current)
        for edit in reversed(row['patches']):
            at = edit['offset']
            exact = edit['current'].encode()
            end = at + len(exact)
            require(0 <= at <= end <= limit and current[at:end] == exact, 'inverse edit differs')
            current = current[:at] + edit['previous'].encode() + current[end:]
            limit = at
        require(current == previous, 'whole document predecessor differs')
    return dict(cases_per_final_compiler=69, unresolved_audit_inputs=26,
                full_package_rerun=False, code_executed=False, qualification=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
