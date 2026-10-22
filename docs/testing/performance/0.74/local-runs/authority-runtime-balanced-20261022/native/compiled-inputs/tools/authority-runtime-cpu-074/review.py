"""Offline paired CPU/elapsed attribution. No product or server admission."""
import argparse
import json
import math
from pathlib import Path
import re
import statistics
import run as method

need = method.common.need
CELLS = [('phases', depth, 'external') for depth in (1, 7, 31, 127)] + [
    ('phases', 7, fault) for fault in ('validation', 'binding', 'transition', 'publication')] + [
    ('cycle', depth, 'external') for depth in (1, 7, 31, 127)]
FIELDS = ('wall_ns', 'cpu_ns', 'user_ns', 'system_ns', 'voluntary', 'involuntary', 'inblock', 'oublock')


def costs(value):
    need(set(value) == set(FIELDS) and all(type(value[key]) is int and value[key] >= 0 for key in FIELDS), 'Invalid timing/counter record')
    need(value['wall_ns'] > 0 and value['cpu_ns'] > 0, 'Unresolved or reversed clock sample')


def prefixed(raw, marker):
    rows = []
    for line in raw.splitlines():
        prefix, separator, value = line.partition(marker)
        if separator:
            need(prefix in ('', 'test ' + method.MEASURE + ' ... '), 'Unexpected libtest prefix')
            rows.append(json.loads(value))
    return rows


def parse(raw):
    headers = prefixed(raw, 'HC074-RUNTIME-CPU-CLOCKS ')
    need(len(headers) == 1, 'One clock metadata record required')
    header = headers[0]
    need(all(type(header[key]) is int and header[key] > 0 for key in ('wall_resolution_ns', 'cpu_resolution_ns')),
        'Invalid clock resolution')
    need(len(header['empty']) == 8, 'Empty-bracket membership changed')
    for value in header['empty']: costs(value)
    rows = prefixed(raw, 'HC074-RUNTIME-CPU ')
    need([(row['mode'], row['depth'], row['fault']) for row in rows] == CELLS, 'CPU cell order/membership drift')
    for row in rows:
        need(row['seed'] == 74 and row['iterations'] == 32 and row['outcome_checked'] is True and
            re.fullmatch('[0-9a-f]{64}', row['corpus']) and type(row['root_name_bytes']) is int and row['root_name_bytes'] > 0,
            'CPU workload/outcome drift')
        if row['mode'] == 'phases':
            need(row['scratch_cleared'] is True and row['original_objects_preserved'] is True, 'CPU phase semantic refusal')
            for phase in ('open', 'warm', 'refusal', 'sticky', 'drop'): costs(row[phase])
            need((row['publication'] is not None) == (row['depth'] == 127), 'Maximum publication changed')
            if row['publication'] is not None: costs(row['publication'])
        else: costs(row['cycle'])
    return header, rows


def metrics(row):
    if row['mode'] == 'cycle': return {'cycle_' + key: row['cycle'][key] for key in FIELDS}
    result = {}
    for phase in ('open', 'warm', 'publication', 'refusal', 'sticky', 'drop'):
        if row[phase] is None: continue
        for key in FIELDS:
            result[phase + '_' + key + ('_per_inspect' if phase == 'warm' else '')] = row[phase][key] / (32 if phase == 'warm' else 1)
    return result


def comparison(baseline, installed):
    need(len(baseline) == len(installed) == 7 and all(type(value) in (int, float) and math.isfinite(value) and value >= 0
        for value in baseline + installed), 'Seven finite paired values required')
    ratios = [right / left for left, right in zip(baseline, installed)] if all(value > 0 for value in baseline) else None
    return dict(baseline=dict(min=min(baseline), median=statistics.median(baseline), max=max(baseline)),
        installed=dict(min=min(installed), median=statistics.median(installed), max=max(installed)),
        paired_deltas=[right - left for left, right in zip(baseline, installed)],
        paired_ratios=ratios, median_ratio=statistics.median(ratios) if ratios else None,
        median_interval=[min(ratios), max(ratios)] if ratios else None,
        nominal_coverage=1 - 2 / 2 ** 7 if ratios else None)


def summarize(pairs):
    need(len(pairs) == 7, 'All seven pairs required, no selection')
    for pair in pairs:
        for left, right in zip(pair['baseline'], pair['installed']):
            for key in ('mode', 'depth', 'fault', 'seed', 'iterations', 'corpus', 'root_name_bytes'):
                need(left[key] == right[key], 'Unmatched CPU pair: ' + key)
    result = []
    for index, (mode, depth, fault) in enumerate(CELLS):
        left = [metrics(pair['baseline'][index]) for pair in pairs]
        right = [metrics(pair['installed'][index]) for pair in pairs]
        result.append(dict(mode=mode, depth=depth, fault=fault, pairs=7, metrics={key:
            comparison([value[key] for value in left], [value[key] for value in right]) for key in left[0]}))
    return result


def primary(lanes, mode, metric):
    values = {lane: next(row for row in cells if (row['mode'], row['depth'], row['fault']) == (mode, 127, 'external'))['metrics'][metric]
        for lane, cells in lanes.items()}
    if all(value['median_interval'][1] < 0.95 for value in values.values()): disposition = 'clear-local-benefit'
    elif all(value['median_interval'][0] > 1.05 for value in values.values()): disposition = 'clear-local-cost'
    else: disposition = 'no-clear-practical-effect-or-mixed'
    return dict(metric=metric, practical_boundary=0.05, disposition=disposition, lanes=values,
        release_admission=False, inference_requires_independent_identically_distributed_pairs=True)


def facts(folder):
    receipt = json.loads((folder / 'result.json').read_text())
    need(receipt['failure'] is None and receipt['measure_before'] == receipt['measure_after'], 'CPU run/context refused')
    need(receipt['runtime_sources'] == dict(baseline=method.common.BASE, installed=method.common.CURRENT), 'CPU source drift')
    need(receipt['measure_before']['qualification'] == method.common.QUAL and not receipt['measure_before']['dirty'], 'CPU method not frozen')
    need(receipt['helper_sha256'] == method.HELPER_SHA and receipt['cpu'] == receipt['measure_before']['affinity'][0], 'CPU placement/helper drift')
    need(not any(receipt[key] for key in ('server', 'product', 'live', 'clocks_changed', 'services_changed', 'allocation_wrapper')), 'CPU scope drift')
    for name, digest in receipt['tool_hashes'].items():
        need(method.common.sha(folder / 'compiled-inputs' / name) == digest, 'Compiled CPU input changed')
    records = {}
    for path in folder.glob('*.json'):
        record = json.loads(path.read_text())
        if 'argv' not in record: continue
        need(record['exit'] == 0 and not record['timeout'], 'First CPU step refused')
        for suffix in ('stdout', 'stderr'):
            need(method.common.sha(folder / (record['name'] + '.' + suffix + '.log')) == record[suffix + '_sha256'], 'CPU raw log changed')
        records[record['name']] = record
    expected = set(); lanes = {}; calibration = {}
    for lane in ('194', '188'):
        expected.add(lane + '-compiler')
        images = receipt['images'][lane]
        need(images['baseline']['path'] != images['installed']['path'] and images['baseline']['sha256'] != images['installed']['sha256'], 'CPU image reuse')
        for role in ('baseline', 'installed'):
            prefix = lane + '-' + role
            expected.update(prefix + suffix for suffix in ('-build', '-clippy'))
            build = (folder / (prefix + '-build.stdout.log')).read_bytes()
            need(method.common.image(build) == images[role]['path'], 'CPU image was not freshly built')
            for index, test in enumerate(method.CONTROLS):
                name = prefix + '-control-' + str(index); expected.add(name)
                need(records[name]['argv'] == method.argv(receipt['cpu'], images[role]['path'], test), 'CPU control command drift')
                method.common.passed((folder / (name + '.stdout.log')).read_bytes(), test)
        pairs = []; calibration[lane] = []
        for pair in range(7):
            selected = {}; headers = {}
            for role in ('baseline', 'installed'):
                name = lane + '-pair-' + str(pair) + '-' + role; expected.add(name)
                need(records[name]['argv'] == method.argv(receipt['cpu'], images[role]['path'], method.MEASURE), 'CPU profile command drift')
                raw = (folder / (name + '.stdout.log')).read_bytes(); method.common.passed(raw, method.MEASURE)
                headers[role], selected[role] = parse(raw.decode())
            pairs.append(selected); calibration[lane].append(headers)
        lanes[lane] = summarize(pairs)
    need(set(records) == expected and len(records) == 46, 'CPU exact step membership drift')
    return dict(schema='authority-runtime-cpu-facts-074-v1', method=receipt['measure_before']['head'],
        runtime_sources=receipt['runtime_sources'], lanes=lanes, calibration=calibration,
        primary_warm_cpu=primary(lanes, 'phases', 'warm_cpu_ns_per_inspect'),
        primary_cycle_elapsed=primary(lanes, 'cycle', 'cycle_wall_ns'), pairs_per_compiler=7,
        operations_per_warm_cell=32, raw_qualification=method.common.QUAL, allocation_wrapper=False,
        product_claim=False, production_end_to_end_claim=False, server_admitted=False)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path); parser.add_argument('--output', type=Path)
    args = parser.parse_args(); value = facts(args.folder)
    if args.output: method.common.publish(args.output, value)
    print(json.dumps(dict(method=value['method'], warm_cpu=value['primary_warm_cpu'], cycle_elapsed=value['primary_cycle_elapsed']), sort_keys=True))
