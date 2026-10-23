"""Retain checked assertion schema controls exclusively, never admit execution."""
import argparse
import json
import re
import shutil
import subprocess
import check as c

PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/bootstrap-installed-entry-20261023'
QUALIFICATION = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'


def need(value, message):
    if not value: raise ValueError(message)


def read(path): return json.loads(path.read_bytes())


def check_logs(folder, steps):
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
    check_logs(folder, result['steps'])
    for name, argv in expected:
        row = read(folder / (name + '.json'))
        need(row['exit'] == 0 and row['argv'] == argv, 'Command drift')
    if lane == 'host':
        need('7 passed; 0 failed; 0 ignored;' in (folder / 'codec.stdout.log').read_text(), 'Codec count drift')
        counts = re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;', (folder / 'canaries.stdout.log').read_text())
        need(sum(map(int, counts)) == 57, 'Source contract count drift')
    if lane in ('194', '188'):
        need('16 passed; 0 failed; 0 ignored;' in (folder / 'installed.stdout.log').read_text(), 'Installed count drift')
    if lane == 'metadata':
        need((folder / 'registry.stdout.log').read_text().strip() == 'gated-test-check: OK (188 discovered gates, all registered)', 'Registry drift')


def run(commit):
    need(not PACKET.exists(), 'Never overwrite/reseal')
    sources = {name: c.sha(c.ROOT / name) for name in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == QUALIFICATION, 'Qualification drift')
    for lane in ('host', '194', '188', 'metadata'): verify(c.OUTPUT / lane, lane, sources)
    need({p.name for p in c.OUTPUT.iterdir()} == set(c.LANES), 'Unexpected output')
    initial = c.OUTPUT / 'baseline'
    result = read(initial / 'result.json')
    need(result['lane'] == 'baseline' and result['steps'] == ['missing'] and result['failure'] == 'RuntimeError: missing refused; first result retained', 'Missing first refusal')
    need(read(initial / 'missing.json')['exit'] == 101, 'Baseline exit drift')
    need('separate installed entry assertion codec is absent' in (initial / 'missing.stdout.log').read_text(), 'Baseline reason drift')
    check_logs(initial, ['missing'])
    for name, digest in read(initial / 'source-before.json').items():
        need(c.sha(initial / 'source' / name) == digest, 'Baseline snapshot drift')
    for name in c.FILES:
        blob = subprocess.run(['git', '--no-replace-objects', 'show', commit + ':' + name], cwd=c.ROOT, capture_output=True, check=True).stdout
        need(blob.replace(b'\r\n', b'\n') == (c.ROOT / name).read_bytes().replace(b'\r\n', b'\n'), 'Commit drift')
    PACKET.mkdir(parents=True)
    shutil.copytree(c.OUTPUT, PACKET / 'checks')
    c.publish(PACKET / 'audit.json', dict(checked_commit=commit, qualification_sha256=QUALIFICATION,
        canonical_codec=True, assertion_bits_mutated=10240, parent_counts_asserted_only=[32, 16],
        original_bytes_preserved=1024, record_bytes=1280, record_materialized=False,
        original_owner_bound=False, child_inventory_remapped=False, child_authentication=False,
        original_delivery=False, writer_cessation=False, admission_allowed=False,
        original_attempt=False, clock_changes=False, service_changes=False, server=False, product=False,
        performance_measurement=False))
    rows = [dict(path=p.relative_to(PACKET).as_posix(), bytes=p.stat().st_size, sha256=c.sha(p)) for p in sorted(PACKET.rglob('*')) if p.is_file()]
    c.publish(PACKET / 'inventory.json', dict(files=rows))
    print(json.dumps(dict(members=len(rows), inventory_sha256=c.sha(PACKET / 'inventory.json')), sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checked-commit', required=True)
    run(parser.parse_args().checked_commit)
