"""Fixed retained-packet capture attribution, with no process or clock APIs."""
import hashlib
import json
from pathlib import Path
import re
import sys

from git_cpu_variance import analyze as owner_analysis, integer, require, stats

ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT/'docs/testing/performance/0.74/local-runs/coordinator-native-20261026'
INDEX = '55b802621e147681f9478a164053a04c055ec28d8d35227d68d49ad0b4de380f'
SOURCE = '1030acb263e96d1e023f852dceb021de458ce6f4'
PHASES = ('setup', 'spawn', 'wait', 'terminal_reads', 'decode', 'close')
WALL = ('wall_lower_ns', 'wall_upper_ns')
REFERENCE, PROFILE = 'reference_cpu_child', 'accounted_cpu_child'


def load_packet(folder):
    raw = (folder/'index.json').read_bytes()
    require(hashlib.sha256(raw).hexdigest() == INDEX, 'fixed input index differs')
    index = json.loads(raw)
    require(index['source_commit'] == SOURCE and index['native_attempts'] == 1 and
            index['native_control_executed'] is True and index['workers'] == 16 and index['git_calls'] == 64 and
            all(index[k] is False for k in ('retry', 'overhead_accepted', 'package_pass', 'server')) and
            len(index['members']) == 159, 'fixed input scope differs')
    members = {}
    for name, entry in index['members'].items():
        require(re.fullmatch('[0-9a-f]{64}', entry['sha256']) and type(entry['bytes']) is int and
                0 <= entry['bytes'] <= 16*1024**2, 'unsafe raw object declaration')
        path = folder/'objects'/entry['sha256']
        require(path.stat().st_size == entry['bytes'], 'raw input size differs: '+name)
        body = path.read_bytes()
        require(hashlib.sha256(body).hexdigest() == entry['sha256'], 'raw input differs: '+name)
        members[name] = body
    result = json.loads(members['native/result.json'])
    for row in result['attempts']:
        stem = f"native/{row['block']}-{row['slot']}"
        require(row == json.loads(members[stem+'-result.json']), 'individual ledger differs')
        records = [json.loads(line[len(b'HC074_GIT_CPU '):]) for line in
                   members[stem+'-'+row['mode']+'.log'].splitlines() if line.startswith(b'HC074_GIT_CPU ')]
        require(records == row['records'], 'raw worker records differ')
    return result


def frame(row, contract):
    require(row['operation_verdict'] == dict(passed=True, first_refusal=None, cleanup_errors=[]) and
            row['operation_verdict']['passed'] is True, 'operation verdict differs')
    tape = row['coordinator_frames']
    require(tape['contract'] == contract and tape['complete'] is tape['capture_valid'] is True and
            tape['clock_domain'] == 'python-coordinator' and tape['first_refusal'] is None and
            tape['not_observed'] == [] and len(tape['probes']) == 12 and len(tape['spans']) == 6,
            'incomplete/invalid native coordinator frame')
    previous = None
    for index, event in enumerate(tape['probes']):
        raw = event['raw']
        require(event['phase'] == PHASES[index//2] and event['kind'] == ('end' if index % 2 else 'begin') and
                set(raw) == {'wall_before_ns', 'process_cpu_ns', 'wall_after_ns'}, 'probe order/shape differs')
        for value in raw.values():
            integer(value)
        require(raw['wall_after_ns'] >= raw['wall_before_ns'] and (previous is None or
                (raw['wall_before_ns'] >= previous['wall_after_ns'] and raw['process_cpu_ns'] >= previous['process_cpu_ns'])),
                'probe clock sequence regressed')
        previous = raw
    phases = {}
    for index, span in enumerate(tape['spans']):
        require((span['phase'], span['status'], span['start_probe'], span['end_probe']) ==
                (PHASES[index], 'complete', 2*index, 2*index+1) and
                type(span['start_probe']) is type(span['end_probe']) is int, 'span schedule differs')
        a, b = (tape['probes'][i]['raw'] for i in (2*index, 2*index+1))
        elapsed = dict(wall_lower_ns=b['wall_before_ns']-a['wall_after_ns'],
            wall_upper_ns=b['wall_after_ns']-a['wall_before_ns'], process_cpu_ns=b['process_cpu_ns']-a['process_cpu_ns'])
        require(set(span['elapsed']) == set(elapsed) and all(type(v) is int for v in span['elapsed'].values()) and
                span['elapsed'] == elapsed and all(v >= 0 for v in elapsed.values()), 'span arithmetic differs')
        phases[PHASES[index]] = elapsed
    wall, cpu = (integer(row['metrics'][key]) for key in ('wall_ns', 'coordinator_cpu_ns'))
    complement = dict(wall_lower_ns=wall-sum(v['wall_upper_ns'] for v in phases.values()),
        wall_upper_ns=wall-sum(v['wall_lower_ns'] for v in phases.values()),
        process_cpu_ns=cpu-sum(v['process_cpu_ns'] for v in phases.values()))
    require(all(v >= 0 for v in complement.values()), 'bracket sum exceeds original frame')
    return dict(block=row['block'], slot=row['slot'], mode=row['mode'], phases=phases,
        arithmetic_complement=complement, complement_is_pure_effect_cost=False)


def phase_totals(frames):
    return {phase: {key: sum(row['phases'][phase][key] for row in frames)
                    for key in (*WALL, 'process_cpu_ns')} for phase in PHASES}


def analyze(result):
    owners = owner_analysis(result)
    contract = result['coordinator_frames_contract']
    require(contract == dict(schema='hc074-coordinator-frames-v1', phases=list(PHASES), max_probes=12,
        max_clock_reads=36, wall_clock='time.monotonic_ns', cpu_clock='time.process_time_ns', units='ns',
        cpu_scope='coordinator process-wide, all threads',
        wall_bounds_scope='elapsed between bracketed CPU-read instants, not effect-only duration',
        clock_failure_policy='freeze telemetry, drain original owner, refuse diagnostic verdict',
        existing_predicates_removed_or_reordered=False, window_renewed=False,
        complete_frame_partition=False, cpu_is_native_git=False, overhead_accepted=False) and
        all(contract[k] is False for k in ('existing_predicates_removed_or_reordered', 'window_renewed',
            'complete_frame_partition', 'cpu_is_native_git', 'overhead_accepted')), 'coordinator contract differs')
    frames = [frame(row, contract) for row in result['attempts']]
    blocks = []
    for block in range(4):
        selected = frames[block*4:block*4+4]
        a, b = (phase_totals([f for f in selected if f['mode'] == mode]) for mode in (REFERENCE, PROFILE))
        phases = {p: dict(reference=a[p], profile=b[p], signed_cpu_delta_ns=b[p]['process_cpu_ns']-a[p]['process_cpu_ns'],
            signed_wall_delta_lower_ns=b[p]['wall_lower_ns']-a[p]['wall_upper_ns'],
            signed_wall_delta_upper_ns=b[p]['wall_upper_ns']-a[p]['wall_lower_ns']) for p in PHASES}
        gap_a, gap_b = (sum(f['arithmetic_complement']['process_cpu_ns'] for f in selected if f['mode'] == mode)
                        for mode in (REFERENCE, PROFILE))
        require(sum(p['signed_cpu_delta_ns'] for p in phases.values())+gap_b-gap_a ==
                owners['blocks'][block]['cpu']['coordinator_cpu_ns']['delta'], 'coordinator CPU partition differs')
        blocks.append(dict(block=block, coordinator_phases=phases,
            arithmetic_complement_cpu_delta_ns=gap_b-gap_a, owners=owners['blocks'][block]['cpu']))
    profile_calls = []
    for row in result['attempts']:
        if row['mode'] != PROFILE:
            continue
        for record in row['records']:
            p = record['phases']
            profile_calls.append(dict(block=row['block'], slot=row['slot'], call=record['call'],
                spawn_wall_ns=p['spawn_ns'], handle_observation_wall_ns=p['handle_observation_ns'],
                wait_and_collect_wall_ns=p['wait_and_collect_ns'], terminal_observation_wall_ns=p['terminal_observation_ns'],
                phase_cpu_ns=None, absolute_monotonic_endpoints=None))
    distributions = {phase: dict(
        cpu_ns=stats([f['phases'][phase]['process_cpu_ns'] for f in frames]),
        zero_cpu_spans=sum(f['phases'][phase]['process_cpu_ns'] == 0 for f in frames),
        wall_lower_ns=stats([f['phases'][phase]['wall_lower_ns'] for f in frames]),
        wall_upper_ns=stats([f['phases'][phase]['wall_upper_ns'] for f in frames])) for phase in PHASES}
    return dict(owners=owners, frames=frames, blocks=blocks, coordinator_phase_totals=phase_totals(frames),
        coordinator_phase_distributions=distributions, profile_calls=profile_calls,
        arithmetic_complement={key: sum(f['arithmetic_complement'][key] for f in frames) for key in (*WALL, 'process_cpu_ns')},
        nested_timing_scopes_additive=False, worker_phase_cpu_available=False,
        reference_call_wall_phases=None, pure_api_cost_proved=False, zero_delta_means_zero_cost=False,
        causal_capture_cost_available=False, telemetry_on_off_compared=False,
        all_samples_retained=True, input_git_calls=64, new_workload_calls=0,
        workload_replayed=False, overhead_accepted=False, speedup_established=False, package_pass=False)


def run(label):
    require(re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', label), 'bounded output label required')
    folder = ROOT/'target/capture-attribution-20261026'/label
    folder.mkdir(parents=True, exist_ok=False)
    def write(name, value):
        with (folder/name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, sort_keys=True, indent=2)
            stream.write('\n')
    write('intent.json', dict(schema='hc074-offline-capture-attribution-v1', input_index_sha256=INDEX,
        input_source=SOURCE, input_git_calls=64, processes_launched_by_analyzer=0, clock_reads_by_analyzer=0,
        workload_replayed=False, retry=False, package_pass=False))
    result = dict(passed=False, first_refusal=None, workload_replayed=False, new_workload_calls=0, package_pass=False)
    try:
        result['analysis'] = analyze(load_packet(PACKET))
        result['passed'] = True
    except Exception as error:
        result['first_refusal'] = f'{type(error).__name__}: {error}'
    finally:
        write('result.json', result)
    return result


if __name__ == '__main__':
    require(len(sys.argv) == 3 and sys.argv[1] == 'analyze', 'usage: capture_attribution.py analyze NEW_LABEL')
    result = run(sys.argv[2])
    print(json.dumps(dict(passed=result['passed'], first_refusal=result['first_refusal'], new_workload_calls=0)))
    raise SystemExit(0 if result['passed'] else 1)
