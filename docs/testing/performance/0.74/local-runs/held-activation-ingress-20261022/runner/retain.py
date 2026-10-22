"""Exclusive offline retention for activation ingress, not launch qualification."""
import argparse
import json
import shutil
from pathlib import Path
import check as c

PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/held-activation-ingress-20261022'
QUALIFICATION = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'


def need(value, reason):
    if not value: raise ValueError(reason)


def read(path): return json.loads(path.read_bytes())


def verify(folder, lane, sources):
    result = read(folder / 'result.json')
    expected = c.commands(lane)
    need(result['failure'] is None and result['lane'] == lane and
         result['steps'] == [name for name, _ in expected], 'Failed/incomplete lane')
    need(not any(result[key] for key in ('server', 'product', 'original_attempt')), 'Wrong scope')
    need(read(folder / 'source-before.json') == read(folder / 'source-after.json') == sources, 'Source drift')
    for name, argv in expected:
        row = read(folder / (name + '.json'))
        need(row['exit'] == 0 and row['argv'] == argv, 'Changed/failed command')
        for suffix in ('stdout', 'stderr'):
            need(c.sha(folder / (name + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'Changed log')


def run(commit):
    need(not PACKET.exists(), 'Never overwrite or reseal')
    sources = {name: c.sha(c.ROOT / name) for name in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == QUALIFICATION, 'Qualification changed')
    for lane in ('host', '194', '188', 'metadata'): verify(c.OUTPUT / lane, lane, sources)
    need(read(c.OUTPUT / 'red.json')['exit'] == 101, 'Missing red baseline')
    for lane in ('194', '188'):
        for name, count in (('binary', 6), ('process', 1)):
            need(f'{count} passed; 0 failed; 0 ignored;' in (c.OUTPUT / lane / (name + '.stdout.log')).read_text(), 'Wrong test selection')
    for name in c.FILES:
        raw = c.logger.subprocess.run(['git', '--no-replace-objects', 'show', commit + ':' + name],
                                     cwd=c.ROOT, capture_output=True, check=True).stdout
        need(raw.replace(b'\r\n', b'\n') == (c.ROOT / name).read_bytes().replace(b'\r\n', b'\n'), 'Commit drift')
    PACKET.mkdir(parents=True)
    for file in c.OUTPUT.rglob('*'):
        if file.is_file():
            dest = PACKET / 'checks' / file.relative_to(c.OUTPUT)
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(file, dest)
    for name in c.FILES + ['docs/testing/performance/0.74/held-activation-ingress-plan-20261022.md']:
        dest = PACKET / 'source' / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(c.ROOT / name, dest)
    for file in Path(__file__).parent.iterdir():
        if file.is_file() and file.suffix in ('.py', '.md'):
            dest = PACKET / 'runner' / file.name
            dest.parent.mkdir(exist_ok=True)
            shutil.copyfile(file, dest)
    c.publish(PACKET / 'audit.json', dict(runtime_commit=commit, qualification_sha256=QUALIFICATION,
        strict_ingress_component_pass=True, executable_addressing=False, original_delivery=False,
        writers_cessation=False, admission_allowed=False, service_changes=False, clock_changes=False,
        original_attempt=False, server=False, product=False, unchanged_original_deadlines=True))
    files = [dict(path=file.relative_to(PACKET).as_posix(), bytes=file.stat().st_size, sha256=c.sha(file))
             for file in sorted(PACKET.rglob('*')) if file.is_file()]
    c.publish(PACKET / 'inventory.json', dict(files=files))
    print(json.dumps(dict(members=len(files), inventory_sha256=c.sha(PACKET / 'inventory.json')), sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime-commit', required=True)
    run(parser.parse_args().runtime_commit)
