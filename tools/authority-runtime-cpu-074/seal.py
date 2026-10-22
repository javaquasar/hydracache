"""Immutable CPU packet verifier. Does not execute a measurement image."""
import argparse
import json
from pathlib import Path
import review
import run as method


def inventory(folder):
    values = []
    for path in sorted(folder.rglob('*')):
        method.common.need(not path.is_symlink(), 'CPU packet symlink refused')
        if path.is_file() and path != folder / 'inventory.json':
            values.append(dict(path=path.relative_to(folder).as_posix(), bytes=path.stat().st_size, sha256=method.common.sha(path)))
    return values


def verify_predecessor():
    packet = method.ROOT / 'docs/testing/performance/0.74/local-runs/authority-runtime-allocation-20261022'
    method.common.need(method.common.sha(packet / 'inventory.json') ==
        '496e3628c1975e5e006c1b8a12ecef8211314c3367cff1d0d50b40c7846f9913', 'Allocation predecessor seal changed')
    rows = json.loads((packet / 'inventory.json').read_text())['files']
    method.common.need(len(rows) == 429, 'Allocation predecessor membership changed')
    for row in rows:
        method.common.member(row['path']); path = packet / row['path']
        method.common.need(path.stat().st_size == row['bytes'] and method.common.sha(path) == row['sha256'], 'Allocation predecessor bytes changed')


def verify(folder):
    rows = json.loads((folder / 'inventory.json').read_text())['files']
    for row in rows: method.common.member(row['path'])
    method.common.need(rows == inventory(folder), 'CPU packet membership/bytes changed')
    value = review.facts(folder / 'measured')
    method.common.need(value == json.loads((folder / 'facts.json').read_text()), 'CPU facts changed')
    verify_predecessor()
    return value


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path); parser.add_argument('--seal', action='store_true')
    args = parser.parse_args()
    if args.seal:
        method.common.publish(args.folder / 'facts.json', review.facts(args.folder / 'measured'))
        method.common.publish(args.folder / 'inventory.json', dict(schema='authority-runtime-cpu-inventory-074-v1', files=inventory(args.folder)))
    value = verify(args.folder)
    print(json.dumps(dict(method=value['method'], pairs_per_compiler=7, cells_per_process=12,
        predecessor_verified=429, warm_cpu=value['primary_warm_cpu']['disposition'],
        cycle_elapsed=value['primary_cycle_elapsed']['disposition'], server_admitted=False), sort_keys=True))
