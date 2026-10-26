"""Offline fixture repair and fixed-Job cost replay, without retained-code import."""
import hashlib
import json
from pathlib import Path
import re

INDEX = '1ba276dcf62f97801f69806ea5efba138c7d47425f30d2563874aa393681fcff'
SOURCE = '20247dd65f821aff33946b9643032c7773f0b146'
REFERENCE, PROFILE = 'reference_cpu_child', 'accounted_cpu_child'
SPEC = '3e9bcc06ebce3439495200162cb0c1900b8eda53:crates/xtask/tests/diagnostic_artifact_fixture_074.rs'
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def verify(folder):
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['source_commit'] == SOURCE and index['fixture_commit'] == '11557ac94c412226b5e13af2ce9713298800e079'
            and (index['cohorts'], index['git_calls'], index['first_baseline_failures']) == (1, 64, 2), 'scope differs')
    require(all(index[key] is False for key in ('job_overhead_measured', 'overhead_accepted', 'speedup_established',
                                               'package_pass', 'server')), 'index promoted')
    members = {}
    for name, row in index['members'].items():
        body = (folder/'objects'/row['sha256']).read_bytes()
        require(len(body) == row['bytes'] and sha(body) == row['sha256'], 'raw member differs: '+name)
        members[name] = body
    require(len(members) == 108, 'member count differs')
    def obj(name):
        return json.loads(members[name])
    checks = sorted(name.split('/')[1] for name in members if name.startswith('checks/') and name.endswith('/result.json'))
    require(len(checks) == 11, 'check catalog differs')
    for label in checks:
        before, after = (obj('checks/'+label+'/'+part) for part in ('intent.json', 'result.json'))
        require(after['exit'] == 0 and after['outer_refusal'] is None and after['source_unchanged'] is True and
                after['head_unchanged'] is True and before['source'] == after['source_after'], 'check failed/changed')
        require(before['bound_seconds'] == 1200 and before['disk_floor_bytes'] == 4*1024**3 and
                all(before[key] is False for key in ('dependency_changes', 'observation_frequency_changed',
                    'services_or_clocks', 'server', 'original_owner', 'qualification', 'retry')), 'check bounds differ')
    for label in ('pipe-fixture-footer-current-first', 'pipe-fixture-footer-msrv-first'):
        log = members['checks/'+label+'/stdout.log']
        require(b'test result: ok. 6 passed; 0 failed;' in log and b'HC074_PIPE ' not in log,
                'fixed fixture verdict/activation differs')
    for label, count in (('git-cpu-overhead-model-first', 13), ('git-cpu-overhead-model-precommit', 16),
                         ('git-cpu-overhead-accounting-regressions', 20)):
        log = members['checks/'+label+'/stderr.log']
        require(('Ran '+str(count)+' tests').encode() in log and log.rstrip().endswith(b'OK'), 'model verdict differs')
    baseline_raw = members['baseline/index.json']
    require(sha(baseline_raw) == 'c12ee4c7515ce6cd94744b7ce1754e172bbf7b2c7eba8f82893e4a3ab5defa41', 'baseline index differs')
    baseline = json.loads(baseline_raw)
    old_source = members['baseline/direct_output_cost_074.rs']
    for label in ('git-job-cpu-rust-current', 'git-job-cpu-msrv'):
        prefix = 'checks/'+label+'/'
        for part in ('intent.json', 'result.json', 'stdout.log', 'stderr.log'):
            require(sha(members['baseline/'+prefix+part]) == baseline['members'][prefix+part]['sha256'],
                    'first baseline changed')
        before = obj('baseline/'+prefix+'intent.json')
        require(sha(old_source) == before['source']['crates/xtask/tests/direct_output_cost_074.rs'], 'old source join differs')
        require(obj('baseline/'+prefix+'result.json')['exit'] == 101, 'first refusal lost')
        raw = members['baseline/'+prefix+'stdout.log']
        require(b'test result: FAILED. 4 passed; 1 failed;' in raw, 'first fixture verdict differs')
        streams = [bytes(int(m[0]) for m in re.finditer(rb'[0-9]+', line)) for line in raw.splitlines()
                   if line.startswith((b'  left: [', b' right: ['))]
        require(len(streams) == 2 and len(streams[0]) == len(streams[1]) == 1048719 and
                [i for i, (a, b) in enumerate(zip(*streams)) if a != b] == [1048715], 'baseline stdout delta differs')
    dispatch = obj('checks/git-cpu-overhead-native-first/intent.json')
    require(dispatch['head'] == SOURCE and dispatch['argv'][-2:] == ['git-cpu-overhead-build-receipt', 'first'],
            'committed native dispatch differs')
    for name, body in members.items():
        if name.startswith('source/'):
            require(sha(body) == dispatch['source'][name[7:]], 'source copy differs')
        elif name.startswith('raw-first-model/'):
            require(sha(body) == obj('checks/git-cpu-overhead-model-first/intent.json')['source'][
                'tools/xtask-package-attribution-074/'+name.split('/')[-1]], 'first model source differs')
    old, new = (body.decode().replace('\r\n', '\n') for body in
                (old_source, members['source/crates/xtask/tests/direct_output_cost_074.rs']))
    for line in ('assert_eq!(original.status, observed.status, "{mode}");',
                 'assert_eq!(original.stdout, observed.stdout, "{mode}");',
                 'assert_eq!(original.stderr, observed.stderr, "{mode}");'):
        require(line in old and line in new, 'old exact stream assertion lost')
    require('std::process::exit(0);' in new and 'independent_fixture_oracle_rejects_payload_status_and_footer_drift' in new,
            'fixture boundary/oracle repair lost')
    old_before = obj('baseline/checks/git-job-cpu-rust-current/intent.json')
    for name in ('scripts/ci/watchdog_windows_job.py', 'tools/xtask-package-attribution-074/pipe_capture.rs'):
        require(sha(members['source/'+name]) == old_before['source'][name], 'shared helper changed')
    intent, result = (obj('native/'+name) for name in ('intent.json', 'result.json'))
    schedule = [(b, s, mode) for b in range(4) for s, mode in enumerate(
        (REFERENCE, PROFILE, PROFILE, REFERENCE) if b % 2 == 0 else (PROFILE, REFERENCE, REFERENCE, PROFILE))]
    require(intent['schedule'] == [list(v) for v in schedule] and intent['calls_per_worker'] == 4 and
            (intent['bound_seconds'], intent['cleanup_seconds'], intent['free_bytes'], intent['log_bytes']) ==
            (120, 5, 4*1024**3, 16*1024**2), 'cohort schedule/bounds differ')
    require(all(intent[key] is False for key in ('job_overhead_measured', 'pure_counter_api_cost_isolated',
            'overhead_accepted', 'git_suspended', 'stdin_hold', 'native_bypass', 'environment_changes',
            'retry', 'package_pass', 'server')), 'intent scope promoted')
    require(result['passed'] is True and result['first_refusal'] is None and len(result['attempts']) == 16 and
            result['pins_before'] == result['pins_after'], 'native outcome/pins differ')
    build = obj('native/build.json')
    require(build['head'] == dispatch['head'] and build['source'] == dispatch['source'], 'build/source join differs')
    for attempt, (block, slot, mode) in zip(result['attempts'], schedule):
        require((attempt['block'], attempt['slot'], attempt['mode']) == (block, slot, mode) and
                attempt == obj(f'native/{block}-{slot}-result.json') and attempt['passed'] is True and
                attempt['first_refusal'] is None and attempt['cleanup_errors'] == [], 'attempt ledger differs')
        text = members[f'native/{block}-{slot}-{mode}.log'].decode()
        records = [json.loads(line[14:]) for line in text.splitlines() if line.startswith('HC074_GIT_CPU ')]
        done = [json.loads(line[19:]) for line in text.splitlines() if line.startswith('HC074_GIT_CPU_DONE ')]
        require(len(records) == 4 and records == attempt['records'] and
                'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out;' in text, 'worker log differs')
        require(done == [dict(calls=4, accounted=mode == PROFILE, git_argv=['git', 'show', SPEC], stdin_hold=False,
                git_suspended=False, native_bypass=False, environment_changes=False, package_pass=False)], 'worker protocol differs')
        for i, record in enumerate(records):
            require(record['call'] == i and record['spec'] == SPEC and record['stdout_bytes'] == 23589 and
                    record['stderr_bytes'] == 0 and record['code'] == 0, 'Git semantics differ')
        job = attempt['job']
        a, b = attempt['worker_initial'], attempt['worker']
        require(attempt['job_terminal_samples'] == [job, job] and job['active'] == 0 and job['total'] == 9 and
                job['terminated'] == 0 and a['terminal'] is False and b['terminal'] is True and
                a['creation_100ns'] == b['creation_100ns'] > 0 and b['exit_100ns'] >= b['creation_100ns'], 'lifetime/census differs')
        require(all(b[key] >= a[key] >= 0 for key in ('user_100ns', 'kernel_100ns')), 'worker CPU regressed')
        metrics = attempt['metrics']
        require(metrics['whole_job_cpu_ns'] == job['user_ns']+job['kernel_ns'] and
                metrics['whole_worker_cpu_ns'] == (b['user_100ns']+b['kernel_100ns'])*100 and
                all(type(v) is int and v >= 0 for v in metrics.values()) and metrics['wall_ns'] > 0, 'CPU metric differs')
        if mode == PROFILE:
            roots = [v['phases']['child_end'] for v in records]
            residual = {metric: job[metric]-b[field]*100-sum(v[metric] for v in roots)
                        for metric, field in (('user_ns', 'user_100ns'), ('kernel_ns', 'kernel_100ns'))}
            require(all(v >= 0 for v in residual.values()) and
                    attempt['native_inclusive']['non_launcher_descendant_cpu'] == residual and
                    attempt['native_inclusive']['exact_native_image_attested'] is False, 'descendant accounting differs')
        else:
            require(all(v['phases'] is None for v in records), 'reference acquired profile')
    analysis = result['analysis']
    require((analysis['workers'], analysis['git_calls']) == (16, 64), 'analysis count differs')
    require(all(analysis[key] is False for key in ('job_overhead_measured', 'pure_counter_api_cost_isolated',
                 'overhead_accepted', 'speedup_established', 'package_pass')), 'analysis promoted')
    for block in analysis['blocks']:
        data = result['attempts'][block['block']*4:block['block']*4+4]
        for metric, recorded in block['metrics'].items():
            x, y = (sum(v['metrics'][metric] for v in data if v['mode'] == mode) for mode in (REFERENCE, PROFILE))
            require(recorded == dict(reference_sum=x, profile_sum=y, delta=y-x,
                ratio_numerator=y if x else None, ratio_denominator=x if x else None,
                unavailable_reason=None if x else 'zero reference counter'), 'block estimator differs')
    return dict(git_calls=64, workers=16, models=36, fixture_passes_per_compiler=6,
                first_baseline_failures_preserved=2, overhead_accepted=False, package_pass=False, workload_replayed=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
