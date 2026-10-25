"""Strict offline helper trace join. Intervals are never target/CPU partitions."""
import json

import review

OPERATIONS = ('git', 'output', 'decode', 'read')
COMMANDS = ('show', 'ls-tree', 'ls-files', 'hash-object', 'check-attr')


def trace_frame(frame):
    require = review.require
    require(frame['schema'] == 'hc074-helper-trace-v1' and frame['valid'] is True,
            'invalid helper trace')
    require(frame['nested_intervals'] is True and frame['product'] is False and frame['package_replay'] is False,
            'trace scope')
    require(isinstance(frame['case'], str) and 0 < len(frame['case']) <= 256 and
            isinstance(frame['thread'], str) and 0 < len(frame['thread']) <= 128, 'thread/case identity')
    rows = frame['records']
    require(type(frame['started']) is int and 1 <= frame['started'] <= 4096 and len(rows) == frame['started'], 'record count')
    for row in rows:
        for key in ('id', 'parent', 'start_ns', 'end_ns', 'elapsed_ns'):
            require(type(row[key]) is int and row[key] >= 0, 'integer field')
        require(row['id'] > row['parent'] and row['end_ns'] >= row['start_ns'] and
                row['elapsed_ns'] == row['end_ns'] - row['start_ns'], 'record time/identity')
        require(row['operation'] in OPERATIONS and row['returned'] is True and row['unwinding'] is False,
                'incomplete or unwinding helper')
        for key in ('output_bytes', 'stderr_bytes'):
            require(type(row[key]) is int and row[key] >= 0, 'byte count')
        args = row['arguments']
        require(isinstance(args, list) and len(args) <= 64 and all(isinstance(arg, str) for arg in args)
                and sum(len(arg.encode()) for arg in args) <= 4096, 'argument budget')
        if row['operation'] == 'git':
            require(row['parent'] == 0 and args and args[0] in COMMANDS, 'git root/command')
        elif row['operation'] == 'read':
            require(row['parent'] == 0 and len(args) == 1, 'read root')
        else:
            require(row['parent'] > 0 and not args, 'phase parent/arguments')
    by_id = {row['id']: row for row in rows}
    require(len(by_id) == len(rows) and sorted(by_id) == list(range(1, len(rows) + 1)), 'missing or duplicate ID')
    children = {}
    for row in sorted(rows, key=lambda value: value['id']):
        children.setdefault(row['parent'], []).append(row)
        if row['parent']:
            parent = by_id[row['parent']]
            require(parent['operation'] == 'git' and parent['start_ns'] <= row['start_ns'] <= row['end_ns'] <= parent['end_ns'],
                    'phase outside original parent')
    expected = []
    for root in children.get(0, []):
        nested = children.get(root['id'], [])
        require([row['operation'] for row in nested] == (['output', 'decode'] if root['operation'] == 'git' else []),
                'original helper phase graph')
        if nested:
            require(root['output_bytes'] == nested[1]['output_bytes'] and
                    root['stderr_bytes'] == nested[0]['stderr_bytes'] == nested[1]['stderr_bytes'], 'parent/phase byte join')
        previous = root['start_ns']
        for row in nested:
            require(row['start_ns'] >= previous, 'overlapping sequential phases')
            previous = row['end_ns']
            expected.append(row['id'])
        expected.append(root['id'])
    require([row['id'] for row in rows] == expected, 'completion order')
    roots = children.get(0, [])
    require(all(left['end_ns'] <= right['start_ns'] for left, right in zip(roots, roots[1:])), 'overlapping thread roots')
    return frame


def traces(raw, allowed_cases, *, required_cases=None):
    require = review.require
    require(len(raw) <= 16 * 1024**2, 'diagnostic log budget')
    frames = [trace_frame(review.decode_json(line[len('HC074_HELPER_TRACE '):]))
              for line in raw.decode().splitlines() if line.startswith('HC074_HELPER_TRACE ')]
    require(bool(frames), 'missing helper traces')
    cases, threads = set(), set()
    groups = {}
    for frame in frames:
        require(frame['case'] in allowed_cases and frame['case'] not in cases and frame['thread'] not in threads,
                'unknown/duplicate case or thread')
        cases.add(frame['case'])
        threads.add(frame['thread'])
        for row in frame['records']:
            key = frame['case'] + '/' + row['operation']
            item = groups.setdefault(key, dict(calls=0, cumulative_wall_ns=0, output_bytes=0, stderr_bytes=0))
            item['calls'] += 1
            for key_to, key_from in (('cumulative_wall_ns', 'elapsed_ns'), ('output_bytes', 'output_bytes'), ('stderr_bytes', 'stderr_bytes')):
                item[key_to] += row[key_from]
    if required_cases is not None:
        require(cases == set(required_cases), 'whole case trace missing or unexpected')
    counts = {op: sum(row['operation'] == op for frame in frames for row in frame['records']) for op in OPERATIONS}
    shows = sum(row['operation'] == 'git' and row['arguments'][0] == 'show' for frame in frames for row in frame['records'])
    return dict(frames=frames, groups=groups, counts=counts, shows=shows,
        nested_intervals=True, cumulative_wall_not_target_cpu=True,
        observer_overhead_subtracted=False, package_pass=False, product_candidate=False)
