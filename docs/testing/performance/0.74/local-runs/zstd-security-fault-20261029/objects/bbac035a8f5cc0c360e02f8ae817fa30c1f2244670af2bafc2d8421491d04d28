"""Offline, exclusive evidence retention for the closed component census."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import check as c

PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/held-complete-role-census-20261022'
QUALIFICATION = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'


def need(value, message):
    if not value: raise ValueError(message)


def read(file): return json.loads(file.read_bytes())


def verify_lane(root, lane, sources):
    folder = root / lane
    expected = c.commands(lane)
    result = read(folder / 'result.json')
    need(result['failure'] is None and result['lane'] == lane and
         result['steps'] == [name for name, _ in expected], 'Incomplete or failed lane')
    need(not any(result[key] for key in ('original_attempt', 'product', 'server')), 'Wrong scope')
    need(read(folder / 'source-before.json') == read(folder / 'source-after.json') == sources, 'Source drift')
    for name, argv in expected:
        row = read(folder / (name + '.json'))
        need(row['exit'] == 0 and row['argv'] == argv, 'Failed or different command')
        for suffix in ('stdout', 'stderr'):
            need(c.sha(folder / (name + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'Changed log')


def run(runtime_commit):
    need(not PACKET.exists(), 'Never overwrite or reseal evidence')
    sources = {name: c.sha(c.ROOT / name) for name in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == QUALIFICATION, 'Qualification changed')
    for name, digest in sources.items():
        raw = subprocess.run(['git', '--no-replace-objects', 'show', runtime_commit + ':' + name],
                             cwd=c.ROOT, capture_output=True, check=True).stdout
        # Git text normalization is accounted for only for source, not raw evidence.
        actual = (c.ROOT / name).read_bytes()
        need(raw.replace(b'\r\n', b'\n') == actual.replace(b'\r\n', b'\n'), 'Runtime commit differs')
    for lane in ('host', '194', '188'): verify_lane(c.OUTPUT, lane, sources)
    need(read(c.OUTPUT / 'red.json')['exit'] == 101, 'First red baseline missing')
    for lane in ('controls', 'controls-v2', 'docs-v2'): verify_lane(c.OUTPUT, lane, sources)
    first = c.OUTPUT / 'docs'
    failure = read(first / 'result.json')
    need(failure['failure'] is not None and failure['steps'] == ['docs', 'registry'], 'Missing first tooling refusal')
    need(read(first / 'source-before.json') == sources, 'First tooling source drift')
    for name, argv in c.commands('docs')[:2]:
        row = read(first / (name + '.json'))
        need(row['argv'] == argv and row['exit'] == (0 if name == 'docs' else 1), 'First tooling result changed')
        for suffix in ('stdout', 'stderr'):
            need(c.sha(first / (name + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'First tooling log changed')
    for lane in ('194', '188'):
        for name, count in (('census', 9), ('material', 17), ('projection', 10)):
            need(f'{count} passed; 0 failed; 0 ignored;' in
                 (c.OUTPUT / lane / (name + '.stdout.log')).read_text(), 'Missing component results')
    for name in ('docs', 'registry', 'contract', 'diff'):
        need(read(c.OUTPUT / 'docs-v2' / (name + '.json'))['exit'] == 0, 'Documentation contract failed')
    PACKET.mkdir(parents=True)
    for file in c.OUTPUT.rglob('*'):
        if file.is_file():
            dest = PACKET / 'checks' / file.relative_to(c.OUTPUT)
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(file, dest)
    for name in c.FILES + ['docs/testing/performance/0.74/held-complete-role-census-plan-20261022.md',
                          'docs/testing/performance/0.74/held-complete-role-census-development-20261022.md']:
        dest = PACKET / 'source' / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(c.ROOT / name, dest)
    for file in Path(__file__).parent.iterdir():
        if file.is_file() and file.suffix in ('.py', '.md'):
            dest = PACKET / 'runner' / file.name
            dest.parent.mkdir(exist_ok=True)
            shutil.copyfile(file, dest)
    c.publish(PACKET / 'audit.json', dict(runtime_commit=runtime_commit, qualification_sha256=QUALIFICATION,
        inherited_roles=11, enumerated_fds=23, component_pass=True, admission_allowed=False,
        original_delivery=False, writers_cessation=False, original_attempt=False, product=False, server=False,
        unchanged_original_deadlines=True, first_component_failure='development summary, not raw process log'))
    files = [dict(path=file.relative_to(PACKET).as_posix(), bytes=file.stat().st_size, sha256=c.sha(file))
             for file in sorted(PACKET.rglob('*')) if file.is_file()]
    c.publish(PACKET / 'inventory.json', dict(files=files))
    print(json.dumps(dict(members=len(files), inventory_sha256=c.sha(PACKET / 'inventory.json')), sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime-commit', required=True)
    run(parser.parse_args().runtime_commit)
