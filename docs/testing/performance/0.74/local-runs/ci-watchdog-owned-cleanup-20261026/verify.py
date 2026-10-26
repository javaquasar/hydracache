"""Offline raw-source/first-outcome verification, never execution of captured code."""
import hashlib
import json
from pathlib import Path
import re

INDEX_SHA256 = '455bbaebec68d634de633811fa7a94135366d7e2271408f9721f3f17b665d36c'
BASE = 'e2b96d64fd85419b5a2be837b9b7336f51b39ed5'
RUNTIME = {'scripts/ci/run-with-heartbeat.py': 'f89594b5e44da2c0398e42a522fecad9cb0cca90a3888d9a3879971013bd9a58',
           'scripts/ci/watchdog_windows_job.py': '8b392811e106eb7e692e21ee5b55cd04ea8ac421cad015edb66ce736b08c2ab1'}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def check(index, members):
    require(index['schema'] == 'hc074-ci-watchdog-owned-cleanup-evidence-v1' and
            index['baseline_revision'] == BASE, 'fixed identity')
    require([index[k] for k in ['first_red_failed_subtests', 'final_unit_cases', 'each_compiler_old_cases',
            'first_native_cases', 'separate_stream_case', 'corrected_unsupported_case', 'posix_cases']] ==
            [9, 17, 12, 5, 1, 1, 4], 'coverage scope')
    require(all(index[k] is False for k in ['first_red_scripts_inventoried', 'full_package_rerun',
            'server', 'product', 'qualification', 'original_marker_assertion_changed']) and
            index['original_marker_timeout'] == 0.35, 'admission/deadline scope')
    require(set(index['members']) == set(members), 'member scope')
    intents = {}
    for label in ['ci-watchdog-units-red-first', *index['success_labels']]:
        intent = json.loads(members['checks/' + label + '/intent.json'])
        result = json.loads(members['checks/' + label + '/result.json'])
        require(result['exit'] == (1 if label.endswith('red-first') else 0) and
                result['outer_refusal'] is None and result['source_unchanged'] is True and
                result['head_unchanged'] is True and intent['head'] == BASE and
                intent['source'] == result['source_after'], 'first outcome/source stability')
        require(intent['bound_seconds'] == 1200 and intent['disk_floor_bytes'] == 4 * 1024**3 and
                all(intent[k] is False for k in ['dependency_changes', 'services_or_clocks', 'server',
                    'original_owner', 'qualification', 'retry']), 'protected bounds')
        intents[label] = intent
    require(b'Ran 7 tests' in members['checks/ci-watchdog-units-red-first/stderr.log'] and
            b'FAILED (failures=9)' in members['checks/ci-watchdog-units-red-first/stderr.log'], 'first red result')
    for label, count in [('ci-watchdog-units-candidate-first', 7), ('ci-watchdog-units-final', 13),
                         ('ci-watchdog-units-complete', 17), ('ci-watchdog-native-first', 5),
                         ('ci-watchdog-streams-first', 1), ('ci-watchdog-unsupported-first', 1)]:
        log = members['checks/' + label + '/stderr.log']
        require(re.search(rb'Ran ' + str(count).encode() + rb' tests? in ', log) and
                log.rstrip().endswith(b'OK') and b'skipped' not in log, 'finite test coverage')
    for label in ['ci-watchdog-old-cases-first', 'ci-watchdog-current-final', 'ci-watchdog-msrv-final']:
        log = members['checks/' + label + '/stdout.log']
        require(b'12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' in log and
                b'test watchdog_times_out_and_kills_descendant_processes ... ok' in log, 'old case/marker result')
    for path, digest in RUNTIME.items():
        require(sha(members['source-final/' + path]) == digest, 'exact runtime bytes')
        for label in ['ci-watchdog-native-first', 'ci-watchdog-current-final', 'ci-watchdog-msrv-final',
                      'ci-watchdog-check-final', 'ci-watchdog-clippy-final', 'ci-watchdog-streams-first',
                      'ci-watchdog-unsupported-first', 'ci-watchdog-posix-first']:
            require(intents[label]['source'][path] == digest, 'common runtime binding')
    for prefix, label in [('initial-candidate/', 'ci-watchdog-units-candidate-first'),
                          ('native-first-source/', 'ci-watchdog-native-first'),
                          ('streams-source/', 'ci-watchdog-streams-first'),
                          ('source-final/', 'ci-watchdog-unsupported-first')]:
        for name, raw in members.items():
            if name.startswith(prefix):
                require(sha(raw) == intents[label]['source'][name.removeprefix(prefix)], 'retained stage source')
    require(sha(members['before/run-with-heartbeat.py']) ==
            'f42cda4480a8c29c649954f838ddda3425a163e062a1016cbad612a393a7f53c', 'old helper')
    require(sha(members['protected/docs/testing/performance/0.74/qualification-manifest.toml']) ==
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification frozen')
    cases = ['test_owner_loss_before_resume_terminates_original_suspended_child',
             'test_owner_loss_terminates_running_root_and_writer',
             'test_successful_root_leaving_writer_is_timeout_not_success']
    for name in cases:
        result = json.loads(members['native/' + name + '/result.json'])
        require(result['pass'] is True and all(result[k] is False for k in ['server', 'product', 'retry']), 'native result')
        require(len(result['identities']) == len(result['before']) == len(result['terminal']) ==
                len(result['cleanup_objects']) == (2 if 'before_resume' in name else 3), 'held scope')
        for identity, before, terminal, cleanup in zip(result['identities'], result['before'],
                                                      result['terminal'], result['cleanup_objects']):
            require(before['terminal'] is False and terminal['terminal'] is True and cleanup['terminal'] is True and
                    identity['creation_100ns'] == before['creation_100ns'] == terminal['creation_100ns'] ==
                    cleanup['creation_100ns'] and terminal['exit_100ns'] == cleanup['exit_100ns'] >
                    identity['creation_100ns'], 'original object termination')
        sizes = result['post_terminal_sizes']
        require(len(sizes) == 2 and sizes[0] == sizes[1], 'post-terminal write growth')
        if 'before_resume' not in name:
            require(result['before'][-1]['io']['write_bytes'] > 0 and sizes[0] > 0, 'running writer not observed')
    for name in ['test_post_create_setup_refusal_never_executes_child',
                 'test_unsupported_job_list_refuses_without_creating_child']:
        result = json.loads(members['native/' + name + '/result.json'])
        require(result['pass'] is True, 'first setup refusal control')
    corrected = json.loads(members['unsupported/test_unsupported_job_list_refuses_without_creating_child/result.json'])
    require(corrected['pass'] is True and 'WinError 50' in corrected['refusal'] and
            'JOB_LIST' in corrected['refusal'], 'corrected exact error injection')
    streams = json.loads(members['streams/test_cli_cwd_unicode_arguments_and_inherited_streams/result.json'])
    require(streams['pass'] is True and streams['controller_exit'] == 0, 'stream control')
    receipt = json.loads(members['native/' + cases[2] + '/receipt.json'])
    require(receipt['classification'] == 'timeout' and receipt['exit_code'] == 0, 'leader exit not whole-tree success')
    posix = json.loads(members['posix/result.json'])
    require(posix['marker_absent'] is True and len(posix['cases']) == 4 and
            all(row['pass'] is True for row in posix['cases']), 'POSIX listed compatibility paths')


def verify(folder):
    raw = (folder / 'index.json').read_bytes()
    require(sha(raw) == INDEX_SHA256, 'fixed index')
    index = json.loads(raw)
    members = {}
    for name, row in index['members'].items():
        require(re.fullmatch('[a-f0-9]{64}', row['sha256']), 'object name')
        raw = (folder / 'objects' / row['sha256']).read_bytes()
        require(sha(raw) == row['sha256'] and len(raw) == row['bytes'], 'raw member')
        members[name] = raw
    check(index, members)
    return index, members


if __name__ == '__main__':
    verify(Path(__file__).resolve().parent)
    print('Verified: unchanged old cases, separate native/stream/error controls, first results; no package/server admission.')
