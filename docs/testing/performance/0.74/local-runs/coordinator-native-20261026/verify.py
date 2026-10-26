"""Verify retained native brackets and joins without launching retained code."""
import hashlib
import json
from pathlib import Path

INDEX = '55b802621e147681f9478a164053a04c055ec28d8d35227d68d49ad0b4de380f'
SOURCE = '1030acb263e96d1e023f852dceb021de458ce6f4'
PHASES = ('setup', 'spawn', 'wait', 'terminal_reads', 'decode', 'close')
REFERENCE, PROFILE = 'reference_cpu_child', 'accounted_cpu_child'
SPEC = '3e9bcc06ebce3439495200162cb0c1900b8eda53:crates/xtask/tests/diagnostic_artifact_fixture_074.rs'
PINS = {
    r'C:\Program Files\Git\cmd\git.exe': '34a408843194be320d8a87a3c12cd5c7d2e08d03b24567a41db32e21d12569d2',
    r'C:\Program Files\Git\mingw64\bin\git.exe': '755d4896d35663d0ff08924f84507f35236b83d240635b512c519bf43cc71a87',
}
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def load(folder):
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['source_commit'] == SOURCE and index['native_attempts'] == 1 and
            index['native_control_executed'] is True and index['workers'] == 16 and index['git_calls'] == 64 and
            all(index[k] is False for k in ('retry', 'overhead_accepted', 'package_pass', 'server')), 'scope differs')
    members = {}
    require(len(index['members']) == 159, 'member count differs')
    for name, entry in index['members'].items():
        body = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(body) == entry['bytes'] and sha(body) == entry['sha256'], 'raw member differs: '+name)
        members[name] = body
    return members


def audit(members):
    def obj(name):
        return json.loads(members[name])
    labels = ('coordinator-frames-native-build-first', 'coordinator-frames-native-first')
    receipts = []
    for label in labels:
        before, after = (obj('checks/'+label+'/'+part) for part in ('intent.json', 'result.json'))
        require(before['head'] == SOURCE and after['exit'] == 0 and after['outer_refusal'] is None and
                after['source_unchanged'] is True and after['head_unchanged'] is True and
                before['source'] == after['source_after'], 'receipt/source differs')
        require(before['bound_seconds'] == 1200 and before['disk_floor_bytes'] == 4*1024**3 and
                all(before[k] is False for k in ('dependency_changes', 'observation_frequency_changed',
                    'services_or_clocks', 'server', 'qualification', 'retry')), 'wrapper scope differs')
        receipts.append(before)
    require(receipts[0]['argv'] == ['cargo', '+1.94.0', 'test', '--offline', '--locked', '-p', 'xtask',
        '--test', 'git_descendant_cpu_074', '--no-run', '--message-format=json'], 'build argv differs')
    require(receipts[1]['argv'] == ['C:/Python313/python.exe', 'tools/xtask-package-attribution-074/git_cpu_overhead.py',
        labels[0], 'first', '--coordinator-frames'], 'native argv differs')
    require(receipts[0]['source'] == receipts[1]['source'], 'build/native source differs')
    for name, raw in members.items():
        if name.startswith('source/') and not name.endswith('/qualification-manifest.toml'):
            require(sha(raw) == receipts[0]['source'][name[7:]], 'retained source differs')
    require(sha(members['source/docs/testing/performance/0.74/qualification-manifest.toml']) ==
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification changed')
    require(sha(members['baseline/model-index.json']) ==
            '7f768549711cb4cdd5a0e2de8bcac1502951499c5a36e28f05c678dd483d2eb1', 'model history differs')
    build = obj('native/build.json')
    require(build['head'] == SOURCE and build['source'] == receipts[0]['source'] and
            build['image_sha256'] == 'c0e2478e96d31e1b9d2100d0bce4706720f274b68e8cda698b9dcc6780dba94f',
            'image/build join differs')
    artifacts = [r for line in members['checks/'+labels[0]+'/stdout.log'].splitlines()
                 if line.startswith(b'{') for r in [json.loads(line)] if r.get('reason') == 'compiler-artifact'
                 and r.get('target', {}).get('name') == 'git_descendant_cpu_074']
    require(len(artifacts) == 1 and artifacts[0]['executable'] == build['image'] and
            artifacts[0]['profile']['test'] is True, 'compiler image differs')
    intent, result = obj('native/intent.json'), obj('native/result.json')
    schedule = [(b, s, mode) for b in range(4) for s, mode in enumerate(
        (REFERENCE, PROFILE, PROFILE, REFERENCE) if b % 2 == 0 else (PROFILE, REFERENCE, REFERENCE, PROFILE))]
    require(intent['schedule'] == [list(v) for v in schedule] and intent['calls_per_worker'] == 4 and
            (intent['bound_seconds'], intent['cleanup_seconds'], intent['free_bytes'], intent['log_bytes']) ==
            (120, 5, 4*1024**3, 16*1024**2) and intent['build_label'] == labels[0] and
            intent['coordinator_frames'] == dict(enabled=True, schema='hc074-coordinator-frames-v1'), 'intent differs')
    require(all(intent[k] is False for k in ('job_overhead_measured', 'pure_counter_api_cost_isolated',
            'overhead_accepted', 'git_suspended', 'stdin_hold', 'native_bypass', 'environment_changes',
            'retry', 'package_pass', 'server')), 'native scope promoted')
    require(result['passed'] is True and result['first_refusal'] is None and len(result['attempts']) == 16 and
            all(result[k] is False for k in ('overhead_accepted', 'speedup_established', 'package_pass', 'server')),
            'native verdict differs')
    require(result['pins_before'] == result['pins_after'] == PINS, 'Git pins differ')
    environment = result['environment']
    require(environment['git_effect_config_active'] is False and environment['git_effect_config_sha256'] == sha(b'') and
            all(environment[k] is True for k in ('external_cargo_configuration_absent', 'fixture_overrides_absent',
                'negative_fixture_executable_absent', 'target_and_protoc_scratch_not_reparse',
                'unreviewed_build_overrides_absent')), 'environment differs')
    totals = {p: dict(wall_lower_ns=0, wall_upper_ns=0, process_cpu_ns=0, zero_cpu_spans=0) for p in PHASES}
    maxima = {p: dict(wall_upper_ns=-1, block=None, slot=None) for p in PHASES}
    worker_ids = set()
    for attempt, (block, slot, mode) in zip(result['attempts'], schedule):
        require(attempt == obj(f'native/{block}-{slot}-result.json') and
                (attempt['block'], attempt['slot'], attempt['mode']) == (block, slot, mode), 'worker ledger differs')
        require(attempt['passed'] is True and attempt['first_refusal'] is None and attempt['cleanup_errors'] == [] and
                attempt['operation_verdict'] == dict(passed=True, first_refusal=None, cleanup_errors=[]), 'operation differs')
        worker, start, job = (attempt[k] for k in ('worker', 'worker_initial', 'job'))
        identity = (attempt['worker_pid'], start['creation_100ns'])
        require(identity not in worker_ids and type(identity[0]) is int and identity[0] > 0 and
                start['terminal'] is False and worker['terminal'] is True and
                worker['creation_100ns'] == start['creation_100ns'] > 0 and
                worker['exit_100ns'] >= worker['creation_100ns'] and attempt['worker_image'] == build['image'], 'identity differs')
        worker_ids.add(identity)
        require(attempt['job_terminal_samples'] == [job, job] and job['active'] == job['terminated'] == 0 and
                job['total'] == 9 and job['user_ns'] >= 0 and job['kernel_ns'] >= 0, 'terminal Job differs')
        require(all(worker[k] >= start[k] >= 0 for k in ('user_100ns', 'kernel_100ns')), 'worker CPU regressed')
        metric = attempt['metrics']
        require(metric['whole_job_cpu_ns'] == job['user_ns']+job['kernel_ns'] and
                metric['whole_worker_cpu_ns'] == (worker['user_100ns']+worker['kernel_100ns'])*100 and
                all(type(v) is int and v >= 0 for v in metric.values()) and
                metric['whole_job_cpu_ns'] >= metric['whole_worker_cpu_ns'], 'CPU arithmetic differs')
        raw = members[f'native/{block}-{slot}-{mode}.log']
        records = [json.loads(line[len(b'HC074_GIT_CPU '):]) for line in raw.splitlines() if line.startswith(b'HC074_GIT_CPU ')]
        done = [json.loads(line[len(b'HC074_GIT_CPU_DONE '):]) for line in raw.splitlines() if line.startswith(b'HC074_GIT_CPU_DONE ')]
        require(len(raw) <= 16*1024**2 and b'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out;' in raw and
                records == attempt['records'] and len(records) == 4 and done == [dict(calls=4, accounted=mode == PROFILE,
                    git_argv=['git', 'show', SPEC], stdin_hold=False, git_suspended=False, native_bypass=False,
                    environment_changes=False, package_pass=False)], 'worker log differs')
        for call, row in enumerate(records):
            require(row['call'] == call and row['accounted'] is (mode == PROFILE) and row['spec'] == SPEC and
                    row['stdout_bytes'] == 23589 and row['stderr_bytes'] == 0 and row['code'] == 0, 'Git semantics differs')
            if mode == REFERENCE:
                require(row['phases'] is None, 'reference phases invented')
            else:
                a, b = (row['phases'][k] for k in ('child_start', 'child_end'))
                require(a['pid'] == b['pid'] and a['created_100ns'] == b['created_100ns'] > 0 and
                        b['exited_100ns'] >= b['created_100ns'] and
                        all(b[k] >= a[k] >= 0 for k in ('user_ns', 'kernel_ns')), 'launcher identity/counter differs')
        if mode == PROFILE:
            residual = attempt['native_inclusive']
            for metric_name, field in (('user_ns', 'user_100ns'), ('kernel_ns', 'kernel_100ns')):
                value = job[metric_name]-worker[field]*100-sum(r['phases']['child_end'][metric_name] for r in records)
                require(value >= 0 and residual['non_launcher_descendant_cpu'][metric_name] == value, 'residual differs')
            require(residual['cpu_includes_native'] is True and all(residual[k] is False for k in
                ('exact_native_image_attested', 'per_call_native_cpu', 'initial_sample_subtracted',
                 'overhead_accepted', 'speedup_established', 'package_pass')), 'residual scope promoted')
        tape = attempt['coordinator_frames']
        contract = tape['contract']
        require(contract == result['coordinator_frames_contract'] and contract['schema'] == 'hc074-coordinator-frames-v1' and
                contract['phases'] == list(PHASES) and (contract['max_probes'], contract['max_clock_reads']) == (12, 36) and
                all(contract[k] is False for k in ('existing_predicates_removed_or_reordered', 'window_renewed',
                    'complete_frame_partition', 'cpu_is_native_git', 'overhead_accepted')), 'phase contract differs')
        require(tape['clock_domain'] == 'python-coordinator' and tape['complete'] is tape['capture_valid'] is True and
                tape['first_refusal'] is None and tape['not_observed'] == [] and len(tape['probes']) == 12 and
                len(tape['spans']) == 6, 'incomplete native phase capture')
        previous = None
        for i, probe in enumerate(tape['probes']):
            q = probe['raw']
            require(probe['phase'] == PHASES[i//2] and probe['kind'] == ('begin' if i % 2 == 0 else 'end') and
                    set(q) == {'wall_before_ns', 'process_cpu_ns', 'wall_after_ns'} and
                    all(type(v) is int and v >= 0 for v in q.values()) and q['wall_after_ns'] >= q['wall_before_ns'], 'probe differs')
            require(previous is None or (q['wall_before_ns'] >= previous['wall_after_ns'] and
                    q['process_cpu_ns'] >= previous['process_cpu_ns']), 'probe sequence regressed')
            previous = q
        for i, span in enumerate(tape['spans']):
            require((span['phase'], span['status'], span['start_probe'], span['end_probe']) ==
                    (PHASES[i], 'complete', i*2, i*2+1), 'span schedule differs')
            a, b = (tape['probes'][j]['raw'] for j in (i*2, i*2+1))
            elapsed = dict(wall_lower_ns=b['wall_before_ns']-a['wall_after_ns'],
                wall_upper_ns=b['wall_after_ns']-a['wall_before_ns'], process_cpu_ns=b['process_cpu_ns']-a['process_cpu_ns'])
            require(span['elapsed'] == elapsed and all(v >= 0 for v in elapsed.values()), 'bracket arithmetic differs')
            phase = PHASES[i]
            for key, value in elapsed.items():
                totals[phase][key] += value
            totals[phase]['zero_cpu_spans'] += int(elapsed['process_cpu_ns'] == 0)
            if elapsed['wall_upper_ns'] > maxima[phase]['wall_upper_ns']:
                maxima[phase] = dict(wall_upper_ns=elapsed['wall_upper_ns'], block=block, slot=slot)
        require(sum(s['elapsed']['process_cpu_ns'] for s in tape['spans']) <= metric['coordinator_cpu_ns'] and
                sum(s['elapsed']['wall_upper_ns'] for s in tape['spans']) <= metric['wall_ns'], 'phase totals exceed frame')
    analysis = result['analysis']
    require(analysis['workers'] == 16 and analysis['git_calls'] == 64 and len(analysis['blocks']) == 4 and
            all(analysis[k] is False for k in ('job_overhead_measured', 'pure_counter_api_cost_isolated',
                'overhead_accepted', 'speedup_established', 'package_pass')), 'analysis scope promoted')
    for block in range(4):
        summary = analysis['blocks'][block]
        require(summary['block'] == block and summary['calls_per_arm'] == 8, 'block scope differs')
        for metric_name, reported in summary['metrics'].items():
            a, b = (sum(v['metrics'][metric_name] for v in result['attempts'][block*4:(block+1)*4] if v['mode'] == mode)
                    for mode in (REFERENCE, PROFILE))
            require(reported == dict(reference_sum=a, profile_sum=b, delta=b-a,
                    ratio_numerator=b if a else None, ratio_denominator=a if a else None,
                    unavailable_reason=None if a else 'zero reference counter'), 'block arithmetic differs')
    return dict(native_attempts=1, workers=16, git_calls=64, probes=192, clock_reads=576,
        phase_totals=totals, phase_maxima=maxima, whole_frame_wall_ns=sum(a['metrics']['wall_ns'] for a in result['attempts']),
        whole_coordinator_cpu_ns=sum(a['metrics']['coordinator_cpu_ns'] for a in result['attempts']),
        overhead_accepted=False, package_pass=False, new_workload_calls=0)


def verify(folder):
    return audit(load(folder))


if __name__ == '__main__':
    print(json.dumps(verify(Path(__file__).resolve().parent), indent=2, sort_keys=True))
