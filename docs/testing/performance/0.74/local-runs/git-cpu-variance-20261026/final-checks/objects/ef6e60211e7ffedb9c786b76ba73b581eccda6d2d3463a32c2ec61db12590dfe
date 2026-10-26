"""Independent offline CPU arithmetic; never executes retained analysis code."""
import hashlib
import json
import math
from pathlib import Path

INDEX = '586480e0b6800d416ea2a83504b1beb3386fc0e33bb67fb78bcad125cf8a95d6'
INPUT = '1ba276dcf62f97801f69806ea5efba138c7d47425f30d2563874aa393681fcff'
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def verify(folder):
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['source_commit'] == '385feda3938aabf467857271047e60a5c961619f' and
            index['input_index_sha256'] == INPUT and index['input_git_calls'] == 64 and index['new_workload_calls'] == 0,
            'scope differs')
    require(all(index[key] is False for key in ('workload_replayed', 'overhead_accepted', 'speedup_established',
                                               'package_pass', 'server')), 'index promoted')
    members = {}
    for name, entry in index['members'].items():
        body = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(body) == entry['bytes'] and sha(body) == entry['sha256'], 'raw member differs: '+name)
        members[name] = body
    require(len(members) == 68, 'member count differs')
    def obj(name):
        return json.loads(members[name])
    require(sha(members['input/index.json']) == INPUT, 'old input index differs')
    old = obj('input/index.json')
    for name, body in members.items():
        if name.startswith('input/') and name != 'input/index.json':
            require(sha(body) == old['members'][name[6:]]['sha256'], 'old input member differs')
    for label in ('git-cpu-variance-model-first', 'git-cpu-variance-model-precommit',
                  'git-cpu-variance-syntax', 'git-cpu-variance-analysis-first'):
        before, after = (obj('checks/'+label+'/'+part) for part in ('intent.json', 'result.json'))
        require(after['exit'] == 0 and after['outer_refusal'] is None and after['source_unchanged'] is True and
                after['head_unchanged'] is True and before['source'] == after['source_after'], 'check verdict/source differs')
        require(before['bound_seconds'] == 1200 and all(before[key] is False for key in ('dependency_changes',
            'observation_frequency_changed', 'services_or_clocks', 'server', 'original_owner', 'qualification', 'retry')),
            'check scope differs')
    for label, count in (('git-cpu-variance-model-first', 13), ('git-cpu-variance-model-precommit', 17)):
        log = members['checks/'+label+'/stderr.log']
        require(('Ran '+str(count)+' tests').encode() in log and log.rstrip().endswith(b'OK'), 'model verdict differs')
    dispatch = obj('checks/git-cpu-variance-analysis-first/intent.json')
    require(dispatch['head'] == index['source_commit'] and dispatch['argv'][-2:] == ['analyze', 'first'], 'analysis dispatch differs')
    for name, body in members.items():
        if name.startswith('source/'):
            require(sha(body) == dispatch['source'][name[7:]], 'analysis source differs')
        elif name.startswith('raw-first-model/'):
            require(sha(body) == obj('checks/git-cpu-variance-model-first/intent.json')['source'][
                'tools/xtask-package-attribution-074/'+name.split('/')[-1]], 'first model source differs')
    intent, result = (obj('analysis/'+part) for part in ('intent.json', 'result.json'))
    require(intent == dict(input_index_sha256=INPUT, input_git_calls=64, workload_replayed=False,
                          processes_launched_by_analyzer=0, clock_reads_by_analyzer=0, package_pass=False), 'offline intent differs')
    require(result['passed'] is True and result['first_refusal'] is None and result['workload_replayed'] is False and
            result['package_pass'] is False, 'analysis first outcome differs')
    analysis = result['analysis']
    source = obj('input/native/result.json')
    require(len(source['attempts']) == len(analysis['workers']) == 16 and analysis['input_git_calls'] == 64 and
            analysis['profile_calls'] == 32 and analysis['all_samples_retained'] is True, 'sample count differs')
    require(all(analysis[key] is False for key in ('workload_replayed', 'os_timer_resolution_proved',
        'cpu_by_phase_available', 'exact_native_image_attested', 'coordinator_leaf_cost_available', 'job_overhead_measured',
        'overhead_accepted', 'speedup_established', 'package_pass')), 'analysis scope promoted')
    primitive = []
    phases = {key: [] for key in ('spawn_ns', 'handle_observation_ns', 'wait_and_collect_ns', 'terminal_observation_ns',
                                 'counter_observation_wall_ns')}
    for raw, row in zip(source['attempts'], analysis['workers']):
        require((raw['block'], raw['slot'], raw['mode'], raw['worker_pid']) ==
                (row['block'], row['slot'], row['mode'], row['worker_pid']), 'ordered worker join differs')
        job, worker, metric = (raw[key] for key in ('job', 'worker', 'metrics'))
        j = job['user_ns']+job['kernel_ns']
        w = (worker['user_100ns']+worker['kernel_100ns'])*100
        c = metric['coordinator_cpu_ns']
        require(row['cpu'] == dict(job_cpu_ns=j, worker_cpu_ns=w, descendant_cpu_ns=j-w,
            coordinator_cpu_ns=c, recorded_cpu_envelope_ns=j+c) and row['recorded_frame_wall_ns'] == metric['wall_ns'],
            'CPU owner partition differs')
        primitive.extend([job['user_ns'], job['kernel_ns'], worker['user_100ns']*100, worker['kernel_100ns']*100, c])
        if raw['mode'] == 'reference_cpu_child':
            require(row['direct_launcher_cpu_ns'] is None and row['non_launcher_descendant_cpu_ns'] is None and
                    row['recorded_phase_wall_ns'] is None, 'invented reference phases/counters')
            continue
        direct = 0
        total = {key: 0 for key in phases if key != 'counter_observation_wall_ns'}
        for record in raw['records']:
            phase = record['phases']
            for key in total:
                total[key] += phase[key]
                phases[key].append(phase[key])
            phases['counter_observation_wall_ns'].append(phase['handle_observation_ns']+phase['terminal_observation_ns'])
            end = phase['child_end']
            direct += end['user_ns']+end['kernel_ns']
            primitive.extend([end['user_ns'], end['kernel_ns']])
        require(row['direct_launcher_cpu_ns'] == direct and row['non_launcher_descendant_cpu_ns'] == j-w-direct and
                row['recorded_phase_wall_ns'] == total, 'profile partition differs')
    for key, values in phases.items():
        require(analysis['profile_call_phase_wall_ns'][key] == dict(count=32, sum=sum(values), minimum=min(values), maximum=max(values)),
                'wall phase statistic differs')
    require(analysis['profile_call_phase_wall_ns']['counter_observation_wall_ns']['sum'] == 643500 and
            analysis['primitive_cpu_counter_count'] == len(primitive) == 144 and
            analysis['observed_cpu_counter_gcd_ns'] == math.gcd(*primitive) == 15625000, 'finite counter statistic differs')
    require(len(analysis['blocks']) == 4, 'block count differs')
    for block in analysis['blocks']:
        values = analysis['workers'][block['block']*4:block['block']*4+4]
        require(block['calls_per_arm'] == 8 and block['reference_call_wall_phases'] is None, 'block scope differs')
        for key, value in block['cpu'].items():
            x, y = (sum(v['cpu'][key] for v in values if v['mode'] == mode)
                    for mode in ('reference_cpu_child', 'accounted_cpu_child'))
            require(value == dict(reference_sum=x, profile_sum=y, delta=y-x), 'block delta differs')
        delta = {key: value['delta'] for key, value in block['cpu'].items()}
        require(delta['job_cpu_ns'] == delta['worker_cpu_ns']+delta['descendant_cpu_ns'] and
                delta['recorded_cpu_envelope_ns'] == delta['job_cpu_ns']+delta['coordinator_cpu_ns'], 'block CPU does not close')
    return dict(input_git_calls=64, new_workload_calls=0, models=17, observation_wall_sum_ns=643500,
                cpu_owners_attributed=True, causal_leaf_cost_proved=False, overhead_accepted=False, package_pass=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
