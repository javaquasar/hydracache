"""Independent retained arithmetic. No analyzer/collector import or execution."""
import hashlib
import json
from pathlib import Path

INDEX = '055c7f306ad9dafb55b85a578ec788231a11f504e2117830991cdc0c5436a5b8'
INPUT = '55b802621e147681f9478a164053a04c055ec28d8d35227d68d49ad0b4de380f'
SOURCE = 'fa9b22330289ad0dacd05faf97310fa0b3b96336'
PHASES = ('setup', 'spawn', 'wait', 'terminal_reads', 'decode', 'close')
KEYS = ('wall_lower_ns', 'wall_upper_ns', 'process_cpu_ns')
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def load(folder):
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['source_commit'] == SOURCE and index['input_index_sha256'] == INPUT and
            index['input_git_calls'] == 64 and index['new_workload_calls'] == 0 and all(index[k] is False for k in
            ('workload_replayed', 'overhead_accepted', 'package_pass', 'server')), 'scope differs')
    require(len(index['members']) == 73, 'member count differs')
    members = {}
    for name, entry in index['members'].items():
        body = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(body) == entry['bytes'] and sha(body) == entry['sha256'], 'raw member differs: '+name)
        members[name] = body
    return members


def audit(members):
    def obj(name):
        return json.loads(members[name])
    require(sha(members['input/index.json']) == INPUT, 'input index differs')
    old = obj('input/index.json')
    for name, raw in members.items():
        if name.startswith('input/') and name != 'input/index.json':
            entry = old['members'][name[6:]]
            require(sha(raw) == entry['sha256'] and len(raw) == entry['bytes'], 'input member differs')
    for label in ('model', 'regressions', 'syntax', 'check', 'clippy', 'offline'):
        stem = 'checks/capture-attribution-'+label+'-first/'
        a, b = (obj(stem+part) for part in ('intent.json', 'result.json'))
        require(b['exit'] == 0 and b['outer_refusal'] is None and b['source_unchanged'] is b['head_unchanged'] is True and
                a['source'] == b['source_after'] and a['bound_seconds'] == 1200 and a['disk_floor_bytes'] == 4*1024**3,
                'receipt/source differs')
        require(all(a[k] is False for k in ('dependency_changes', 'observation_frequency_changed',
                'services_or_clocks', 'server', 'qualification', 'retry')), 'check scope differs')
    for label, count in (('model', 14), ('regressions', 17)):
        log = members[f'checks/capture-attribution-{label}-first/stderr.log']
        require(f'Ran {count} tests'.encode() in log and log.rstrip().endswith(b'OK'), 'test verdict differs')
    dispatch = obj('checks/capture-attribution-offline-first/intent.json')
    require(dispatch['head'] == SOURCE and dispatch['argv'][-2:] == ['analyze', 'first'], 'analysis dispatch differs')
    for name, raw in members.items():
        if name.startswith('source/'):
            require(sha(raw) == dispatch['source'][name[7:]], 'analysis source differs')
    require(obj('analysis/intent.json') == dict(schema='hc074-offline-capture-attribution-v1', input_index_sha256=INPUT,
        input_source='1030acb263e96d1e023f852dceb021de458ce6f4', input_git_calls=64, processes_launched_by_analyzer=0,
        clock_reads_by_analyzer=0, workload_replayed=False, retry=False, package_pass=False), 'offline intent differs')
    output = obj('analysis/result.json')
    require(output['passed'] is True and output['first_refusal'] is None and output['workload_replayed'] is False and
            output['new_workload_calls'] == 0 and output['package_pass'] is False, 'analysis first outcome differs')
    analysis, native = output['analysis'], obj('input/native/result.json')
    require(all(analysis[k] is False for k in ('nested_timing_scopes_additive', 'worker_phase_cpu_available',
        'pure_api_cost_proved', 'zero_delta_means_zero_cost', 'causal_capture_cost_available', 'telemetry_on_off_compared',
        'workload_replayed', 'overhead_accepted', 'speedup_established', 'package_pass')) and
        analysis['all_samples_retained'] is True and analysis['reference_call_wall_phases'] is None and
        analysis['input_git_calls'] == 64 and analysis['new_workload_calls'] == 0, 'analysis scope promoted')
    require(len(native['attempts']) == len(analysis['frames']) == len(analysis['owners']['workers']) == 16 and
            len(analysis['blocks']) == 4 and len(analysis['profile_calls']) == 32, 'sample count differs')
    require(all(analysis['owners'][k] is False for k in ('workload_replayed', 'os_timer_resolution_proved',
        'cpu_by_phase_available', 'exact_native_image_attested', 'coordinator_leaf_cost_available',
        'job_overhead_measured', 'overhead_accepted', 'speedup_established', 'package_pass')), 'owner scope promoted')
    frames, cpus, calls = [], [], []
    for raw, reported_owner in zip(native['attempts'], analysis['owners']['workers']):
        j = raw['job']['user_ns']+raw['job']['kernel_ns']
        w = (raw['worker']['user_100ns']+raw['worker']['kernel_100ns'])*100
        c = raw['metrics']['coordinator_cpu_ns']
        cpu = dict(job_cpu_ns=j, worker_cpu_ns=w, descendant_cpu_ns=j-w, coordinator_cpu_ns=c, recorded_cpu_envelope_ns=j+c)
        cpus.append(cpu)
        require(reported_owner['cpu'] == cpu and reported_owner['worker_pid'] == raw['worker_pid'], 'owner arithmetic differs')
        phases = {}
        for i, phase in enumerate(PHASES):
            a, b = (raw['coordinator_frames']['probes'][k]['raw'] for k in (i*2, i*2+1))
            phases[phase] = dict(wall_lower_ns=b['wall_before_ns']-a['wall_after_ns'],
                wall_upper_ns=b['wall_after_ns']-a['wall_before_ns'], process_cpu_ns=b['process_cpu_ns']-a['process_cpu_ns'])
        complement = dict(wall_lower_ns=raw['metrics']['wall_ns']-sum(p['wall_upper_ns'] for p in phases.values()),
            wall_upper_ns=raw['metrics']['wall_ns']-sum(p['wall_lower_ns'] for p in phases.values()),
            process_cpu_ns=c-sum(p['process_cpu_ns'] for p in phases.values()))
        require(all(v >= 0 for v in complement.values()), 'negative complement')
        frame = dict(block=raw['block'], slot=raw['slot'], mode=raw['mode'], phases=phases,
            arithmetic_complement=complement, complement_is_pure_effect_cost=False)
        frames.append(frame)
        if raw['mode'] == 'reference_cpu_child':
            require(reported_owner['recorded_phase_wall_ns'] is reported_owner['direct_launcher_cpu_ns'] is
                    reported_owner['non_launcher_descendant_cpu_ns'] is None, 'invented reference phases/counters')
            continue
        for row in raw['records']:
            p = row['phases']
            calls.append(dict(block=raw['block'], slot=raw['slot'], call=row['call'], spawn_wall_ns=p['spawn_ns'],
                handle_observation_wall_ns=p['handle_observation_ns'], wait_and_collect_wall_ns=p['wait_and_collect_ns'],
                terminal_observation_wall_ns=p['terminal_observation_ns'], phase_cpu_ns=None, absolute_monotonic_endpoints=None))
    require(analysis['frames'] == frames and analysis['profile_calls'] == calls, 'phase/frame/call join differs')
    total = lambda rows: {p: {k: sum(f['phases'][p][k] for f in rows) for k in KEYS} for p in PHASES}
    require(analysis['coordinator_phase_totals'] == total(frames) and analysis['arithmetic_complement'] ==
            {k: sum(f['arithmetic_complement'][k] for f in frames) for k in KEYS}, 'total/complement arithmetic differs')
    stat = lambda values: dict(count=len(values), sum=sum(values), minimum=min(values), maximum=max(values))
    for field, column in (('spawn_ns', 'spawn_wall_ns'), ('handle_observation_ns', 'handle_observation_wall_ns'),
                          ('wait_and_collect_ns', 'wait_and_collect_wall_ns'),
                          ('terminal_observation_ns', 'terminal_observation_wall_ns')):
        require(analysis['owners']['profile_call_phase_wall_ns'][field] == stat([c[column] for c in calls]),
                'worker phase distribution differs')
    require(analysis['owners']['profile_call_phase_wall_ns']['counter_observation_wall_ns'] ==
            stat([c['handle_observation_wall_ns']+c['terminal_observation_wall_ns'] for c in calls]),
            'worker observation sum differs')
    for p in PHASES:
        require(analysis['coordinator_phase_distributions'][p] == dict(
            cpu_ns=stat([f['phases'][p]['process_cpu_ns'] for f in frames]),
            zero_cpu_spans=sum(f['phases'][p]['process_cpu_ns'] == 0 for f in frames),
            wall_lower_ns=stat([f['phases'][p]['wall_lower_ns'] for f in frames]),
            wall_upper_ns=stat([f['phases'][p]['wall_upper_ns'] for f in frames])), 'distribution differs')
    for block in range(4):
        selected = frames[block*4:block*4+4]
        a, b = (total([f for f in selected if f['mode'] == mode]) for mode in ('reference_cpu_child', 'accounted_cpu_child'))
        expected = {p: dict(reference=a[p], profile=b[p], signed_cpu_delta_ns=b[p]['process_cpu_ns']-a[p]['process_cpu_ns'],
            signed_wall_delta_lower_ns=b[p]['wall_lower_ns']-a[p]['wall_upper_ns'],
            signed_wall_delta_upper_ns=b[p]['wall_upper_ns']-a[p]['wall_lower_ns']) for p in PHASES}
        owner_delta = {}
        for k in cpus[0]:
            x, y = (sum(cpus[i][k] for i in range(block*4, block*4+4) if frames[i]['mode'] == mode)
                    for mode in ('reference_cpu_child', 'accounted_cpu_child'))
            owner_delta[k] = dict(reference_sum=x, profile_sum=y, delta=y-x)
        gap_a, gap_b = (sum(f['arithmetic_complement']['process_cpu_ns'] for f in selected if f['mode'] == mode)
                       for mode in ('reference_cpu_child', 'accounted_cpu_child'))
        require(analysis['blocks'][block] == dict(block=block, coordinator_phases=expected,
                arithmetic_complement_cpu_delta_ns=gap_b-gap_a, owners=owner_delta), 'block arithmetic differs')
    return dict(input_git_calls=64, new_workload_calls=0, workers=16, profile_calls=32, models=31,
        independent_phase_arithmetic=True, overhead_accepted=False, package_pass=False)


def verify(folder):
    return audit(load(folder))


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
