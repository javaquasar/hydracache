"""Retain local sandbox observer checks; never certify original enrollment."""
import argparse
import json
import re
import shutil
import subprocess
import check as c
PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/bootstrap-sandbox-owner-20261023'
QUALIFICATION = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
def need(ok, reason):
    if not ok: raise ValueError(reason)
def read(path): return json.loads(path.read_bytes())
def logs(folder, steps):
    for name in steps:
        row = read(folder / (name + '.json'))
        for suffix in ('stdout', 'stderr'):
            need(c.sha(folder / (name + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'Log drift')
def verify(folder, lane, sources):
    expected = c.commands(lane)
    result = read(folder / 'result.json')
    need(result == dict(lane=lane, failure=None, steps=[n for n, _ in expected], original_attempt=False, server=False, product=False), 'Failed/incomplete lane')
    need(read(folder / 'source-before.json') == read(folder / 'source-after.json') == sources, 'Source drift')
    for name, digest in sources.items(): need(c.sha(folder / 'source' / name) == digest, 'Snapshot drift')
    logs(folder, result['steps'])
    for name, argv in expected:
        row = read(folder / (name + '.json'))
        need(row['exit'] == 0 and row['argv'] == argv, 'Command drift')
    kind = lane.split('-', 1)[0]
    if kind == 'host':
        counts = re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;', (folder / 'canaries.stdout.log').read_text())
        need(sum(map(int, counts)) == 66, 'Contract count drift')
    if kind in ('194', '188'):
        need('9 passed; 0 failed; 0 ignored;' in (folder / 'sandbox.stdout.log').read_text(), 'Sandbox control count drift')
    if kind == 'metadata':
        need((folder / 'registry.stdout.log').read_text().strip() == 'gated-test-check: OK (188 discovered gates, all registered)', 'Registry drift')
def run(commit, lanes):
    need(not PACKET.exists(), 'Never overwrite/reseal')
    need(len(lanes) == 4 and {n.split('-', 1)[0] for n in lanes} == {'host', '194', '188', 'metadata'}, 'Wrong final lane set')
    sources = {n: c.sha(c.ROOT / n) for n in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == QUALIFICATION, 'Qualification drift')
    for lane in lanes: verify(c.OUTPUT / lane, lane, sources)
    initial = c.OUTPUT / 'baseline'
    need(read(initial / 'missing.json')['exit'] == 101 and read(initial / 'missing.json')['argv'] == c.commands('baseline')[0][1], 'First exit/command drift')
    need('original owner sandbox observation binding is absent' in (initial / 'missing.stdout.log').read_text(), 'Missing baseline refusal')
    for folder in c.OUTPUT.iterdir():
        need(folder.is_dir() and folder.name in c.LANES, 'Unknown output')
        for name, digest in read(folder / 'source-before.json').items(): need(c.sha(folder / 'source' / name) == digest, 'Prior snapshot drift')
        logs(folder, read(folder / 'result.json')['steps'])
    for name in c.FILES:
        blob = subprocess.run(['git', '--no-replace-objects', 'show', commit + ':' + name], cwd=c.ROOT, capture_output=True, check=True).stdout
        need(blob.replace(b'\r\n', b'\n') == (c.ROOT / name).read_bytes().replace(b'\r\n', b'\n'), 'Commit drift')
    PACKET.mkdir(parents=True)
    shutil.copytree(c.OUTPUT, PACKET / 'checks')
    c.publish(PACKET / 'audit.json', dict(checked_commit=commit, qualification_sha256=QUALIFICATION,
        owner_derived_constructor_compiled=True, fixed_channel_queries_compiled=True,
        kernel_reader_compiled=True, deterministic_controls=True, owned_helper_refusal=True,
        positive_original_manager_sandbox=False, authenticated_child_exchange=False,
        complete_loaded_unit_settings=False, original_delivery=False, writer_cessation=False,
        completion=False, admission_allowed=False, original_attempt=False, clock_changes=False,
        service_changes=False, server=False, product=False, performance_measurement=False))
    rows = [dict(path=p.relative_to(PACKET).as_posix(), bytes=p.stat().st_size, sha256=c.sha(p)) for p in sorted(PACKET.rglob('*')) if p.is_file()]
    c.publish(PACKET / 'inventory.json', dict(files=rows))
    print(json.dumps(dict(members=len(rows), inventory_sha256=c.sha(PACKET / 'inventory.json')), sort_keys=True))
if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checked-commit', required=True)
    parser.add_argument('--lanes', nargs=4, choices=c.LANES, default=['host', '194', '188', 'metadata'])
    args = parser.parse_args()
    run(args.checked_commit, args.lanes)
