"""Offline reader for complete native v2 cohorts; synthetic transcripts cannot qualify."""
import argparse
import json
import math
from pathlib import Path
import balanced as m
from runner import validate_context, validate_host


def summarize(observations, expected):
    m.validate_schedule([value['selection'] for value in observations])
    for value in observations:
        m.need(m.signature(value['rows']) == expected, 'Unmatched block workload/corpus')
    lanes = {}
    for lane in m.COMPILERS:
        cells = []
        for index, (mode, depth, fault) in enumerate(m.cpu_review.CELLS):
            blocks = []
            for block in range(8):
                roles = {role: [m.cpu_review.metrics(value['rows'][index]) for value in observations
                         if value['selection']['compiler'] == lane and value['selection']['block'] == block and
                         m.ROLE[value['selection']['role']] == role] for role in m.ROLE.values()}
                m.need(all(len(values) == 2 for values in roles.values()), 'Incomplete block')
                blocks.append({key: m.block_comparison([v[key] for v in roles['baseline']],
                              [v[key] for v in roles['installed']]) for key in roles['baseline'][0]})
            cells.append(dict(mode=mode, depth=depth, fault=fault, metrics={key:
                dict(blocks=[block[key] for block in blocks], **m.interval([block[key]['ratio'] for block in blocks]))
                for key in blocks[0]}))
        lanes[lane] = cells
    primaries = {}
    for label, mode, metric in (('warm_cpu', 'phases', 'warm_cpu_ns_per_inspect'),
                                ('cycle_elapsed', 'cycle', 'cycle_wall_ns')):
        values = {lane: next(row for row in cells if (row['mode'], row['depth'], row['fault']) ==
                  (mode, 127, 'external'))['metrics'][metric] for lane, cells in lanes.items()}
        if all(m.beyond_boundary(block, 'benefit') for v in values.values() for block in v['blocks']): disposition = 'clear-local-benefit'
        elif all(m.beyond_boundary(block, 'cost') for v in values.values() for block in v['blocks']): disposition = 'clear-local-cost'
        else: disposition = 'no-clear-practical-effect-or-mixed'
        primaries[label] = dict(lanes=values, disposition=disposition, practical_boundary=0.05,
                                iid_assumption_required=True, release_admission=False)
    return dict(schema='authority-runtime-balanced-facts-074-v2', lanes=lanes, primary=primaries,
                schedule_sha256=m.SCHEDULE_SHA, rows=768, processes=64, server_admitted=False, product_claim=False)


def load(path): return json.loads(path.read_text())


def build_commands(lane, owned_root):
    common = ['--release', '--offline', '--locked', '-p', 'hydracache-long-run-supervisor-074']
    cargo = ['/home/artur/.cargo/bin/cargo', '+' + m.COMPILERS[lane]]
    result = {lane + '-compiler': (['/home/artur/.cargo/bin/rustc', '+' + m.COMPILERS[lane], '--version'], None)}
    for role in m.ROLE.values():
        cwd = owned_root + '/' + role; name = lane + '-' + role
        result[name + '-build'] = (cargo + ['test'] + common + ['--lib', '--no-run', '--message-format=json'], cwd)
        result[name + '-clippy'] = (cargo + ['clippy'] + common + ['--lib', '--tests', '--', '-D', 'warnings'], cwd)
        result[name + '-check'] = (cargo + ['check'] + common + ['--lib', '--tests'], cwd)
    return result


def verify(folder):
    expected = m.reference(); result = load(folder / 'result.json'); cohort = folder / 'cohort'
    m.need(result['failure'] is None and result['plan'] == m.plan(), 'Refused/changed v2 attempt')
    m.need(result['runtime_sources'] == m.plan()['runtime_sources'] and result['allocation_wrapper'] is False and
           all(result[key] is False for key in ('server', 'product', 'live', 'clocks_changed', 'services_changed')),
           'Scope drift')
    reference = result['measure_before']
    validate_host(reference)
    m.need(reference == result['measure_after'] and reference['head'] == result['method'] and not reference['dirty'] and
           reference['qualification'] == m.common.QUAL and result['cpu'] == reference['affinity'][0] and
           set(reference['namespaces']) == {'mnt', 'pid', 'user', 'cgroup', 'net', 'ipc', 'uts', 'time'}, 'Host/method drift')
    m.need({k: v for k, v in result['build_before'].items() if k not in ('head', 'dirty')} ==
           {k: v for k, v in reference.items() if k not in ('head', 'dirty')}, 'Build/measure drift')
    m.need(set(result['tool_hashes']) == set(m.INPUTS), 'Incomplete method input membership')
    for name, digest in result['tool_hashes'].items():
        m.common.member(name)
        m.need(m.common.sha(folder / 'compiled-inputs' / name) == digest, 'Compiled input changed')
    images = result['images']; m.validate_images(images)
    for lane, roles in images.items():
        for role, image in roles.items():
            m.need(image['path'].startswith(result['owned_root'] + '/target-' + lane + '-' + role + '/'),
                   'Image escaped its compiler/role target')
    commands = {}
    for path in folder.glob('*.json'):
        row = load(path)
        if 'argv' not in row: continue
        m.need(row['name'] == path.stem and row['exit'] == 0 and row['timeout'] is False, 'First build/control refused')
        for suffix in ('stdout', 'stderr'):
            m.need(m.common.sha(folder / (row['name'] + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'Build log changed')
        commands[row['name']] = row
    required = set()
    for lane in m.COMPILERS:
        required.add(lane + '-compiler')
        for name, (argv, cwd) in build_commands(lane, result['owned_root']).items():
            m.need(commands[name]['argv'] == argv and (cwd is None or commands[name]['cwd'] == cwd),
                   'Build/check/compiler command drift')
        for role in m.ROLE.values():
            name = lane + '-' + role
            required.update(name + suffix for suffix in ('-build', '-clippy', '-check'))
            m.need(m.common.image((folder / (name + '-build.stdout.log')).read_bytes()) == images[lane][role]['path'],
                   'Not a fresh selected image')
            for index, test in enumerate(m.cpu.CONTROLS):
                control = name + '-control-' + str(index); required.add(control)
                m.need(commands[control]['argv'] == m.cpu.argv(result['cpu'], images[lane][role]['path'], test), 'Control command drift')
                m.common.passed((folder / (control + '.stdout.log')).read_bytes(), test)
    m.need(set(commands) == required and len(commands) == 22, 'Build/control command membership drift')
    terminal = load(cohort / 'result.json')
    m.need(terminal['mode'] == 'native' and terminal['failure'] is None and
           type(terminal['completed']) is type(terminal['attempted']) is int and
           terminal['completed'] == terminal['attempted'] == 64 and terminal['schedule_sha256'] == m.SCHEDULE_SHA,
           'Synthetic, refused or incomplete cohort is diagnostic only')
    start, end, deadline = (terminal[key] for key in ('started_monotonic', 'finished_monotonic', 'deadline_monotonic'))
    m.need(all(type(v) in (int, float) and math.isfinite(v) for v in (start, end, deadline)) and
           start <= end < deadline and deadline - start == m.COHORT_SECONDS, 'Cohort budget drift')
    m.need(load(cohort / 'plan.json') == m.plan(), 'Cohort plan changed')
    members = {'result.json', 'plan.json'}; observations = []; previous = 0
    for ordinal, selection in enumerate(m.schedule()):
        prefix = f'{ordinal:03d}'; members.update((prefix + '-step.json', prefix + '-profile.json',
            prefix + '-profile.stdout.log', prefix + '-profile.stderr.log'))
        step = load(cohort / (prefix + '-step.json')); command = load(cohort / (prefix + '-profile.json'))
        m.need(type(step['ordinal']) is int and step['ordinal'] == ordinal and
               all(type(step['selection'][key]) is int for key in ('block', 'position')) and
               step['selection'] == selection and step['stage'] == 'complete' and
               step['failure'] is None and command['error'] is None and command['exit'] == 0 and command['timeout'] is False,
               'First profile step refused/reordered')
        image = images[selection['compiler']][m.ROLE[selection['role']]]
        m.need(command['argv'] == m.cpu.argv(result['cpu'], image['path'], m.cpu.MEASURE) and
               command['cwd'] == result['owned_root'] + '/' + m.ROLE[selection['role']], 'Profile image/cwd drift')
        m.need(all(type(step[key]) in (int, float) and math.isfinite(step[key]) for key in ('timeout_seconds', 'launch_monotonic')) and
               0 < step['timeout_seconds'] <= m.PROCESS_SECONDS and start <= step['launch_monotonic'] < deadline and
               step['timeout_seconds'] == min(m.PROCESS_SECONDS, deadline - step['launch_monotonic']), 'Child budget drift')
        for suffix in ('stdout', 'stderr'):
            m.need(m.common.sha(cohort / (prefix + '-profile.' + suffix + '.log')) == command[suffix + '_sha256'], 'Raw profile log changed')
        raw = (cohort / (prefix + '-profile.stdout.log')).read_bytes()
        header, rows = m.parse(raw, expected)
        m.need(header == step['clock_metadata'], 'Clock metadata changed')
        for key in ('before', 'after'): validate_context(step[key], result['cpu'])
        process = step['process']
        m.need(all(process[key] == command[key] for key in ('pid', 'start_raw_ns', 'spawned_raw_ns', 'exit_raw_ns')) and
               all(type(process[key]) is int for key in ('start_raw_ns', 'spawned_raw_ns', 'exit_raw_ns')) and
               type(process['pid']) is int and process['pid'] > 0 and previous <= step['before']['start_raw_ns'] and
               step['before']['end_raw_ns'] <= process['start_raw_ns'] <= process['spawned_raw_ns'] <= process['exit_raw_ns'] <=
               step['after']['start_raw_ns'], 'Parent timeline drift')
        previous = step['after']['end_raw_ns']
        observations.append(dict(selection=selection, rows=rows))
    m.need(set(p.name for p in cohort.iterdir() if p.is_file()) == members and
           not any(p.is_symlink() or p.is_dir() for p in cohort.iterdir()), 'Unexpected cohort members')
    return summarize(observations, expected)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path); parser.add_argument('--output', type=Path)
    args = parser.parse_args(); value = verify(args.folder)
    if args.output: m.common.publish(args.output, value)
    print(json.dumps(dict(processes=64, rows=768, primary=value['primary'], product_claim=False), sort_keys=True))
