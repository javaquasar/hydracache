"""Exclusively retain local installed-origin checks, not deployment or child admission."""
import argparse
import json
import shutil
import subprocess
import check as c

PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/bootstrap-installed-origin-20261023'
QUALIFICATION = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
def need(ok, message):
    if not ok: raise ValueError(message)
def read(path): return json.loads(path.read_bytes())


def verify(folder, lane, sources):
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
    kind = lane.removesuffix('-final')
    counts = [('child-units', 7), ('child-cli', 1), ('entry', 2), ('build', 5)] if kind == 'host' else []
    if kind in ('194', '188'):
        counts = [('installed', 9), ('child-units', 19), ('child-cli', 2), ('census-cli', 3), ('image-cli', 3),
                  ('entry', 7), ('images', 9), ('build', 6), ('selection', 4), ('owner', 3), ('enrollment', 5),
                  ('old-source', 1), ('old-replacement', 1), ('old-keys', 1)]
    for name, count in counts:
        need(f'{count} passed; 0 failed; 0 ignored;' in (folder / (name + '.stdout.log')).read_text(), 'Count drift')
    if kind == 'metadata':
        need((folder / 'registry.stdout.log').read_text().strip() == 'gated-test-check: OK (188 discovered gates, all registered)', 'Registry drift')


def run(commit, lanes):
    need(not PACKET.exists(), 'Never overwrite/reseal')
    need(len(lanes) == 4 and {n.removesuffix('-final') for n in lanes} == {'host', '194', '188', 'metadata'}, 'Wrong lane set')
    sources = {n: c.sha(c.ROOT / n) for n in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == QUALIFICATION, 'Qualification drift')
    for lane in lanes: verify(c.OUTPUT / lane, lane, sources)
    for folder in c.OUTPUT.iterdir():
        need(folder.is_dir() and folder.name in c.LANES, 'Unknown prior output')
        for name, digest in read(folder / 'source-before.json').items():
            need(c.sha(folder / 'source' / name) == digest, 'Prior snapshot drift')
        for step in read(folder / 'result.json')['steps']:
            row = read(folder / (step + '.json'))
            for suffix in ('stdout', 'stderr'):
                need(c.sha(folder / (step + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'Prior log drift')
    for name in c.FILES:
        blob = subprocess.run(['git', '--no-replace-objects', 'show', commit + ':' + name], cwd=c.ROOT, capture_output=True, check=True).stdout
        need(blob.replace(b'\r\n', b'\n') == (c.ROOT / name).read_bytes().replace(b'\r\n', b'\n'), 'Commit drift')
    PACKET.mkdir(parents=True)
    shutil.copytree(c.OUTPUT, PACKET / 'checks')
    c.publish(PACKET / 'audit.json', dict(checked_commit=commit, qualification_sha256=QUALIFICATION,
        original_owner_wiring_compiled=True, installed_source_private_filesystem_controls=True, owner_fault_models=True,
        parent_descriptors=32, parent_roles=16, production_fixed_path_installed=False, original_live_join=False,
        child_wire_v2=False, child_namespace_enrollment=False, credentials_manager_guardians_authenticated=False,
        original_delivery=False, writer_cessation=False, admission_allowed=False, performance_measurement=False,
        clock_changes=False, service_changes=False, server=False, product=False))
    rows = [dict(path=f.relative_to(PACKET).as_posix(), bytes=f.stat().st_size, sha256=c.sha(f)) for f in sorted(PACKET.rglob('*')) if f.is_file()]
    c.publish(PACKET / 'inventory.json', dict(files=rows))
    print(json.dumps(dict(members=len(rows), inventory_sha256=c.sha(PACKET / 'inventory.json')), sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checked-commit', required=True)
    parser.add_argument('--lanes', nargs=4, choices=c.LANES, default=['host', '194', '188', 'metadata'])
    args = parser.parse_args()
    run(args.checked_commit, args.lanes)
