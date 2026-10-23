"""Exclusively retain component evidence, never claim bootstrap execution."""
import argparse
import json
import shutil
from pathlib import Path
import check as c

PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/bootstrap-build-owner-20261023'
QUALIFICATION = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'


def need(value, message):
    if not value: raise ValueError(message)


def read(file): return json.loads(file.read_bytes())


def verify(folder, lane, sources):
    result = read(folder / 'result.json')
    expected = c.commands(lane)
    need(result['failure'] is None and result['lane'] == lane and result['steps'] == [n for n, _ in expected], 'Incomplete/failed lane')
    need(not any(result[k] for k in ('server', 'product', 'original_attempt')), 'Wrong scope')
    need(read(folder / 'source-before.json') == read(folder / 'source-after.json') == sources, 'Source drift')
    for name, argv in expected:
        row = read(folder / (name + '.json'))
        need(row['exit'] == 0 and row['argv'] == argv, 'Wrong/failed command')
        for suffix in ('stdout', 'stderr'):
            need(c.sha(folder / (name + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'Changed output')
    normalized = lane.removesuffix('-v2').removesuffix('-final')
    counts = [('build', 5)] if normalized == 'host' else []
    if normalized in ('194', '188'):
        counts = [('build', 6), ('selection', 4), ('owner', 3), ('enrollment', 5), ('old-source', 1), ('old-replacement', 1), ('old-keys', 1)]
    for name, count in counts:
        need(f'{count} passed; 0 failed; 0 ignored;' in (folder / (name + '.stdout.log')).read_text(), 'Wrong test count')


def run(commit):
    need(not PACKET.exists(), 'Never overwrite/reseal')
    sources = {n: c.sha(c.ROOT / n) for n in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == QUALIFICATION, 'Qualification changed')
    for lane in ('host-final', '194-final', '188-final', 'metadata'): verify(c.OUTPUT / lane, lane, sources)
    for name in c.FILES:
        raw = c.logger.subprocess.run(['git', '--no-replace-objects', 'show', commit + ':' + name], cwd=c.ROOT, capture_output=True, check=True).stdout
        need(raw.replace(b'\r\n', b'\n') == (c.ROOT / name).read_bytes().replace(b'\r\n', b'\n'), 'Commit drift')
    PACKET.mkdir(parents=True)
    for file in c.OUTPUT.rglob('*'):
        if file.is_file():
            dest = PACKET / 'checks' / file.relative_to(c.OUTPUT)
            dest.parent.mkdir(parents=True, exist_ok=True); shutil.copyfile(file, dest)
    for name in c.FILES:
        dest = PACKET / 'source' / name
        dest.parent.mkdir(parents=True, exist_ok=True); shutil.copyfile(c.ROOT / name, dest)
    for file in Path(__file__).parent.iterdir():
        if file.is_file() and file.suffix in ('.py', '.md'):
            dest = PACKET / 'runner' / file.name
            dest.parent.mkdir(exist_ok=True); shutil.copyfile(file, dest)
    c.publish(PACKET / 'audit.json', dict(checked_commit=commit, qualification_sha256=QUALIFICATION,
        signed_policy_receipt=True, protected_source_unit_checks=True, consumed_owner_wiring_compiled=True,
        owner_fault_models=True, original_live_join=False, bootstrap_executed=False, original_delivery=False,
        writers_cessation=False, server_enrollment=False, admission_allowed=False, clock_changes=False,
        service_changes=False, server=False, product=False))
    members = [dict(path=f.relative_to(PACKET).as_posix(), bytes=f.stat().st_size, sha256=c.sha(f)) for f in sorted(PACKET.rglob('*')) if f.is_file()]
    c.publish(PACKET / 'inventory.json', dict(files=members))
    print(json.dumps(dict(members=len(members), inventory_sha256=c.sha(PACKET / 'inventory.json')), sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checked-commit', required=True)
    run(parser.parse_args().checked_commit)
