"""Offline retention of the first manager probe and both zero-mutation refusals."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path.cwd()
sys.path.insert(0, str(ROOT / 'tools/held-manager-compat-074'))
import probe as p

sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
names = ('first', 'second', 'third')
pins = ('aabcb3601f7b0624eb3257a347ad4308362ca7ed', '88dbabf6a52ca0e42521f7543568b02e3509c70c',
    '13dcce838ecd92102f514e29148291f938cbef27')
counts = (1, 4, 10)


def verify(packet):
    results = []
    start_calls = []
    for name, pin, count in zip(names, pins, counts):
        folder = packet / 'native' / name
        result = json.loads((folder / 'result.json').read_text())
        assert result['before']['source'] == pin and result['before']['qualification'] == p.QUALIFICATION
        assert result['schema'] == 'held-manager-compatibility-074-v' + str(names.index(name) + 1)
        assert all(result[key] is False for key in ('server', 'product', 'clocks_changed', 'existing_services_changed', 'original_delivery', 'writers_cessation', 'admission_allowed'))
        rows = [json.loads((folder / f'{index:03d}-command.json').read_text()) for index in range(count)]
        assert {path.name for path in folder.iterdir()} == {'intent.json', 'result.json'} | {f'{i:03d}-command.json' for i in range(count)}
        for row in rows:
            assert row['failure'] is None and row['argv'][:6] == ['/usr/bin/busctl', '--json=short', '--timeout=250ms', '--auto-start=no', 'call', row['argv'][5]]
            if row['argv'][8] == 'StartTransientUnit': start_calls.append((name, row))
        for path, digest in result['before']['source_sha256'].items():
            assert sha(packet / 'source' / name / path) == digest
        if name == 'first':
            assert result['failure'] == 'ValueError: Unexpected bus reply value'
            assert rows[0]['exit'] == 0 and p.scalar(rows[0], 's') == ':1.2'
        elif name == 'second':
            assert result['failure'] == 'ValueError: Manager version query failed'
            assert rows[3]['stderr'] == 'Call failed: Unknown interface org.freedesktop.systemd1 or property Version.\n'
            assert rows[3]['argv'][-2] == p.MANAGER
        else:
            assert result['failure'] is None and result['before'] == result['after']
            unit = json.loads((folder / 'intent.json').read_text())['unit']
            class RecordedBus:
                index = 0
                def call(self, *args):
                    row = rows[self.index]
                    self.index += 1
                    assert row['argv'][5:] == list(args)
                    return row
            replay = RecordedBus()
            assert p.check(replay, unit) == result['observation'] and replay.index == 10
        results.append(result)
    assert len(start_calls) == 1 and start_calls[0][0] == 'third'
    assert p.refusal(start_calls[0][1])
    for name, total in (('windows', 9), ('linux', 9), ('linux-final', 9), ('windows-v2', 10), ('linux-v2', 10), ('windows-v3', 11), ('linux-v3', 11)):
        folder = packet / 'checks'
        row = json.loads((folder / (name + '.json')).read_text())
        assert row['exit'] == 0
        for suffix in ('stdout', 'stderr'):
            assert sha(folder / (name + '.' + suffix + '.log')) == row[suffix + '_sha256']
        raw = (folder / (name + '.stderr.log')).read_text()
        assert 'Ran ' + str(total) + ' tests in ' in raw and raw.rstrip().endswith('OK')
    baseline = json.loads((packet / 'checks/fixed-interface-baseline.json').read_text())
    assert baseline['exit'] != 0 and 'FAILED (failures=1)' in (packet / 'checks/fixed-interface-baseline.stderr.log').read_text()
    return dict(schema='held-manager-compat-audit-074-v1', source=pins[-1], command_receipts=15,
        start_transient_unit_calls=1, early_zero_mutation_refusals=2, final_controls_per_platform=11,
        disposition='blocked-unsupported-manager-fd-api', version=results[-1]['observation']['version'],
        unit=results[-1]['observation']['unit'], final_context_unchanged=True, unit_absent_before_and_after=True,
        original_delivery=False, writers_cessation=False, admission_allowed=False,
        existing_services_changed=False, clocks_changed=False, server=False, product=False)


packet = ROOT / 'docs/testing/performance/0.74/local-runs/held-manager-compat-20261022'
assert not packet.exists()
assert not subprocess.check_output(['git', 'diff', 'c237f580', 'HEAD', '--name-only', '--',
    'tools/long-run-supervisor-074', 'docs/testing/performance/0.74/qualification-manifest.toml', 'docs/plans/releases.toml'])
assert sha(ROOT / 'docs/testing/performance/0.74/qualification-manifest.toml') == p.QUALIFICATION
packet.mkdir()
for name, pin in zip(names, pins):
    source = ROOT / ('target/held-manager-compat-' + name)
    result = json.loads((source / 'result.json').read_text())
    shutil.copytree(source, packet / 'native' / name)
    for path, digest in result['before']['source_sha256'].items():
        raw = subprocess.check_output(['git', 'show', pin + ':' + path])
        if hashlib.sha256(raw).hexdigest() != digest:
            raw = raw.replace(b'\n', b'\r\n')
        assert hashlib.sha256(raw).hexdigest() == digest
        destination = packet / 'source' / name / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open('xb') as stream: stream.write(raw)
shutil.copytree(ROOT / 'target/held-manager-compat-checks-20261022', packet / 'checks')
shutil.copyfile(__file__, packet / 'checks/retain.py')
audit = verify(packet)
with (packet / 'audit.json').open('x') as stream: json.dump(audit, stream, sort_keys=True, indent=2)
rows = []
for path in sorted(packet.rglob('*'), key=lambda f: f.relative_to(packet).as_posix()):
    assert not path.is_symlink()
    if path.is_file(): rows.append(dict(path=path.relative_to(packet).as_posix(), bytes=path.stat().st_size, sha256=sha(path)))
with (packet / 'inventory.json').open('x') as stream:
    json.dump(dict(schema='held-manager-compat-inventory-074-v1', files=rows), stream, sort_keys=True, indent=2)
print(json.dumps(dict(members=len(rows), inventory_sha256=sha(packet/'inventory.json'), **audit), sort_keys=True))
