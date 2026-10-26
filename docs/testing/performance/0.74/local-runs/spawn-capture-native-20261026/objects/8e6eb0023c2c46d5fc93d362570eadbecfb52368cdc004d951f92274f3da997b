"""Pure replay of one sealed leaf packet; no collector, process or clock API."""
import hashlib
import json
from pathlib import Path
import re
import sys

from capture_attribution import analyze as outer_analysis, REFERENCE, PROFILE
from git_cpu_variance import integer, require, stats

ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT/'docs/testing/performance/0.74/local-runs/capture-leaves-native-20261026'
INDEX = '926f30f0eca6ae708b3706976f256a9ca52a3a5d7e9a9c447a8f45cec59a0855'
SOURCE = 'ef3d0cadd70e436b75be94e579a605447aaf7e2b'
GROUPS = dict(owner=('owner_identity', 'owner_initial'),
              decode=('log_stat', 'log_read', 'text_decode', 'record_parse', 'protocol_validation', 'result_join'))
KEYS = ('wall_lower_ns', 'wall_upper_ns', 'process_cpu_ns')
CONTRACT = dict(schema='hc074-capture-leaves-v1', groups={g: list(p) for g, p in GROUPS.items()},
    max_additional_probes=16, max_additional_clock_reads=48, clock_domain='python-coordinator',
    cpu_scope='coordinator process-wide, all threads; not worker or native Git CPU',
    wall_bounds_scope='elapsed between bracketed CPU-read instants, not effect-only duration',
    owner_already_retained_before_first_leaf=True, wait_with_output_unchanged=True,
    native_create_call_wrapped=False, cleanup_instrumented=False, nested_scopes_additive=False,
    window_renewed=False, existing_predicates_removed_or_reordered=False, overhead_accepted=False, package_pass=False)


def load_packet(folder):
    raw = (folder/'index.json').read_bytes()
    require(hashlib.sha256(raw).hexdigest() == INDEX, 'fixed input index differs')
    index = json.loads(raw)
    require(index['source_commit'] == SOURCE and index['native_attempts'] == 1 and
            index['native_control_executed'] is True and index['workers'] == 16 and index['git_calls'] == 64 and
            all(index[k] is False for k in ('retry', 'overhead_accepted', 'package_pass', 'server')) and
            len(index['members']) == 164, 'fixed input scope differs')
    members = {}
    for name, entry in index['members'].items():
        require(re.fullmatch('[0-9a-f]{64}', entry['sha256']) and type(entry['bytes']) is int and
                0 <= entry['bytes'] <= 16*1024**2, 'unsafe raw object declaration')
        path = folder/'objects'/entry['sha256']
        require(path.stat().st_size == entry['bytes'], 'raw input size differs: '+name)
        body = path.read_bytes()
        require(hashlib.sha256(body).hexdigest() == entry['sha256'], 'raw input differs: '+name)
        members[name] = body
    require(json.loads(members['native/intent.json'])['capture_leaves'] ==
            dict(enabled=True, schema='hc074-capture-leaves-v1'), 'explicit leaf intent differs')
    result = json.loads(members['native/result.json'])
    logs = {}
    for row in result['attempts']:
        stem = f"native/{row['block']}-{row['slot']}"
        require(row == json.loads(members[stem+'-result.json']), 'individual ledger differs')
        log = members[stem+'-'+row['mode']+'.log']
        records = [json.loads(line[len(b'HC074_GIT_CPU '):]) for line in log.splitlines()
                   if line.startswith(b'HC074_GIT_CPU ')]
        require(records == row['records'], 'raw worker records differ')
        logs[(row['block'], row['slot'])] = log
    return result, logs


def strict_contract(value):
    require(value == CONTRACT and set(value) == set(CONTRACT) and
            all(type(value[k]) is type(v) for k, v in CONTRACT.items()), 'leaf contract differs')


def leaf_frame(row):
    bundle = row['capture_leaves']
    strict_contract(bundle['contract'])
    require(bundle['complete'] is bundle['capture_valid'] is True and bundle['first_refusal'] is None and
            set(bundle['groups']) == set(GROUPS), 'incomplete/refused leaf bundle')
    previous, leaves, complements = None, {}, {}
    for group, phases in GROUPS.items():
        tape = bundle['groups'][group]
        require(tape['group'] == group and tape['clock_domain'] == 'python-coordinator' and
                tape['complete'] is tape['capture_valid'] is True and tape['first_refusal'] is None and
                tape['not_observed'] == [] and len(tape['probes']) == len(phases)*2 and
                len(tape['spans']) == len(phases), 'incomplete/invalid leaf group')
        for i, probe in enumerate(tape['probes']):
            raw = probe['raw']
            require(probe['phase'] == phases[i//2] and probe['kind'] == ('end' if i % 2 else 'begin') and
                    set(raw) == {'wall_before_ns', 'process_cpu_ns', 'wall_after_ns'}, 'leaf probe order/shape differs')
            for value in raw.values():
                integer(value)
            require(raw['wall_after_ns'] >= raw['wall_before_ns'] and (previous is None or
                    (raw['wall_before_ns'] >= previous['wall_after_ns'] and
                     raw['process_cpu_ns'] >= previous['process_cpu_ns'])), 'leaf clock sequence regressed')
            previous = raw
        for i, phase in enumerate(phases):
            span = tape['spans'][i]
            require((span['phase'], span['status'], span['start_probe'], span['end_probe']) ==
                    (phase, 'complete', i*2, i*2+1) and
                    type(span['start_probe']) is type(span['end_probe']) is int, 'leaf span schedule differs')
            a, b = (tape['probes'][j]['raw'] for j in (i*2, i*2+1))
            elapsed = dict(wall_lower_ns=b['wall_before_ns']-a['wall_after_ns'],
                wall_upper_ns=b['wall_after_ns']-a['wall_before_ns'], process_cpu_ns=b['process_cpu_ns']-a['process_cpu_ns'])
            require(span['elapsed'] == elapsed and set(span['elapsed']) == set(KEYS) and
                    all(type(v) is int and v >= 0 for v in span['elapsed'].values()), 'leaf span arithmetic differs')
            leaves[phase] = elapsed
        outer = row['coordinator_frames']
        parent = next(s for s in outer['spans'] if s['phase'] == ('spawn' if group == 'owner' else 'decode'))
        a, b = (outer['probes'][i]['raw'] for i in (parent['start_probe'], parent['end_probe']))
        x, y = tape['probes'][0]['raw'], tape['probes'][-1]['raw']
        require(a['wall_after_ns'] <= x['wall_before_ns'] <= y['wall_after_ns'] <= b['wall_before_ns'] and
                a['process_cpu_ns'] <= x['process_cpu_ns'] <= y['process_cpu_ns'] <= b['process_cpu_ns'],
                'leaf group outside original parent')
        local = {k: sum(leaves[p][k] for p in phases) for k in KEYS}
        complement = dict(wall_lower_ns=parent['elapsed']['wall_lower_ns']-local['wall_upper_ns'],
            wall_upper_ns=parent['elapsed']['wall_upper_ns']-local['wall_lower_ns'],
            process_cpu_ns=parent['elapsed']['process_cpu_ns']-local['process_cpu_ns'])
        require(all(v >= 0 for v in complement.values()), 'leaf sum exceeds parent')
        complements[group] = complement
    return dict(block=row['block'], slot=row['slot'], mode=row['mode'], leaves=leaves,
                parent_complements=complements, complement_is_pure_api_cost=False)


def totals(frames, field):
    return {name: {k: sum(f[field][name][k] for f in frames) for k in KEYS} for name in frames[0][field]}


def delta(a, b):
    return dict(reference=a, profile=b, signed_cpu_delta_ns=b['process_cpu_ns']-a['process_cpu_ns'],
                signed_wall_delta_lower_ns=b['wall_lower_ns']-a['wall_upper_ns'],
                signed_wall_delta_upper_ns=b['wall_upper_ns']-a['wall_lower_ns'])


def analyze(result, logs):
    outer = outer_analysis(result)
    strict_contract(result['capture_leaves_contract'])
    require(set(logs) == {(b, s) for b in range(4) for s in range(4)} and
            all(type(v) is bytes and len(v) <= 16*1024**2 for v in logs.values()), 'log inventory differs')
    frames = [leaf_frame(row) for row in result['attempts']]
    for f, o, owner in zip(frames, outer['frames'], outer['owners']['workers']):
        f.update(log_bytes=len(logs[(f['block'], f['slot'])]), cpu_owners=owner['cpu'],
                 outer_phases=o['phases'], nested_worker_wall_ns=owner['recorded_phase_wall_ns'])
    blocks = []
    for block in range(4):
        selected = frames[block*4:block*4+4]
        parts = {}
        for field in ('leaves', 'parent_complements'):
            a, b = (totals([f for f in selected if f['mode'] == mode], field) for mode in (REFERENCE, PROFILE))
            parts[field] = {name: delta(a[name], b[name]) for name in a}
        for group, phases in GROUPS.items():
            parent = 'spawn' if group == 'owner' else 'decode'
            require(sum(parts['leaves'][p]['signed_cpu_delta_ns'] for p in phases) +
                    parts['parent_complements'][group]['signed_cpu_delta_ns'] ==
                    outer['blocks'][block]['coordinator_phases'][parent]['signed_cpu_delta_ns'],
                    'leaf CPU partition differs')
        blocks.append(dict(block=block, **parts))
    distributions = {p: dict(**{k: stats([f['leaves'][p][k] for f in frames]) for k in KEYS},
        zero_cpu_spans=sum(f['leaves'][p]['process_cpu_ns'] == 0 for f in frames))
        for phases in GROUPS.values() for p in phases}
    return dict(outer=outer, frames=frames, blocks=blocks, leaf_totals=totals(frames, 'leaves'),
        parent_complement_totals=totals(frames, 'parent_complements'), leaf_distributions=distributions,
        leaf_probes=256, total_recorder_probes=448, total_recorder_clock_reads=1344,
        original_metric_and_deadline_reads_additional=True,
        input_git_calls=64, new_workload_calls=0, all_samples_retained=True,
        nested_scopes_additive=False, reference_worker_phases=None, worker_phase_cpu_available=False,
        zero_delta_means_zero_work=False, pure_api_cost_proved=False, causal_read_size_relation_proved=False,
        telemetry_on_off_compared=False, workload_replayed=False, overhead_accepted=False,
        speedup_established=False, package_pass=False)


def run(label):
    require(re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', label), 'bounded output label required')
    folder = ROOT/'target/leaf-attribution-20261026'/label
    folder.mkdir(parents=True, exist_ok=False)
    def write(name, value):
        with (folder/name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, sort_keys=True, indent=2)
            stream.write('\n')
    write('intent.json', dict(schema='hc074-offline-leaf-attribution-v1', input_index_sha256=INDEX,
        input_source=SOURCE, input_git_calls=64, processes_launched_by_analyzer=0, clock_reads_by_analyzer=0,
        workload_replayed=False, retry=False, package_pass=False))
    result = dict(passed=False, first_refusal=None, workload_replayed=False, new_workload_calls=0, package_pass=False)
    try:
        result['analysis'] = analyze(*load_packet(PACKET))
        result['passed'] = True
    except Exception as error:
        result['first_refusal'] = f'{type(error).__name__}: {error}'
    finally:
        write('result.json', result)
    return result


if __name__ == '__main__':
    require(len(sys.argv) == 3 and sys.argv[1] == 'analyze', 'usage: leaf_attribution.py analyze NEW_LABEL')
    result = run(sys.argv[2])
    print(json.dumps(dict(passed=result['passed'], first_refusal=result['first_refusal'], new_workload_calls=0)))
    raise SystemExit(0 if result['passed'] else 1)
