"""Strict offline review of one finite direct-pipe diagnostic; no subprocesses."""
import json
import statistics

SPEC = '3e9bcc06ebce3439495200162cb0c1900b8eda53:crates/xtask/tests/diagnostic_artifact_fixture_074.rs'
BODY = '5819f89ec58a899e03d7c0f14c6af91f4be6248a26f8578186cfc47b5900a805'
ROW = {'worker', 'block', 'slot', 'variant', 'setup_ns', 'wall_ns', 'phases', 'stdout_bytes', 'stderr_bytes', 'code'}
PHASE = {'spawn_ns', 'handle_observation_ns', 'wait_and_collect_ns', 'terminal_observation_ns', 'child_start', 'child_end'}
TIMES = {'pid', 'created_100ns', 'exited_100ns', 'kernel_ns', 'user_ns'}
DONE = {'samples', 'workers', 'blocks', 'wall_ns', 'parent_cpu_ns', 'spec', 'body_sha256', 'product_candidate', 'installed_helper_changed', 'admission'}


def require(value, message):
    if not value:
        raise ValueError(message)


def number(value):
    require(type(value) is int and 0 <= value <= 2**64 - 1, 'bounded nonnegative integer')
    return value


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def read(raw):
    require(len(raw) <= 16 * 1024**2, 'log budget')
    rows, terminals = [], []
    for line in raw.decode('utf-8').splitlines():
        if line.startswith('HC074_PIPE '):
            require(not terminals, 'no samples after terminal')
            rows.append(json.loads(line.removeprefix('HC074_PIPE '), object_pairs_hook=unique))
        elif line.startswith('HC074_PIPE_DONE '):
            terminals.append(json.loads(line.removeprefix('HC074_PIPE_DONE '), object_pairs_hook=unique))
        else:
            require('HC074_PIPE' not in line, 'unsupported marker')
    require(len(rows) == 96 and len(terminals) == 1, 'one complete cohort')
    done = terminals[0]
    require(type(done) is dict and set(done) == DONE, 'terminal schema')
    require([number(done[k]) for k in ('samples', 'workers', 'blocks')] == [96, 4, 6], 'fixed schedule')
    require(done['spec'] == SPEC and done['body_sha256'] == BODY, 'independent reference')
    require(all(done[k] is False for k in ('product_candidate', 'installed_helper_changed', 'admission')), 'non-admission')
    number(done['wall_ns']); number(done['parent_cpu_ns'])
    seen, identities = set(), set()
    values = {key: [] for key in ('control_wall_ns', 'profile_wall_ns', 'spawn_ns', 'wait_and_collect_ns',
                                  'handle_observation_ns', 'terminal_observation_ns', 'child_cpu_ns')}
    for row in rows:
        require(type(row) is dict and set(row) == ROW, 'row schema')
        worker, block, slot = (number(row[k]) for k in ('worker', 'block', 'slot'))
        require(worker < 4 and block < 6 and slot < 4, 'schedule bounds')
        key = (worker, block, slot)
        require(key not in seen, 'duplicate slot'); seen.add(key)
        order = ('control', 'profile', 'profile', 'control') if block % 2 == 0 else ('profile', 'control', 'control', 'profile')
        require(row['variant'] == order[slot], 'preregistered ABBA/BAAB')
        require([number(row[k]) for k in ('stdout_bytes', 'stderr_bytes', 'code')] == [23589, 0, 0], 'output status/length')
        wall, setup = number(row['wall_ns']), number(row['setup_ns'])
        require(setup <= wall <= done['wall_ns'], 'wall containment')
        values[row['variant'] + '_wall_ns'].append(wall)
        phase = row['phases']
        if row['variant'] == 'control':
            require(phase is None, 'unobserved original control')
            continue
        require(type(phase) is dict and set(phase) == PHASE, 'phase schema')
        require(setup + sum(number(phase[k]) for k in PHASE if k.endswith('_ns')) <= wall, 'nonoverlapping wall brackets')
        start, end = phase['child_start'], phase['child_end']
        require(type(start) is dict and type(end) is dict and set(start) == set(end) == TIMES, 'process schema')
        for snapshot in (start, end):
            for v in snapshot.values(): number(v)
            require(snapshot['pid'] > 0 and snapshot['created_100ns'] > 0, 'process identity')
        require(start['pid'] == end['pid'] and start['created_100ns'] == end['created_100ns'], 'retained identity join')
        require(end['exited_100ns'] >= end['created_100ns'], 'terminal process')
        require(start['exited_100ns'] in (0, end['exited_100ns']), 'possible already exited child')
        require(start['kernel_ns'] <= end['kernel_ns'] and start['user_ns'] <= end['user_ns'], 'monotonic process CPU')
        identity = (end['pid'], end['created_100ns'])
        require(identity not in identities, 'unique retained child'); identities.add(identity)
        for name in values:
            if name in phase: values[name].append(phase[name])
        values['child_cpu_ns'].append(end['kernel_ns'] + end['user_ns'])
    require(len(seen) == 96 and len(identities) == 48, 'complete original schedule')
    return dict(samples=96, observed_children=48, cohort=done,
                medians_ns={key: statistics.median(v) for key, v in values.items()},
                totals_ns={key: sum(v) for key, v in values.items()},
                overhead_acceptance=False, pure_collection_attribution=False, product_admission=False)
