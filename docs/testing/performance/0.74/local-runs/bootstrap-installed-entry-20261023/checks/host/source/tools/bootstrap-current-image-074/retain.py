"""Verify and retain fixed local current-image controls without rerunning them."""
import argparse
import importlib.util
import json
import shutil
from pathlib import Path
import check as c

spec = importlib.util.spec_from_file_location('build_retainer', c.ROOT / 'tools/bootstrap-build-owner-074/retain.py')
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)
base.c = c
need, read = base.need, base.read
PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/bootstrap-current-image-20261023'


def verify(folder, lane, sources):
    base.verify(folder, lane, sources)
    kind = lane.removesuffix('-final')
    counts = [('child-units', 7), ('child-cli', 1), ('entry', 2)] if kind == 'host' else []
    if kind in ('194', '188'):
        counts = [('child-units', 19), ('child-cli', 2), ('census-cli', 3), ('image-cli', 3), ('entry', 7), ('images', 9)]
    for name, count in counts:
        need(f'{count} passed; 0 failed; 0 ignored;' in (folder / (name + '.stdout.log')).read_text(), 'Wrong selected count')
    if kind == 'metadata':
        need('188' in (folder / 'registry.stdout.log').read_text(), 'New gate missing')
    for name, digest in sources.items():
        need(c.sha(folder / 'source' / name) == digest, 'Snapshot drift')


def run(commit, lanes):
    need(not PACKET.exists(), 'Never overwrite/reseal')
    need(len(lanes) == 4 and {n.removesuffix('-final') for n in lanes} == {'host', '194', '188', 'metadata'}, 'Wrong lane set')
    sources = {n: c.sha(c.ROOT / n) for n in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == base.QUALIFICATION, 'Qualification changed')
    for lane in lanes: verify(c.OUTPUT / lane, lane, sources)
    # Preserve and verify every recorded failed revision as well as accepted lanes.
    for folder in c.OUTPUT.iterdir():
        need(folder.is_dir() and folder.name in c.LANES, 'Unexpected result path')
        before = read(folder / 'source-before.json')
        for name, digest in before.items(): need(c.sha(folder / 'source' / name) == digest, 'Prior source changed')
        result = read(folder / 'result.json')
        for step in result['steps']:
            row = read(folder / (step + '.json'))
            for suffix in ('stdout', 'stderr'):
                need(c.sha(folder / (step + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'Prior log changed')
    for name in c.FILES:
        raw = c.logger.subprocess.run(['git', '--no-replace-objects', 'show', commit + ':' + name], cwd=c.ROOT, capture_output=True, check=True).stdout
        need(raw.replace(b'\r\n', b'\n') == (c.ROOT / name).read_bytes().replace(b'\r\n', b'\n'), 'Commit drift')
    PACKET.mkdir(parents=True)
    shutil.copytree(c.OUTPUT, PACKET / 'checks')
    c.publish(PACKET / 'audit.json', dict(checked_commit=commit, accepted_lanes=lanes,
        qualification_sha256=base.QUALIFICATION, matching_sealed_current_image_control=True,
        same_bytes_different_inode_refuses=True, matching_forged_context_refuses_authority=True,
        original_owner_transfer=False, protected_procfs_namespace=False, signed_actual_build_enrolled=False,
        credentials_manager_guardians_authenticated=False, original_delivery=False, writer_cessation=False,
        admission_allowed=False, performance_measurement=False, clock_changes=False, service_changes=False,
        server=False, product=False))
    rows = [dict(path=f.relative_to(PACKET).as_posix(), bytes=f.stat().st_size, sha256=c.sha(f)) for f in sorted(PACKET.rglob('*')) if f.is_file()]
    c.publish(PACKET / 'inventory.json', dict(files=rows))
    print(json.dumps(dict(members=len(rows), inventory_sha256=c.sha(PACKET / 'inventory.json')), sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checked-commit', required=True)
    parser.add_argument('--lanes', nargs=4, choices=c.LANES, default=['host', '194', '188', 'metadata'])
    args = parser.parse_args()
    run(args.checked_commit, args.lanes)
