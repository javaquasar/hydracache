"""Offline raw-byte and lifetime-accounting replay; no retained code execution."""
import hashlib
import json
from pathlib import Path
import re

INDEX = 'c12ee4c7515ce6cd94744b7ce1754e172bbf7b2c7eba8f82893e4a3ab5defa41'
SOURCE = 'bd1b3cf2e6fc4a252037caf69603b0df6571aab4'
FIRST = 'e3f560e69a8d1deda20999a0b1c9b7e31a75f11d'
SPEC = '3e9bcc06ebce3439495200162cb0c1900b8eda53:crates/xtask/tests/diagnostic_artifact_fixture_074.rs'
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def protocol(raw, accounted):
    text = raw.decode()
    records = [json.loads(line[14:]) for line in text.splitlines() if line.startswith('HC074_GIT_CPU ')]
    done = [json.loads(line[19:]) for line in text.splitlines() if line.startswith('HC074_GIT_CPU_DONE ')]
    require(done == [dict(calls=4, accounted=accounted, git_argv=['git', 'show', SPEC],
        stdin_hold=False, git_suspended=False, native_bypass=False, environment_changes=False,
        package_pass=False)], 'worker scope differs')
    require(len(records) == 4 and 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out;' in text,
            'worker verdict differs')
    for call, row in enumerate(records):
        require(row['call'] == call and row['accounted'] is accounted and row['spec'] == SPEC and
                row['stdout_bytes'] == 23589 and row['stderr_bytes'] == 0 and row['code'] == 0,
                'Git bytes/status/schedule differs')
        if not accounted:
            require(row['phases'] is None, 'reference acquired profile')
    return records


def verify(folder):
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['source_commit'] == SOURCE and index['first_source_commit'] == FIRST and
            (index['native_attempts'], index['first_attempt_git_calls'], index['changed_source_git_calls']) == (2, 4, 8),
            'attempt scope differs')
    require(all(index[key] is False for key in ('package_pass', 'overhead_accepted', 'speedup_established', 'server')),
            'index scope promoted')
    members = {}
    for name, entry in index['members'].items():
        body = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(body) == entry['bytes'] and sha(body) == entry['sha256'], 'raw member differs: '+name)
        members[name] = body
    require(len(members) == 97, 'member count differs')
    def obj(name):
        return json.loads(members[name])
    labels = sorted(name.split('/')[1] for name in members if name.startswith('checks/') and name.endswith('/result.json'))
    require(len(labels) == 17, 'check catalog differs')
    for label in labels:
        before, after = (obj('checks/'+label+'/'+name) for name in ('intent.json', 'result.json'))
        expected = 101 if label in ('git-job-cpu-rust-current', 'git-job-cpu-msrv') else 1 if label == 'git-job-cpu-native-first' else 0
        require(after['exit'] == expected and after['outer_refusal'] is None and
                after['source_unchanged'] is True and after['head_unchanged'] is True and
                before['source'] == after['source_after'], 'check outcome/source differs')
        require(before['bound_seconds'] == 1200 and before['disk_floor_bytes'] == 4*1024**3 and
                all(before[key] is False for key in ('dependency_changes', 'observation_frequency_changed',
                    'services_or_clocks', 'server', 'original_owner', 'qualification', 'retry')),
                'check limits differ')
    for label, count in (('git-job-cpu-model-first', 10), ('git-job-cpu-model-expanded', 16),
                         ('git-job-cpu-model-precommit', 16), ('git-job-cpu-live-identity-models', 20),
                         ('git-job-cpu-python-controls', 60)):
        stderr = members['checks/'+label+'/stderr.log'].decode()
        require('Ran '+str(count)+' tests' in stderr and stderr.rstrip().endswith('OK'), 'model verdict differs')
    for label in ('git-job-cpu-new-current', 'git-job-cpu-new-msrv'):
        stdout = members['checks/'+label+'/stdout.log'].decode()
        require('test result: ok. 4 passed; 0 failed;' in stdout and 'HC074_GIT_CPU ' not in stdout,
                'ordinary target activated diagnostic or differs')
    audit = obj('offline/first-pipe-fixture-difference.json')
    for entry in audit:
        raw = members['checks/'+entry['label']+'/stdout.log']
        require(b'test result: FAILED. 4 passed; 1 failed;' in raw, 'first pipe verdict differs')
        streams = [bytes(int(m[0]) for m in re.finditer(rb'[0-9]+', line))
                   for line in raw.splitlines() if line.startswith((b'  left: [', b' right: ['))]
        require(len(streams) == 2 and [len(v) for v in streams] == entry['lengths'] == [1048719, 1048719],
                'first pipe output length differs')
        require([i for i, (a, b) in enumerate(zip(*streams)) if a != b] == entry['differing_offsets'] == [1048715] and
                [v[-120:].decode() for v in streams] == entry['tails'] and
                {re.search(rb'finished in ([0-9.]+)s', v)[1] for v in streams} == {b'0.00', b'0.01'},
                'first pipe elapsed-only difference differs')
    initial = obj('checks/git-job-cpu-native-first/intent.json')
    final = obj('checks/git-job-cpu-native-live-identity/intent.json')
    require(initial['head'] == FIRST and final['head'] == SOURCE and
            initial['argv'][-2:] == ['git-job-cpu-build-receipt', 'first'] and
            final['argv'][-2:] == ['git-job-cpu-fixed-build-receipt', 'live-identity'], 'dispatch source differs')
    for name, body in members.items():
        if name.startswith('source/'):
            require(sha(body) == final['source'][name[7:]], 'source copy differs')
        elif name.startswith('raw-before-live-identity/'):
            require(sha(body) == initial['source']['tools/xtask-package-attribution-074/'+name.split('/')[-1]],
                    'first native source differs')
        elif name.startswith('raw-first-model/'):
            require(sha(body) == obj('checks/git-job-cpu-model-first/intent.json')['source'][
                    'tools/xtask-package-attribution-074/'+name.split('/')[-1]], 'first model source differs')
    for label in ('first', 'live-identity'):
        intent = obj('native/'+label+'/intent.json')
        require((intent['bound_seconds'], intent['cleanup_seconds'], intent['calls_per_mode'], intent['free_bytes'],
                 intent['log_bytes']) == (120, 5, 4, 4*1024**3, 16*1024**2) and
                intent['modes'] == ['reference_cpu_child', 'accounted_cpu_child'] and
                all(intent[key] is False for key in ('retry', 'git_suspended', 'stdin_hold', 'native_bypass',
                    'environment_changes', 'job_limits', 'job_kill_on_close', 'package_pass', 'server')),
                'native bounds/scope differs')
        build = obj('native/'+label+'/build.json')
        wrapper = initial if label == 'first' else final
        require(build['head'] == wrapper['head'] and build['source'] == wrapper['source'], 'build join differs')
    first = obj('native/first/result.json')
    require(first['passed'] is False and first['first_refusal'] ==
            'Refusal: QueryFullProcessImageNameW: Win32 error 31' and first['cleanup_errors'] == [] and first['modes'] == [] and
            'native/first/accounted_cpu_child.log' not in members, 'first native refusal differs')
    protocol(members['native/first/reference_cpu_child.log'], False)
    result = obj('native/live-identity/result.json')
    require(result['passed'] is True and result['first_refusal'] is None and result['cleanup_errors'] == [] and
            result['pins_before'] == result['pins_after'] == first['pins_before'] and len(result['pins_before']) == 2,
            'changed-source native outcome/pins differs')
    require(all(result[key] is False for key in ('package_pass', 'speedup_established', 'overhead_accepted', 'server')),
            'native scope promoted')
    require([v['mode'] for v in result['modes']] == ['reference_cpu_child', 'accounted_cpu_child'], 'mode order differs')
    for mode in result['modes']:
        accounted = mode['mode'] == 'accounted_cpu_child'
        records = protocol(members['native/live-identity/'+mode['mode']+'.log'], accounted)
        require(records == mode['records'], 'raw worker/result join differs')
        job = mode['job']
        require(mode['job_terminal_samples'] == [job, job] and job['active'] == 0 and job['total'] == 9 and
                job['terminated'] == 0, 'terminal Job census/counter differs')
        start, end = mode['worker_initial'], mode['worker']
        require(start['terminal'] is False and start['exit_100ns'] is None and end['terminal'] is True and
                end['exit_100ns'] >= end['creation_100ns'] == start['creation_100ns'] > 0 and mode['worker_pid'] > 0,
                'worker original lifetime differs')
        require(all(end[key] >= start[key] >= 0 for key in ('user_100ns', 'kernel_100ns')) and
                all(end['io'][key] >= start['io'][key] >= 0 for key in start['io']), 'worker counter drift')
        if not accounted:
            continue
        roots = []
        for record in records:
            a, b = (record['phases'][key] for key in ('child_start', 'child_end'))
            require(a['pid'] == b['pid'] != mode['worker_pid'] and a['created_100ns'] == b['created_100ns'] and
                    start['creation_100ns'] <= b['created_100ns'] <= b['exited_100ns'] <= end['exit_100ns'] and
                    all(b[key] >= a[key] >= 0 for key in ('user_ns', 'kernel_ns')), 'launcher lifetime differs')
            roots.append(b)
        require(len({(v['pid'], v['created_100ns']) for v in roots}) == 4, 'launcher identities not unique')
        residual = {metric: job[metric]-end[field]*100-sum(v[metric] for v in roots)
                    for metric, field in (('user_ns', 'user_100ns'), ('kernel_ns', 'kernel_100ns'))}
        require(all(v >= 0 for v in residual.values()) and residual == dict(user_ns=109375000, kernel_ns=109375000),
                'native-inclusive accounting differs')
        require(mode['analysis'] == dict(non_launcher_descendant_cpu=residual, cpu_includes_native=True,
            exact_native_image_attested=False, per_call_native_cpu=False, initial_sample_subtracted=False,
            overhead_accepted=False, speedup_established=False, package_pass=False), 'accounting scope differs')
    return dict(native_attempts=2, changed_source_git_calls=8, first_refusal_preserved=True,
        non_launcher_descendant_cpu_ns=218750000, exact_native_image_attested=False,
        models=80, old_pipe_first_failures=2, package_pass=False, workload_replayed=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
