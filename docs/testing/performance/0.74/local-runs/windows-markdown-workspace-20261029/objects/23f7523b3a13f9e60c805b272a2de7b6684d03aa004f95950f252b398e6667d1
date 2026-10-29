"""Offline verification of the installed-runtime correctness packet, never execution."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re


def need(value, message):
    if not value:
        raise ValueError(message)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def inventory(root):
    rows = []
    for path in sorted(root.rglob('*')):
        need(not path.is_symlink(), 'Packet symlink refused')
        if path.is_file() and path != root / 'inventory.json':
            rows.append(dict(path=path.relative_to(root).as_posix(), bytes=path.stat().st_size, sha256=sha(path)))
    return rows


def member(row):
    path = row['path']
    need(isinstance(path, str) and '\\' not in path and ':' not in path, 'Unsafe inventory path')
    parts = PurePosixPath(path).parts
    need(bool(parts) and not path.startswith('/') and all(p not in ('', '.', '..') for p in path.split('/')), 'Unsafe inventory traversal')
    need(isinstance(row['bytes'], int) and row['bytes'] >= 0 and re.fullmatch('[0-9a-f]{64}', row['sha256']), 'Invalid member identity')


def verify_inventory(root, rows):
    for row in rows:
        member(row)
    need(len({row['path'] for row in rows}) == len(rows), 'Duplicate inventory member')
    need(rows == inventory(root), 'Packet membership or raw bytes changed')


def native(packet, selected, source, lane):
    receipt = json.loads((packet / 'result.json').read_text(encoding='utf-8-sig'))
    need(receipt['schema'] == 'authority-runtime-local-linux-074-v1', 'Wrong native receipt')
    need(receipt['phase'] == 'final' and receipt['lane'] == lane and receipt['failure'] is None, 'Native lane refused')
    need(('rustc ' + {'194': '1.94.0', '188': '1.88.0'}[lane] + ' ') in (packet / 'compiler.stdout.log').read_text(), 'Wrong native compiler')
    before = receipt['before']
    need(before == receipt['after'] and not before['tracked_dirty'] and before['source'] == source, 'Native method or metadata drift')
    need(before['uid'] > 0 and before['gid'] > 0 and before['qualification'] == '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'Native identity drift')
    need(len(receipt['steps']) == 38, 'Wrong native step count')
    need([row['name'] for row in receipt['steps']] == ['compiler', 'check', 'clippy', 'build', 'list'] + [f'test-{i:03}' for i in range(33)], 'Native step order changed')
    for row in receipt['steps']:
        need(row['exit'] == 0 and not row['timeout'], 'Native step refused')
        for suffix in ('stdout', 'stderr'):
            need(sha(packet / (row['name'] + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'Native raw log changed')
    for name, row in zip(selected, receipt['steps'][5:]):
        need(row['argv'] == [receipt['image'], name, '--exact', '--nocapture', '--test-threads=1'], 'Native selection or flags drift')
        output = (packet / (row['name'] + '.stdout.log')).read_text()
        need('test ' + name + ' ... ok' in output and '1 passed; 0 failed; 0 ignored' in output, 'Native test not passed')
    need(not receipt['server'] and not receipt['product'] and receipt['live_profiles'] == 0 and not receipt['ignored_measurements'] and not receipt['clocks_changed'] and not receipt['services_changed'], 'Native scope drift')
    return dict(tests=33, new_tests=7, steps=38, source=source, image_sha256=receipt['image_sha256'], boot=before['boot'], kernel=before['kernel'])


def facts(root):
    context = json.loads((root / 'context.json').read_text(encoding='utf-8-sig'))
    source = context['source']
    selected = json.loads((root / 'source/tools/authority-runtime-074/tests.json').read_text(encoding='utf-8-sig'))['tests']
    need(len(selected) == 33 and len(set(selected)) == 33, 'Wrong frozen native selection')
    lanes = {lane: native(root / ('linux-' + lane), selected, source, lane) for lane in ('194', '188')}
    windows = json.loads((root / 'windows/result.json').read_text(encoding='utf-8-sig'))
    need(windows['source'] == source and windows['failure'] is None and len(windows['steps']) == 19, 'Windows cohort refused')
    for row in windows['steps']:
        need(row['exit'] == 0 and sha(root / 'windows' / row['log']) == row['sha256'], 'Windows log or result drift')
    full = (root / 'windows/current-performance-contracts.log').read_text(encoding='utf-8-sig')
    need('321 passed; 0 failed; 0 ignored' in full, 'Full contract cohort refused')
    source_log = (root / 'windows/source-controls.log').read_text(encoding='utf-8-sig')
    source_counts = [int(n) for n in re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored', source_log)]
    need(source_counts == [5, 10, 8], 'Source gate membership changed')
    canary_log = (root / 'windows/canaries.log').read_text(encoding='utf-8-sig')
    canary_counts = [int(n) for n in re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored', canary_log)]
    need(len(canary_counts) == 30 and sum(canary_counts) == 79, 'Historical canary membership changed')
    python_counts = []
    for name, expected in [('selector-controls', 4), ('packet-controls', 4), ('history-controls', 12), ('history-receipt-controls', 8)]:
        output = (root / 'windows' / (name + '.log')).read_text(encoding='utf-8-sig')
        count = re.findall(r'Ran (\d+) tests? in ', output)
        need(count == [str(expected)] and re.search(r'(?m)^OK\s*$', output), 'Offline Python controls refused')
        python_counts.append(expected)
    predecessor = root / 'predecessor-inventory.json'
    need(sha(predecessor) == '4904ede1ef28f2e4d96b1da0f5be8b633c20a3d2fbb6524289567b974581b63f', 'Predecessor seal changed')
    return dict(schema='authority-runtime-correctness-facts-074-v1', source=source, linux=lanes, windows_steps=19,
        runtime_source_controls=source_counts[0], prior_source_controls=sum(source_counts[1:]), historical_canaries=sum(canary_counts), current_contracts=321,
        selector_receipt_controls=sum(python_counts[:2]), history_receipt_controls=sum(python_counts[2:]), performance_claim=False, full_sigkill=False, server_admitted=False)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('packet', type=Path)
    parser.add_argument('--seal', action='store_true')
    args = parser.parse_args()
    result = facts(args.packet)
    if args.seal:
        for name in ('facts.json', 'inventory.json'):
            value = result if name == 'facts.json' else dict(schema='authority-runtime-inventory-074-v1', files=inventory(args.packet))
            with (args.packet / name).open('x', encoding='utf-8', newline='\n') as stream:
                json.dump(value, stream, indent=2, sort_keys=True)
                stream.write('\n')
    need(json.loads((args.packet / 'facts.json').read_text()) == result, 'Derived facts changed')
    verify_inventory(args.packet, json.loads((args.packet / 'inventory.json').read_text())['files'])
    print(json.dumps(result, sort_keys=True))
