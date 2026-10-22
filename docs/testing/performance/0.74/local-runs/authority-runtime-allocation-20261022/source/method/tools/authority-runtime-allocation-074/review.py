"""Derive paired request/layout observations; no timing, RSS-win or ship admission."""
import argparse
import json
from pathlib import Path
import run as method

CELLS = [(depth, 'external') for depth in (1, 7, 31, 127)] + [(7, fault) for fault in ('validation', 'binding', 'transition', 'publication')]


def validate_counts(value):
    method.need(all(type(value[key]) is int and value[key] >= 0 for key in ('allocations', 'zeroed', 'reallocations', 'requested_bytes', 'deallocations', 'released_bytes', 'peak_delta')), 'Invalid allocation counter')
    method.need(type(value['live_delta']) is int and value['live_delta'] == value['requested_bytes'] - value['released_bytes'], 'Bad live requested-byte accounting')
    return value['live_delta']


def rows(raw):
    found = []
    for line in raw.splitlines():
        prefix, separator, payload = line.partition('HC074-RUNTIME-ALLOCATION ')
        if separator:
            method.need(prefix in ('', 'test ' + method.MEASURE + ' ... '), 'Unexpected measurement output prefix')
            found.append(json.loads(payload))
    method.need([(row['depth'], row['fault']) for row in found] == CELLS, 'Wrong allocation cell order/membership')
    for row in found:
        method.need(row['seed'] == 74 and row['iterations'] == 32 and row['owner_reconciled'] is True and row['original_objects_preserved'] is True, 'Workload/outcome changed')
        method.need(not row['server'] and not row['product'], 'Allocation scope drift')
        method.need(row['after_refusal']['scratch_capacity'] == row['after_refusal']['scratch_slots'] == 0, 'Scratch not evicted')
        total = 0
        for phase in ('open', 'warm', 'refusal', 'sticky', 'drop'):
            value = row[phase]
            total += validate_counts(value)
        method.need((row['max_publication'] is not None) == (row['depth'] == 127), 'Missing or unexpected maximum publication')
        if row['max_publication'] is not None:
            method.need(row['depth'] == 127 and row['max_publication']['state']['generations'] == 128, 'Wrong maximum publication')
            total += validate_counts(row['max_publication']['counts'])
        method.need(total == 0, 'Owner requested-layout lifetime leaked')
    return found


def metrics(row):
    return dict(warm_allocations_per_inspect=row['warm']['allocations'] / 32,
        warm_reallocations_per_inspect=row['warm']['reallocations'] / 32,
        warm_requested_bytes_per_inspect=row['warm']['requested_bytes'] / 32,
        open_allocations=row['open']['allocations'], open_requested_bytes=row['open']['requested_bytes'],
        live_requested_after_open=row['open']['live_delta'],
        live_requested_after_warm=row['open']['live_delta'] + row['warm']['live_delta'],
        scratch_after_warm=row['after_warm']['scratch_capacity'],
        live_requested_after_refusal=row['open']['live_delta'] + row['warm']['live_delta'] + row['refusal']['live_delta'] +
            (row['max_publication']['counts']['live_delta'] if row['max_publication'] is not None else 0),
        scratch_released_at_refusal=-row['refusal']['live_delta'],
        rss_after_warm=row['rss']['warm'], rss_after_drop=row['rss']['drop'])


def summarize(pairs):
    method.need(len(pairs) == 5, 'All five pairs required')
    for pair in pairs:
        for baseline, installed in zip(pair['baseline'], pair['installed']):
            for key in ('depth', 'fault', 'seed', 'iterations', 'corpus', 'root_name_bytes'):
                method.need(baseline[key] == installed[key], 'Unmatched pair: ' + key)
    result = []
    for index, (depth, fault) in enumerate(CELLS):
        a = [metrics(pair['baseline'][index]) for pair in pairs]
        b = [metrics(pair['installed'][index]) for pair in pairs]
        values = {key: dict(baseline=[min(row[key] for row in a), max(row[key] for row in a)],
            installed=[min(row[key] for row in b), max(row[key] for row in b)],
            paired_delta_installed_minus_baseline=[min(right[key] - left[key] for left, right in zip(a, b)),
                max(right[key] - left[key] for left, right in zip(a, b))]) for key in a[0]}
        result.append(dict(depth=depth, fault=fault, pairs=5, metrics=values,
            allocation_counts_repeatable=all(values[key][role][0] == values[key][role][1] for key in values if not key.startswith('rss') for role in ('baseline', 'installed'))))
    return result


def facts(folder):
    receipt = json.loads((folder / 'result.json').read_text())
    method.need(receipt['failure'] is None and receipt['measure_before'] == receipt['measure_after'], 'Failed or changed measurement context')
    method.need(receipt['runtime_sources'] == dict(baseline=method.BASE, installed=method.CURRENT), 'Runtime source drift')
    method.need(not receipt['measure_before']['dirty'] and receipt['measure_before']['qualification'] == method.QUAL, 'Unfrozen measurement')
    method.need(not any(receipt[key] for key in ('server', 'product', 'live', 'clocks_changed', 'services_changed')), 'Scope drift')
    records = {}
    for path in sorted(folder.glob('*.json')):
        step = json.loads(path.read_text())
        if 'argv' not in step:
            continue
        method.need(step['exit'] == 0 and not step['timeout'], 'First step refused')
        for suffix in ('stdout', 'stderr'):
            method.need(method.sha(folder / (step['name'] + '.' + suffix + '.log')) == step[suffix + '_sha256'], 'Raw log changed')
        records[step['name']] = step
    method.need(len(records) == 46, 'Build/control/measurement membership changed')
    expected = set()
    for lane in ('194', '188'):
        expected.add(lane + '-compiler')
        for role in ('baseline', 'installed'):
            prefix = lane + '-' + role
            expected.update(prefix + suffix for suffix in ('-build', '-clippy'))
            for index, test in enumerate(method.CONTROLS):
                name = prefix + '-control-' + str(index)
                expected.add(name)
                method.need(records[name]['argv'] == [receipt['images'][lane][role]['path'], test,
                    '--exact', '--nocapture', '--test-threads=1'], 'Wrong control image/test flags')
                method.passed((folder / (name + '.stdout.log')).read_bytes(), test)
            expected.update(lane + '-pair-' + str(pair) + '-' + role for pair in range(5))
    method.need(set(records) == expected, 'Wrong exact build/control/profile membership')
    lanes = {}
    for lane in ('194', '188'):
        pairs = []
        for pair in range(5):
            selected = {}
            for role in ('baseline', 'installed'):
                name = lane + '-pair-' + str(pair) + '-' + role
                step = records[name]
                method.need(step['argv'] == [receipt['images'][lane][role]['path'], method.MEASURE, '--exact', '--nocapture', '--test-threads=1'], 'Wrong measured image/test flags')
                raw = (folder / (name + '.stdout.log')).read_bytes()
                method.passed(raw, method.MEASURE)
                selected[role] = rows(raw.decode())
            pairs.append(selected)
        lanes[lane] = summarize(pairs)
    return dict(schema='authority-runtime-allocation-facts-074-v1', method=receipt['measure_before']['head'],
        runtime_sources=receipt['runtime_sources'], images=receipt['images'], lanes=lanes,
        pairs_per_compiler=5, operations_per_warm_cell=32, source_overlay=True, rust_layout_requests_only=True,
        rss_supplemental=True, timing_claim=False, product_claim=False, server_admitted=False)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    value = facts(args.folder)
    if args.output:
        method.publish(args.output, value)
    print(json.dumps(value, sort_keys=True))
