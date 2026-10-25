"""Verify retained source edits/check results without executing retained code or Git."""
import hashlib
import json
from pathlib import Path
import re

INDEX_SHA256 = 'fc8a4d25330d0a4b4701ba072ac55bebee67d9f40e9450b0b9a301913cf1c215'
RULES_SHA256 = '61d3e1941cdae497600592a49af0487e4cf545573ba3f091add173110b560cda'
EXITS = {
    'successor-view-first-red': 101, 'successor-view-candidate-first': 101,
    'successor-view-candidate-second': 0, 'successor-view-producer-first': 0,
    'successor-view-msrv-first': 0, 'successor-view-current-final': 0,
    'successor-view-msrv-artifact-first': 0,
    'successor-view-check-first': 0, 'successor-view-clippy-first': 0,
    'successor-view-fmt-first': 0, 'successor-view-doc-check-first': 0,
    'successor-view-contract-check-first': 0,
}
PATHS = ['docs/COMPAT.md', 'docs/testing/gated-test-registry.toml', *[
    'tools/long-run-supervisor-074/src/' + name for name in (
        'diagnostic_authority_fixed.rs', 'diagnostic_authority_fixture.rs',
        'diagnostic_kernel_origin.rs')]]


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(raw):
    return raw.decode('utf-8').replace('\r\n', '\n').encode('utf-8')


def verify(folder):
    index_raw = (folder / 'index.json').read_bytes()
    require(sha(index_raw) == INDEX_SHA256, 'fixed index differs')
    index = json.loads(index_raw)
    require(index['schema'] == 'hc074-source-successor-evidence-v1', 'schema differs')
    require(index['source_only'] is True and index['full_package_rerun'] is False and
            index['qualification'] is False and index['server'] is False, 'scope differs')
    require(index['expected_exit'] == EXITS, 'first outcomes differ')
    members = {}
    for name, row in index['members'].items():
        digest = row['sha256']
        require(re.fullmatch('[a-f0-9]{64}', digest), 'invalid digest')
        raw = (folder / 'objects' / digest).read_bytes()
        require(sha(raw) == digest and len(raw) == row['bytes'], 'raw member differs')
        members[name] = raw
    for label, expected in EXITS.items():
        base = 'checks/' + label + '/'
        intent = json.loads(members[base + 'intent.json'])
        result = json.loads(members[base + 'result.json'])
        require(result['exit'] == expected and result['outer_refusal'] is None, 'check outcome differs')
        require(result['source_unchanged'] is True and result['head_unchanged'] is True and
                intent['source'] == result['source_after'], 'check source changed')
    cohorts = {'successor-view-candidate-second': [20, 5],
               'successor-view-msrv-first': [9, 5, 5],
               'successor-view-current-final': [9, 5, 5],
               'successor-view-msrv-artifact-first': [20]}
    for label, expected in cohorts.items():
        log = members['checks/' + label + '/stdout.log'].decode()
        observed = [int(value) for value in re.findall(
            r'test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;', log)]
        require(observed == expected and '... FAILED' not in log, 'case accounting differs')
    require(b'0 passed; 1 failed' in members['checks/successor-view-first-red/stdout.log'],
            'first canary refusal missing')
    require(b'error[E0277]' in members['checks/successor-view-candidate-first/stderr.log'],
            'first compiler refusal missing')
    rules_raw = members['source-final/tools/source-successor-view-074/rules.json']
    require(sha(rules_raw) == RULES_SHA256, 'independent rules differ')
    rules = json.loads(rules_raw)
    require([row['path'] for row in rules['rows']] == PATHS, 'five-path scope differs')
    edits = 0
    for row in rules['rows']:
        path = row['path']
        current = canonical(members['producer/' + row['revision'] + '/' + path])
        previous = canonical(members['producer/' + row['previous_revision'] + '/' + path])
        require(sha(current) == row['current_sha256'] and sha(previous) == row['previous_sha256'],
                'fixed producer body differs')
        limit = len(current)
        for patch in reversed(row['patches']):
            at = patch['offset']
            exact, prior = patch['current'].encode(), patch['previous'].encode()
            end = at + len(exact)
            require(0 <= at <= end <= limit and current[at:end] == exact, 'inverse edit differs')
            current = current[:at] + prior + current[end:]
            limit = at
            edits += 1
        require(current == previous, 'whole predecessor differs')
    require(edits == 38, 'edit count differs')
    return dict(paths=5, edits=38, artifact_tests_each=20, new_tests_each=5,
                inventory_tests_each=14, full_package_rerun=False, code_executed=False,
                qualification=False)


if __name__ == '__main__':
    print(json.dumps(verify(Path(__file__).resolve().parent), indent=2, sort_keys=True))
