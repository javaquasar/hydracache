"""Offline immutable allocation packet verification; never executes a measured image."""
import argparse
import json
from pathlib import Path
import review
import run as method


def inventory(folder):
    values = []
    for path in sorted(folder.rglob('*')):
        method.need(not path.is_symlink(), 'Packet symlink refused')
        if path.is_file() and path != folder / 'inventory.json':
            values.append(dict(path=path.relative_to(folder).as_posix(), bytes=path.stat().st_size, sha256=method.sha(path)))
    return values


def verify(folder):
    index = json.loads((folder / 'inventory.json').read_text())['files']
    for row in index:
        method.member(row['path'])
    method.need(index == inventory(folder), 'Allocation packet membership or bytes changed')
    value = review.facts(folder / 'measured')
    method.need(value == json.loads((folder / 'facts.json').read_text()), 'Allocation facts changed')
    predecessor = method.ROOT / 'docs/testing/performance/0.74/local-runs/authority-runtime-20261022'
    method.need(method.sha(predecessor / 'inventory.json') == 'fafe2b63fde717f1f34d3ad62b06a50014c9653916a8a8b79577fd9f91568375', 'Previous runtime seal changed')
    previous = json.loads((predecessor / 'inventory.json').read_text())['files']
    method.need(len(previous) == 364, 'Previous runtime member count changed')
    for row in previous:
        method.member(row['path'])
        path = predecessor / row['path']
        method.need(path.stat().st_size == row['bytes'] and method.sha(path) == row['sha256'], 'Previous runtime member changed')
    return value


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    parser.add_argument('--seal', action='store_true')
    args = parser.parse_args()
    if args.seal:
        method.publish(args.folder / 'facts.json', review.facts(args.folder / 'measured'))
        method.publish(args.folder / 'inventory.json', dict(schema='authority-runtime-allocation-inventory-074-v1', files=inventory(args.folder)))
    value = verify(args.folder)
    print(json.dumps(dict(method=value['method'], pairs_per_compiler=value['pairs_per_compiler'],
        cells_per_compiler=len(value['lanes']['194']), predecessor_verified=364, performance_release_claim=False), sort_keys=True))
