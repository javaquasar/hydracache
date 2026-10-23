"""Exclusively retain a static trust-boundary audit, not execution evidence."""
import argparse
import json
import shutil
import subprocess
import check as c

PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/bootstrap-child-boundary-20261023'
QUALIFICATION = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'


def need(ok, message):
    if not ok: raise ValueError(message)


def read(path): return json.loads(path.read_bytes())


def run(commit):
    need(not PACKET.exists(), 'Never overwrite/reseal')
    sources = {n: c.sha(c.ROOT / n) for n in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == QUALIFICATION, 'Qualification drift')
    for lane in c.LANES:
        folder = c.OUTPUT / lane
        result = read(folder / 'result.json')
        expected = c.commands(lane)
        need(result == dict(lane=lane, failure=None, steps=[n for n, _ in expected], original_attempt=False, server=False, product=False), 'Failed/incomplete lane')
        need(read(folder / 'source-before.json') == read(folder / 'source-after.json') == sources, 'Source drift')
        for name, digest in sources.items(): need(c.sha(folder / 'source' / name) == digest, 'Snapshot drift')
        for name, argv in expected:
            row = read(folder / (name + '.json'))
            need(row['exit'] == 0 and row['argv'] == argv, 'Command drift')
            for suffix in ('stdout', 'stderr'):
                need(c.sha(folder / (name + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'Log drift')
        counts = [('audit', 4)] if lane == 'host' else [('projection', 7), ('build', 6)] if lane in ('194', '188') else []
        for name, count in counts:
            need(f'{count} passed; 0 failed; 0 ignored;' in (folder / (name + '.stdout.log')).read_text(), 'Count drift')
        if lane == 'metadata':
            need((folder / 'registry.stdout.log').read_text().strip() == 'gated-test-check: OK (188 discovered gates, all registered)', 'Registry drift')
    for name in c.FILES:
        blob = subprocess.run(['git', '--no-replace-objects', 'show', commit + ':' + name], cwd=c.ROOT, capture_output=True, check=True).stdout
        need(blob.replace(b'\r\n', b'\n') == (c.ROOT / name).read_bytes().replace(b'\r\n', b'\n'), 'Commit drift')
    PACKET.mkdir(parents=True)
    shutil.copytree(c.OUTPUT, PACKET / 'checks')
    c.publish(PACKET / 'audit.json', dict(checked_commit=commit, qualification_sha256=QUALIFICATION,
        runtime_changes=False, source_contracts=True, existing_projection_and_build_models=True,
        child_authentication=False, child_namespace_enrollment=False, production_bootstrap_address=False,
        original_delivery=False, writer_cessation=False, admission_allowed=False, performance_measurement=False,
        clock_changes=False, service_changes=False, server=False, product=False))
    rows = [dict(path=f.relative_to(PACKET).as_posix(), bytes=f.stat().st_size, sha256=c.sha(f)) for f in sorted(PACKET.rglob('*')) if f.is_file()]
    c.publish(PACKET / 'inventory.json', dict(files=rows))
    print(json.dumps(dict(members=len(rows), inventory_sha256=c.sha(PACKET / 'inventory.json')), sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checked-commit', required=True)
    run(parser.parse_args().checked_commit)
