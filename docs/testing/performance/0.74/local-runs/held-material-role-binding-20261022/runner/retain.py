"""Offline verification and exclusive retention of the focused check receipts."""
import json
from pathlib import Path
import shutil
import subprocess
import check as c

PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/held-material-role-binding-20261022'
QUALIFICATION = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'


def need(value, reason):
    if not value: raise ValueError(reason)


def read(file): return json.loads(file.read_bytes())


def verify_lane(root, lane, sources):
    folder = root / lane
    result = read(folder / 'result.json')
    expected = ['canaries', 'check', 'clippy', 'format'] if lane == 'host' else ['material', 'projection', 'check', 'clippy']
    need(result['failure'] is None and result['steps'] == expected, 'Incomplete or failed lane')
    need(not any(result[key] for key in ('original_attempt', 'product', 'server')), 'Wrong scope')
    need(read(folder / 'source-before.json') == read(folder / 'source-after.json') == sources, 'Source drift')
    for name in expected:
        row = read(folder / (name + '.json'))
        need(row['exit'] == 0, 'Failed command')
        for suffix in ('stdout', 'stderr'):
            need(c.sha(folder / (name + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'Changed log')


def run():
    need(not PACKET.exists(), 'Never overwrite or reseal evidence')
    sources = {name: c.sha(c.ROOT / name) for name in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == QUALIFICATION, 'Qualification changed')
    for lane in ('host', '194', '188'): verify_lane(c.OUTPUT, lane, sources)
    need(read(c.OUTPUT / 'red.json')['exit'] == 101, 'First red baseline missing')
    need(read(c.OUTPUT / 'runner-tests.json')['exit'] == 0, 'Runner controls failed')
    need(read(c.OUTPUT / 'retainer-tests.json')['exit'] == 0, 'Retention controls failed')
    for lane in ('194', '188'):
        need('17 passed; 0 failed; 0 ignored;' in (c.OUTPUT / lane / 'material.stdout.log').read_text(), 'Missing component results')
        need('10 passed; 0 failed; 0 ignored;' in (c.OUTPUT / lane / 'projection.stdout.log').read_text(), 'Missing projection results')
    for name in ('docs', 'registry', 'contract', 'diff'):
        need(read(c.OUTPUT / 'docs' / (name + '.json'))['exit'] == 0, 'Documentation contract failed')
    PACKET.mkdir(parents=True)
    for file in c.OUTPUT.rglob('*'):
        if file.is_file():
            dest = PACKET / 'checks' / file.relative_to(c.OUTPUT)
            dest.parent.mkdir(parents=True, exist_ok=True); shutil.copyfile(file, dest)
    for name in c.FILES + ['docs/testing/performance/0.74/held-material-role-binding-plan-20261022.md']:
        dest = PACKET / 'source' / name; dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(c.ROOT / name, dest)
    for file in Path(__file__).parent.iterdir():
        if file.is_file():
            dest = PACKET / 'runner' / file.name; dest.parent.mkdir(exist_ok=True); shutil.copyfile(file, dest)
    c.publish(PACKET / 'audit.json', dict(runtime_commit='4e1d38cc55f59e89e1f0191a2950bc96162f8121',
        qualification_sha256=QUALIFICATION, component_pass=True, inherited_roles_resolved=8, inherited_roles_total=11,
        original_delivery=False, writers_cessation=False, admission_allowed=False, original_attempt=False,
        product=False, server=False, no_original_deadline_changes=True))
    files = [dict(path=file.relative_to(PACKET).as_posix(), bytes=file.stat().st_size, sha256=c.sha(file))
             for file in sorted(PACKET.rglob('*')) if file.is_file()]
    c.publish(PACKET / 'inventory.json', dict(files=files))
    print(json.dumps(dict(members=len(files), inventory_sha256=c.sha(PACKET / 'inventory.json')), sort_keys=True))


if __name__ == '__main__': run()
