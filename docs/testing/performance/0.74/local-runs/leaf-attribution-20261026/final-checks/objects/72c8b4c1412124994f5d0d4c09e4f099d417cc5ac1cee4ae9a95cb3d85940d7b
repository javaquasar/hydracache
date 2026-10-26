"""Independent fixed-byte arithmetic replay; no analyzer/collector imports."""
import hashlib
import json
from pathlib import Path

INDEX = '70ba30460b8e4c995c23d3f37090b0ef849bc2915e6507e62dfc5b23a7837417'
INPUT = '926f30f0eca6ae708b3706976f256a9ca52a3a5d7e9a9c447a8f45cec59a0855'
SOURCE = '10f160bef32c02171d42f0f8138acbbbc4d8695d'
GROUPS = dict(owner=('owner_identity', 'owner_initial'),
              decode=('log_stat', 'log_read', 'text_decode', 'record_parse', 'protocol_validation', 'result_join'))
KEYS = ('wall_lower_ns', 'wall_upper_ns', 'process_cpu_ns')
REFERENCE, PROFILE = 'reference_cpu_child', 'accounted_cpu_child'
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def load(folder):
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['source_commit'] == SOURCE and index['input_index_sha256'] == INPUT and
            index['checks'] == 7 and index['input_git_calls'] == 64 and index['new_workload_calls'] == 0 and
            index['new_native_attempts'] == 0 and len(index['members']) == 73 and
            all(index[k] is False for k in ('final_checks', 'workload_replayed', 'overhead_accepted', 'package_pass', 'server')),
            'scope differs')
    members = {}
    for name, entry in index['members'].items():
        raw = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(raw) == entry['bytes'] and sha(raw) == entry['sha256'], 'raw member differs: '+name)
        members[name] = raw
    return members


def audit(members):
    def obj(name):
        return json.loads(members[name])
    require(sha(members['native-index.json']) == INPUT, 'native index differs')
    baseline = obj('native-index.json')
    require(baseline['source_commit'] == 'ef3d0cadd70e436b75be94e579a605447aaf7e2b', 'native source differs')
    for name, entry in baseline['members'].items():
        if name.startswith('native/'):
            require(len(members[name]) == entry['bytes'] and sha(members[name]) == entry['sha256'], 'native join differs')
    labels = ('leaf-attribution-models-first', 'leaf-attribution-regressions', 'leaf-attribution-syntax',
        'leaf-attribution-offline-first', 'leaf-attribution-check', 'leaf-attribution-clippy', 'leaf-attribution-format')
    sources = []
    for label in labels:
        before, after = (obj('checks/'+label+'/'+part) for part in ('intent.json', 'result.json'))
        expected_head = 'c454fb982831242082f77b60c7ca6dc30a1f9425' if label in labels[:3] else SOURCE
        require(before['head'] == expected_head and after['source_unchanged'] is after['head_unchanged'] is True and
                before['source'] == after['source_after'] and after['outer_refusal'] is None,
                'source receipt differs')
        require(before['bound_seconds'] == 1200 and before['disk_floor_bytes'] == 4*1024**3 and
                all(before[k] is False for k in ('dependency_changes', 'observation_frequency_changed',
                    'services_or_clocks', 'server', 'qualification', 'retry')), 'check scope differs')
        require(after['exit'] == (1 if label == labels[-1] else 0), 'first check exit differs')
        sources.append(before['source'])
    require(all(s == sources[0] for s in sources), 'check inputs differ')
    require(b'os error 206' in members['checks/leaf-attribution-format/stderr.log'] +
            members['checks/leaf-attribution-format/stdout.log'], 'first formatting refusal missing')
    for name, raw in members.items():
        if name.startswith('source/'):
            require(sha(raw) == sources[0][name[7:]], 'source object differs')
    require(sha(members['qualification-manifest.toml']) ==
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification differs')
    intent, saved = obj('offline/intent.json'), obj('offline/result.json')
    require(intent == dict(schema='hc074-offline-leaf-attribution-v1', input_index_sha256=INPUT,
        input_source=baseline['source_commit'], input_git_calls=64, processes_launched_by_analyzer=0,
        clock_reads_by_analyzer=0, workload_replayed=False, retry=False, package_pass=False), 'offline intent differs')
    require(saved['passed'] is True and saved['first_refusal'] is None and saved['new_workload_calls'] == 0 and
            saved['workload_replayed'] is saved['package_pass'] is False, 'offline verdict differs')
    data, a = obj('native/result.json'), saved['analysis']
    require(len(a['frames']) == 16 and len(a['blocks']) == 4 and len(a['outer']['profile_calls']) == 32 and
            a['leaf_probes'] == 256 and a['total_recorder_probes'] == 448 and
            a['total_recorder_clock_reads'] == 1344 and a['input_git_calls'] == 64 and a['new_workload_calls'] == 0 and
            a['all_samples_retained'] is a['original_metric_and_deadline_reads_additional'] is True,
            'sample scope differs')
    require(all(a[k] is False for k in ('nested_scopes_additive', 'worker_phase_cpu_available',
        'zero_delta_means_zero_work', 'pure_api_cost_proved', 'causal_read_size_relation_proved',
        'telemetry_on_off_compared', 'workload_replayed', 'overhead_accepted', 'speedup_established', 'package_pass')) and
        a['reference_worker_phases'] is None, 'analysis promoted')
    frames = []
    for row, frame in zip(data['attempts'], a['frames']):
        require((row['block'], row['slot'], row['mode']) == (frame['block'], frame['slot'], frame['mode']), 'order differs')
        leaves, complements = {}, {}
        for group, phases in GROUPS.items():
            tape = row['capture_leaves']['groups'][group]
            for i, phase in enumerate(phases):
                x, y = (tape['probes'][j]['raw'] for j in (i*2, i*2+1))
                elapsed = dict(wall_lower_ns=y['wall_before_ns']-x['wall_after_ns'],
                    wall_upper_ns=y['wall_after_ns']-x['wall_before_ns'],
                    process_cpu_ns=y['process_cpu_ns']-x['process_cpu_ns'])
                require(elapsed == tape['spans'][i]['elapsed'] and all(v >= 0 for v in elapsed.values()), 'raw leaf differs')
                leaves[phase] = elapsed
            parent = next(s['elapsed'] for s in row['coordinator_frames']['spans']
                          if s['phase'] == ('spawn' if group == 'owner' else 'decode'))
            complements[group] = dict(wall_lower_ns=parent['wall_lower_ns']-sum(leaves[p]['wall_upper_ns'] for p in phases),
                wall_upper_ns=parent['wall_upper_ns']-sum(leaves[p]['wall_lower_ns'] for p in phases),
                process_cpu_ns=parent['process_cpu_ns']-sum(leaves[p]['process_cpu_ns'] for p in phases))
        job, worker = row['job'], row['worker']
        j, w = job['user_ns']+job['kernel_ns'], (worker['user_100ns']+worker['kernel_100ns'])*100
        c = row['metrics']['coordinator_cpu_ns']
        owners = dict(job_cpu_ns=j, worker_cpu_ns=w, descendant_cpu_ns=j-w, coordinator_cpu_ns=c, recorded_cpu_envelope_ns=j+c)
        nested = {k: sum(r['phases'][k] for r in row['records']) for k in
            ('spawn_ns', 'handle_observation_ns', 'wait_and_collect_ns', 'terminal_observation_ns')} if row['mode'] == PROFILE else None
        require(frame['leaves'] == leaves and frame['parent_complements'] == complements and frame['cpu_owners'] == owners and
                frame['nested_worker_wall_ns'] == nested and frame['complement_is_pure_api_cost'] is False and
                frame['log_bytes'] == len(members[f"native/{row['block']}-{row['slot']}-{row['mode']}.log"]) and
                frame['outer_phases'] == {s['phase']: s['elapsed'] for s in row['coordinator_frames']['spans']},
                'worker projection differs')
        frames.append(dict(block=row['block'], mode=row['mode'], leaves=leaves, parent_complements=complements))
    def sums(selected, field):
        return {p: {k: sum(f[field][p][k] for f in selected) for k in KEYS} for p in selected[0][field]}
    for field, output in (('leaves', 'leaf_totals'), ('parent_complements', 'parent_complement_totals')):
        require(a[output] == sums(frames, field), 'totals differ')
    for p in a['leaf_totals']:
        expected = {k: dict(count=16, sum=sum(v), minimum=min(v), maximum=max(v)) for k in KEYS
                    for v in [[f['leaves'][p][k] for f in frames]]}
        expected['zero_cpu_spans'] = sum(f['leaves'][p]['process_cpu_ns'] == 0 for f in frames)
        require(a['leaf_distributions'][p] == expected, 'distribution differs')
    for block, summary in enumerate(a['blocks']):
        require(summary['block'] == block, 'block order differs')
        for field in ('leaves', 'parent_complements'):
            x, y = (sums([f for f in frames if f['block'] == block and f['mode'] == mode], field)
                    for mode in (REFERENCE, PROFILE))
            expected = {p: dict(reference=x[p], profile=y[p], signed_cpu_delta_ns=y[p]['process_cpu_ns']-x[p]['process_cpu_ns'],
                signed_wall_delta_lower_ns=y[p]['wall_lower_ns']-x[p]['wall_upper_ns'],
                signed_wall_delta_upper_ns=y[p]['wall_upper_ns']-x[p]['wall_lower_ns']) for p in x}
            require(summary[field] == expected, 'block arithmetic differs')
        for group, phases in GROUPS.items():
            parent = 'spawn' if group == 'owner' else 'decode'
            selected = data['attempts'][block*4:block*4+4]
            x, y = (sum(next(s['elapsed']['process_cpu_ns'] for s in f['coordinator_frames']['spans'] if s['phase'] == parent)
                        for f in selected if f['mode'] == mode) for mode in (REFERENCE, PROFILE))
            require(sum(summary['leaves'][p]['signed_cpu_delta_ns'] for p in phases) +
                    summary['parent_complements'][group]['signed_cpu_delta_ns'] == y-x, 'CPU partition differs')
    return dict(workers=16, input_git_calls=64, new_workload_calls=0, checks=7, checks_passed=6,
                retained_format_refusals=1, leaf_totals=a['leaf_totals'], parent_complements=a['parent_complement_totals'],
                overhead_accepted=False, package_pass=False)


def verify(folder):
    return audit(load(folder))


if __name__ == '__main__':
    print(json.dumps(verify(Path(__file__).resolve().parent), sort_keys=True, indent=2))
