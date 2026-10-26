"""Offline receipt and fixed source-delta replay; never executes retained code."""
import hashlib
import json
from pathlib import Path
import re

INDEX_SHA256 = '8022ce3470effdd0f4c1df62747cf076f2c42c5420d6b3146b08c70534a03417'
RULES_SHA256 = '9373e36370e3893316a983fb159debba29afbbf4095c86f8f71d4384c80cc4f5'
BASE = '628bc3c113ce12e9ecb10ee4daee6961a6b2ad15'
EXITS = {'authority-continuation-first-targets': 101, 'authority-reader-phase-candidate-first': 101,
         'authority-reader-phase-candidate-second': 0, 'authority-reader-phase-msrv-first': 0,
         'authority-reader-phase-current-guards': 0, 'authority-reader-phase-producer-first': 0,
         'authority-reader-phase-check-first': 0, 'authority-reader-phase-clippy-first': 0,
         'authority-reader-phase-fmt-first': 0, 'authority-reader-phase-doc-check-first': 0,
         'authority-reader-phase-contract-check-first': 0}
BASELINES = {
    'authority_hex_reader_074': 'b9ecbcccb582f784ec99787d05ecf7e88b51991bc3fc102d519ba3190d668090',
    'owned_case_074': '2a5def6be0bef44b32ffb177540b9b98b100464c995b997db4a69572e3b17239',
    'historical_input_view_074': '5a020d1c4c98f338bb1c2b9182bc7bb139363cbfaf643ea0ab996f6541d28261',
    'authority_runtime_074': 'fcc4472d1ffa648449913bbe3782c8065ff3d606511ffd4a936d223b3725c48e',
    'current_tree_reader_074': '7d76db82b03761e933c407021454ccf717989e553310f0269d27d27cdd0e7973'}


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
    require(index['schema'] == 'hc074-authority-reader-phase-evidence-v1' and
            index['baseline_revision'] == BASE, 'identity differs')
    require(index['full_package_rerun'] is False and index['server'] is False and
            index['runtime_or_deadline_changes'] is False and index['qualification'] is False, 'scope differs')
    require(index['expected_exit'] == EXITS, 'first outcomes differ')
    members, intents = {}, {}
    for name, row in index['members'].items():
        require(re.fullmatch('[a-f0-9]{64}', row['sha256']), 'invalid digest')
        body = (folder / 'objects' / row['sha256']).read_bytes()
        require(sha(body) == row['sha256'] and len(body) == row['bytes'], 'raw member differs')
        members[name] = body
    for label, expected in EXITS.items():
        base = 'checks/' + label + '/'
        intent = json.loads(members[base + 'intent.json'])
        result = json.loads(members[base + 'result.json'])
        require(result['exit'] == expected and result['outer_refusal'] is None, 'check outcome differs')
        require(result['source_unchanged'] is True and result['head_unchanged'] is True and
                result['source_after'] == intent['source'], 'check source changed')
        intents[label] = intent
    positive = [(2, 0), (2, 0), (2, 0), (6, 0), (5, 0), (4, 0), (3, 0), (4, 0)]
    cohorts = {'authority-continuation-first-targets': [(2, 0), (1, 1), (2, 0), (3, 2), (4, 0), (2, 1), (3, 1)],
               'authority-reader-phase-candidate-first': [(2, 0), (2, 0), (2, 0), (5, 0), (3, 2), (4, 0), (3, 0), (4, 0)],
               'authority-reader-phase-candidate-second': positive,
               'authority-reader-phase-msrv-first': positive + [(5, 0)],
               'authority-reader-phase-current-guards': [(5, 0)]}
    for label, expected in cohorts.items():
        log = members['checks/' + label + '/stdout.log'].decode()
        observed = [(int(passed), int(failed)) for passed, failed in re.findall(
            r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; 0 ignored; 0 measured; 0 filtered out;', log)]
        require(observed == expected, 'cohort accounting differs')
    for name, body in members.items():
        if name.startswith('source-final/'):
            path = name.removeprefix('source-final/')
            require(sha(body) == intents['authority-reader-phase-current-guards']['source'][path] ==
                    intents['authority-reader-phase-msrv-first']['source'][path], 'final source binding differs')
            if not path.endswith('/README.md'):
                require(sha(body) == intents['authority-reader-phase-candidate-second']['source'][path],
                        'current cohort source binding differs')
    for name, digest in BASELINES.items():
        path = 'crates/xtask/tests/' + name + '.rs'
        require(sha(canonical(members['baseline/' + path])) == digest, 'independent old reader differs')
    rules_raw = members['source-final/tools/authority-reader-phase-074/main-rules.json']
    require(sha(rules_raw) == RULES_SHA256, 'independent terminal rules differ')
    rules = json.loads(rules_raw)
    require(len(rules['rows']) == 1, 'terminal scope differs')
    row = rules['rows'][0]
    current = canonical(members['producer/' + row['revision'] + '/' + row['path']])
    previous = canonical(members['producer/' + row['previous_revision'] + '/' + row['path']])
    require(sha(current) == row['current_sha256'] and sha(previous) == row['previous_sha256'], 'producer differs')
    require(len(row['patches']) == 5, 'terminal edit count differs')
    limit = len(current)
    for edit in reversed(row['patches']):
        at = edit['offset']
        exact = edit['current'].encode()
        end = at + len(exact)
        require(0 <= at <= end <= limit and current[at:end] == exact, 'inverse edit differs')
        current = current[:at] + edit['previous'].encode() + current[end:]
        limit = at
    require(current == previous, 'whole predecessor differs')
    binding = json.loads(members['derived-original-context-binding.json'])
    excerpt = members['derived-original-context-section.raw']
    require(binding['excerpt_sha256'] == sha(excerpt) and binding['bytes'] == len(excerpt) and
            binding['derived_excerpt_not_new_run'] is True and
            binding['original_index_sha256'] == '0b0b568cc495e0a24f21f6684dbc6d471e3e3d1c1240fce002958975ebd31a9a' and
            binding['stdout_sha256'] == 'c6bcd489b137ab88eb8bf01f9a3289c7e5324a5cf3c8b3e83169e5c770a62fd6', 'original excerpt binding differs')
    require(b'unreviewed input cannot enter historical assertions: docs/testing/gated-test-registry.toml' in excerpt,
            'original final registry refusal missing')
    return dict(original_cases=22, new_cases=6, unchanged_successor_controls=5,
                current_and_msrv_distinct_cases_each=33, full_package_rerun=False,
                code_executed=False, qualification=False)


if __name__ == '__main__':
    print(json.dumps(verify(Path(__file__).resolve().parent), indent=2, sort_keys=True))
