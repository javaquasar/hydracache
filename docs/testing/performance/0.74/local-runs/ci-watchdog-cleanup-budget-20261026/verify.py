"""Offline late-witness regression replay; executes no captured command."""
import hashlib
import json
from pathlib import Path
import re

INDEX_SHA256 = 'cf57df5264e4a33ce629111d07e578373beec1995ba2cb3f01af5001d53822aa'
BASE = 'd9941796dc8766bcc67b2472b58abe94b6120855'
RUNTIME = '4fe6cd53a04e0d1e24cb60287fdad5317d1f20bc5472881b3bab28e37b2a965c'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def check(index, members):
    require(index['schema'] == 'hc074-ci-watchdog-cleanup-budget-v1' and
            index['baseline_revision'] == BASE and index['previous_index_sha256'] ==
            '455bbaebec68d634de633811fa7a94135366d7e2271408f9721f3f17b665d36c', 'fixed identity')
    require([index[k] for k in ['red_passed', 'red_failed', 'final_units', 'native_cases',
            'each_compiler_old_cases', 'original_timeout', 'cleanup_bound']] == [17, 2, 19, 6, 12, 0.35, 5],
            'scope/bounds')
    require(all(index[k] is False for k in ['full_package', 'server', 'qualification']), 'admission')
    require(set(index['members']) == set(members), 'member set')
    intents = {}
    for label in ['ci-watchdog-budget-red-first', *index['success_labels']]:
        intent = json.loads(members['checks/' + label + '/intent.json'])
        result = json.loads(members['checks/' + label + '/result.json'])
        require(result['exit'] == (1 if label.endswith('red-first') else 0) and
                result['outer_refusal'] is None and result['source_unchanged'] is True and
                result['head_unchanged'] is True and intent['head'] == BASE and
                intent['source'] == result['source_after'], 'first outcome/source binding')
        require(intent['bound_seconds'] == 1200 and intent['disk_floor_bytes'] == 4 * 1024**3 and
                all(intent[k] is False for k in ['dependency_changes', 'services_or_clocks', 'server',
                    'original_owner', 'qualification', 'retry']), 'protected guards')
        intents[label] = intent
    red = members['checks/ci-watchdog-budget-red-first/stderr.log']
    require(b'Ran 19 tests' in red and b'FAILED (failures=2)' in red and
            b'test_empty_witness_after_cleanup_budget_is_not_success' in red and
            b'test_close_after_cleanup_budget_is_not_success' in red, 'first late-witness failures')
    for label, count in [('ci-watchdog-budget-units-first', 19), ('ci-watchdog-budget-native-first', 6)]:
        log = members['checks/' + label + '/stderr.log']
        require(re.search(rb'Ran ' + str(count).encode() + rb' tests in ', log) and
                log.rstrip().endswith(b'OK') and b'skipped' not in log, 'complete finite cohort')
    for label in ['ci-watchdog-budget-current', 'ci-watchdog-budget-msrv']:
        log = members['checks/' + label + '/stdout.log']
        require(b'12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' in log and
                b'test watchdog_times_out_and_kills_descendant_processes ... ok' in log, 'old marker/case verdicts')
    for name, raw in members.items():
        if name.startswith('source/'):
            path = name.removeprefix('source/')
            require(all(sha(raw) == intents[label]['source'][path] for label in index['success_labels']),
                    'common final source')
    require(sha(members['source/scripts/ci/run-with-heartbeat.py']) == RUNTIME and
            sha(members['source/scripts/ci/watchdog_windows_job.py']) ==
            '8b392811e106eb7e692e21ee5b55cd04ea8ac421cad015edb66ce736b08c2ab1', 'fixed runtime')
    before = members['before/run-with-heartbeat.py']
    require(sha(before) == 'f89594b5e44da2c0398e42a522fecad9cb0cca90a3888d9a3879971013bd9a58' and
            sha(before) == intents['ci-watchdog-budget-red-first']['source']['scripts/ci/run-with-heartbeat.py'],
            'exact preceding runtime')
    expected = before.decode().replace('\r\n', '\n')
    hook = '                if process is not None:\n                    process.wait(timeout=max(0, cleanup_deadline - time.monotonic()))\n'
    require(expected.count(hook) == 1, 'first insertion boundary')
    expected = expected.replace(hook, hook + '                if time.monotonic() >= cleanup_deadline:\n'
        '                    raise RuntimeError("owned CI Job cleanup exceeded five seconds")\n')
    hook = '            if failures:\n'
    require(expected.count(hook) == 1, 'last insertion boundary')
    expected = expected.replace(hook, '            if time.monotonic() >= cleanup_deadline:\n'
        '                failures.append("owned CI Job cleanup exceeded five seconds")\n' + hook)
    require(expected == members['source/scripts/ci/run-with-heartbeat.py'].decode().replace('\r\n', '\n'),
            'only two deadline checks')
    test = members['first-red-test.py']
    require(test == members['source/scripts/ci/test_watchdog_074.py'] and sha(test) ==
            intents['ci-watchdog-budget-red-first']['source']['scripts/ci/test_watchdog_074.py'], 'same red/green tests')
    native = [name for name in members if name.startswith('native/') and name.endswith('/result.json')]
    require(len(native) == 6, 'native case set')
    for name in native:
        result = json.loads(members[name])
        require(result['pass'] is True and all(result[k] is False for k in ['server', 'product', 'retry']), 'native result')
        if 'identities' not in result:
            continue
        require(len(result['identities']) == len(result['before']) == len(result['terminal']) ==
                len(result['cleanup_objects']), 'held object scope')
        for identity, before_row, terminal, cleanup in zip(result['identities'], result['before'],
                                                           result['terminal'], result['cleanup_objects']):
            require(before_row['terminal'] is False and terminal['terminal'] is True and cleanup['terminal'] is True and
                    identity['creation_100ns'] == before_row['creation_100ns'] == terminal['creation_100ns'] ==
                    cleanup['creation_100ns'], 'held original lifetime')
        require(result['post_terminal_sizes'][0] == result['post_terminal_sizes'][1], 'post-terminal write growth')


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
    verify(Path(__file__).resolve().parent)
    print('Verified: first 17/2 refusal, unchanged 19-case tests, six native controls and old marker cases per compiler.')
