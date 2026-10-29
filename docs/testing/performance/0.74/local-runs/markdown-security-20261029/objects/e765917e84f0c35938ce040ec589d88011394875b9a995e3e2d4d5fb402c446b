"""Pure fixed-packet attribution. No process, clocks, collector import or replay."""
import hashlib
import json
import math
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT/'docs/testing/performance/0.74/local-runs/pipe-fixture-git-overhead-20261026'
INDEX = '1ba276dcf62f97801f69806ea5efba138c7d47425f30d2563874aa393681fcff'
REFERENCE, PROFILE = 'reference_cpu_child', 'accounted_cpu_child'
PHASES = ('spawn_ns', 'handle_observation_ns', 'wait_and_collect_ns', 'terminal_observation_ns')
CPU = ('job_cpu_ns', 'worker_cpu_ns', 'descendant_cpu_ns', 'coordinator_cpu_ns', 'recorded_cpu_envelope_ns')
SPEC = '3e9bcc06ebce3439495200162cb0c1900b8eda53:crates/xtask/tests/diagnostic_artifact_fixture_074.rs'


def require(value, reason):
    if not value:
        raise ValueError(reason)


def integer(value):
    require(type(value) is int and value >= 0, 'invalid nonnegative integer counter')
    return value


def stats(values):
    return dict(count=len(values), sum=sum(values), minimum=min(values), maximum=max(values))


def load_packet(folder):
    raw = (folder/'index.json').read_bytes()
    require(hashlib.sha256(raw).hexdigest() == INDEX, 'fixed input index differs')
    index = json.loads(raw)
    require(len(index['members']) == 108 and index['source_commit'] == '20247dd65f821aff33946b9643032c7773f0b146'
            and index['cohorts'] == 1 and index['git_calls'] == 64 and index['overhead_accepted'] is False,
            'input scope differs')
    members = {}
    for name, entry in index['members'].items():
        require(re.fullmatch('[0-9a-f]{64}', entry['sha256']) and
                type(entry['bytes']) is int and 0 <= entry['bytes'] <= 16*1024**2, 'unsafe raw object declaration')
        path = folder/'objects'/entry['sha256']
        require(path.stat().st_size == entry['bytes'], 'raw input size differs: '+name)
        body = path.read_bytes()
        require(hashlib.sha256(body).hexdigest() == entry['sha256'], 'raw input differs: '+name)
        members[name] = body
    result = json.loads(members['native/result.json'])
    for row in result['attempts']:
        prefix = f"native/{row['block']}-{row['slot']}"
        require(row == json.loads(members[prefix+'-result.json']), 'individual attempt ledger differs')
        text = members[prefix+'-'+row['mode']+'.log'].decode()
        records = [json.loads(v[14:]) for v in text.splitlines() if v.startswith('HC074_GIT_CPU ')]
        require(records == row['records'], 'raw worker record join differs')
    return result


def analyze(result):
    require(result['passed'] is True and result['first_refusal'] is None and
            all(result[key] is False for key in ('overhead_accepted', 'speedup_established', 'package_pass', 'server')),
            'refused/promoted input cohort')
    attempts = result['attempts']
    require(len(attempts) == 16, 'incomplete cohort; never analyze a successful subset')
    expected = [(b, s, mode) for b in range(4) for s, mode in enumerate(
        (REFERENCE, PROFILE, PROFILE, REFERENCE) if b % 2 == 0 else (PROFILE, REFERENCE, REFERENCE, PROFILE))]
    workers, counters, lifetimes = [], [], set()
    phase_values = {name: [] for name in (*PHASES, 'counter_observation_wall_ns')}
    for row, order in zip(attempts, expected):
        require((row['block'], row['slot'], row['mode']) == order and row['passed'] is True and
                row['first_refusal'] is None and row['cleanup_errors'] == [], 'schedule/first outcome differs')
        job, a, b, metric = (row[key] for key in ('job', 'worker_initial', 'worker', 'metrics'))
        require(row['job_terminal_samples'] == [job, job] and type(job['total']) is int and job['total'] == 9 and
                type(job['active']) is int and job['active'] == 0 and type(job['terminated']) is int and
                job['terminated'] == 0, 'nonterminal/unstable Job census')
        require(a['terminal'] is False and a['exit_100ns'] is None and b['terminal'] is True and
                b['creation_100ns'] == a['creation_100ns'] > 0 and b['exit_100ns'] >= b['creation_100ns'],
                'original worker lifetime drift')
        require(type(row['worker_pid']) is int and row['worker_pid'] > 0, 'invalid worker PID')
        lifetime = (row['worker_pid'], b['creation_100ns'])
        require(lifetime not in lifetimes, 'duplicate worker lifetime')
        lifetimes.add(lifetime)
        for key in ('user_100ns', 'kernel_100ns'):
            require(integer(b[key]) >= integer(a[key]), 'worker CPU counter regressed')
        job_cpu = integer(job['user_ns'])+integer(job['kernel_ns'])
        worker_cpu = (b['user_100ns']+b['kernel_100ns'])*100
        require(job_cpu >= worker_cpu and integer(metric['whole_job_cpu_ns']) == job_cpu and
                integer(metric['whole_worker_cpu_ns']) == worker_cpu, 'worker/Job CPU identity differs')
        cpu = dict(job_cpu_ns=job_cpu, worker_cpu_ns=worker_cpu, descendant_cpu_ns=job_cpu-worker_cpu,
            coordinator_cpu_ns=integer(metric['coordinator_cpu_ns']),
            recorded_cpu_envelope_ns=job_cpu+metric['coordinator_cpu_ns'])
        wall = integer(metric['wall_ns'])
        require(wall > 0, 'zero recorded frame wall')
        counters.extend([job['user_ns'], job['kernel_ns'], b['user_100ns']*100,
                         b['kernel_100ns']*100, metric['coordinator_cpu_ns']])
        records = row['records']
        require(len(records) == 4, 'missing command records')
        phases = {key: 0 for key in PHASES} if row['mode'] == PROFILE else None
        roots, root_lifetimes = [], set()
        for call, record in enumerate(records):
            require(type(record['call']) is int and record['call'] == call and record['accounted'] is (row['mode'] == PROFILE)
                    and record['spec'] == SPEC and record['stdout_bytes'] == 23589 and
                    record['stderr_bytes'] == 0 and record['code'] == 0,
                    'command identity/semantics drift')
            phase = record['phases']
            if row['mode'] == REFERENCE:
                require(phase is None, 'reference acquired phases; never invent missing observations')
                continue
            for key in PHASES:
                value = integer(phase[key])
                phases[key] += value
                phase_values[key].append(value)
            phase_values['counter_observation_wall_ns'].append(phase['handle_observation_ns']+phase['terminal_observation_ns'])
            first, last = phase['child_start'], phase['child_end']
            require(first['pid'] == last['pid'] != row['worker_pid'] and
                    first['created_100ns'] == last['created_100ns'] and
                    b['creation_100ns'] <= last['created_100ns'] <= last['exited_100ns'] <= b['exit_100ns'],
                    'retained launcher lifetime drift')
            key = (last['pid'], last['created_100ns'])
            require(key not in root_lifetimes, 'duplicate launcher lifetime')
            root_lifetimes.add(key)
            require(all(integer(last[k]) >= integer(first[k]) for k in ('user_ns', 'kernel_ns')),
                    'launcher CPU regressed')
            roots.append(last)
            counters.extend([last['user_ns'], last['kernel_ns']])
        direct = sum(v['user_ns']+v['kernel_ns'] for v in roots) if phases is not None else None
        residual = None
        if phases is not None:
            residual = {key: job[key]-b[field]*100-sum(v[key] for v in roots)
                        for key, field in (('user_ns', 'user_100ns'), ('kernel_ns', 'kernel_100ns'))}
            require(all(v >= 0 for v in residual.values()) and
                    residual == row['native_inclusive']['non_launcher_descendant_cpu'], 'CPU residual underflow/drift')
        workers.append(dict(block=row['block'], slot=row['slot'], mode=row['mode'], worker_pid=row['worker_pid'],
            cpu=cpu, recorded_frame_wall_ns=wall, direct_launcher_cpu_ns=direct,
            non_launcher_descendant_cpu_ns=sum(residual.values()) if residual is not None else None,
            recorded_phase_wall_ns=phases))
    blocks = []
    for block in range(4):
        values = workers[block*4:block*4+4]
        parts = {}
        for key in CPU:
            x, y = (sum(v['cpu'][key] for v in values if v['mode'] == mode) for mode in (REFERENCE, PROFILE))
            parts[key] = dict(reference_sum=x, profile_sum=y, delta=y-x)
        require(parts['job_cpu_ns']['delta'] == parts['worker_cpu_ns']['delta']+parts['descendant_cpu_ns']['delta'],
                'block CPU partition does not close')
        require(parts['recorded_cpu_envelope_ns']['delta'] == parts['job_cpu_ns']['delta']+parts['coordinator_cpu_ns']['delta'],
                'recorded CPU envelope does not close')
        blocks.append(dict(block=block, calls_per_arm=8, cpu=parts,
            reference_call_wall_phases=None, reference_phases_reason='not captured in original reference'))
    return dict(workers=workers, blocks=blocks, profile_call_phase_wall_ns={key: stats(v) for key, v in phase_values.items()},
        primitive_cpu_counter_count=len(counters), observed_cpu_counter_gcd_ns=math.gcd(*counters),
        input_git_calls=64, profile_calls=32, workload_replayed=False, all_samples_retained=True,
        os_timer_resolution_proved=False, cpu_by_phase_available=False, exact_native_image_attested=False,
        coordinator_leaf_cost_available=False, job_overhead_measured=False, overhead_accepted=False,
        speedup_established=False, package_pass=False)


def run(label):
    require(re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', label), 'bounded output label required')
    folder = ROOT/'target/git-cpu-variance-20261026'/label
    folder.mkdir(parents=True, exist_ok=False)
    def write(name, value):
        with (folder/name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, sort_keys=True, indent=2)
            stream.write('\n')
    write('intent.json', dict(input_index_sha256=INDEX, input_git_calls=64, workload_replayed=False,
        processes_launched_by_analyzer=0, clock_reads_by_analyzer=0, package_pass=False))
    result = dict(passed=False, first_refusal=None, workload_replayed=False, package_pass=False)
    try:
        result['analysis'] = analyze(load_packet(PACKET))
        result['passed'] = True
    except Exception as error:
        result['first_refusal'] = f'{type(error).__name__}: {error}'
    finally:
        write('result.json', result)
    return result


if __name__ == '__main__':
    require(len(sys.argv) == 3 and sys.argv[1] == 'analyze', 'usage: git_cpu_variance.py analyze NEW_LABEL')
    result = run(sys.argv[2])
    print(json.dumps(dict(passed=result['passed'], first_refusal=result['first_refusal'], workload_replayed=False)))
    raise SystemExit(0 if result['passed'] else 1)
