"""Retain closed role consistency, not provenance, writer cessation or admission."""
import argparse
import importlib.util
import json
import shutil
from pathlib import Path
import check as c

spec = importlib.util.spec_from_file_location('census_base_retainer', c.ROOT / 'tools/bootstrap-build-owner-074/retain.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
previous.c = c
PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/bootstrap-child-census-20261023'
need, read = previous.need, previous.read


def verify(folder, lane, sources):
    previous.verify(folder, lane, sources)
    lane = lane.removesuffix('-final')
    if lane in ('host', '194', '188'):
        counts = [('child-units', 5 if lane == 'host' else 15), ('child-cli', 1 if lane == 'host' else 2), ('entry', 2 if lane == 'host' else 7)]
        if lane != 'host': counts += [('census-cli', 3), ('images', 9)]
        for name, count in counts:
            need(f'{count} passed; 0 failed; 0 ignored;' in (folder / (name + '.stdout.log')).read_text(), 'Missing selected tests')
    if lane == '194':
        rows = [json.loads(line) for line in (folder / 'selected-build.stdout.log').read_text().splitlines()]
        artifacts = [r for r in rows if r.get('reason') == 'compiler-artifact' and r.get('target', {}).get('name') == 'diagnostic_held_bootstrap']
        need(len(artifacts) == 1 and artifacts[0]['target']['kind'] == ['bin'] and artifacts[0]['executable'], 'Selected binary missing')
        need(rows[-1] == dict(reason='build-finished', success=True), 'Selected build incomplete')
        digest = (folder / 'image-sha.stdout.log').read_text().split()[0]
        need(len(digest) == 64 and all(x in '0123456789abcdef' for x in digest), 'Invalid unsigned digest')
        need(0 < int((folder / 'image-size.stdout.log').read_text()) <= 128 * 1024 * 1024, 'Image exceeds old bound')


def run(commit):
    need(not PACKET.exists(), 'Never overwrite/reseal')
    sources = {n: c.sha(c.ROOT / n) for n in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == previous.QUALIFICATION, 'Qualification changed')
    for lane in ('host-final', '194-final', '188-final', 'metadata-final'): verify(c.OUTPUT / lane, lane, sources)
    initial = read(c.OUTPUT / 'host/source-before.json')
    need(all(c.sha(c.OUTPUT / 'initial-source' / name) == digest for name, digest in initial.items()), 'Initial source changed')
    for lane in ('host', '194', '188', 'metadata'):
        folder = c.OUTPUT / lane
        need(read(folder / 'source-before.json') == initial, 'Initial source mismatch')
        for step in read(folder / 'result.json')['steps']:
            row = read(folder / (step + '.json'))
            for suffix in ('stdout', 'stderr'):
                need(c.sha(folder / (step + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'Initial output changed')
    for lane in ('194', '188'):
        need(read(c.OUTPUT / lane / 'result.json')['failure'] is not None and 'duplicate_mod' in (c.OUTPUT / lane / 'clippy.stderr.log').read_text(), 'Initial lint refusal missing')
    need(read(c.OUTPUT / 'metadata/result.json')['failure'] is not None and 'unregistered cfg-gated target' in (c.OUTPUT / 'metadata/registry.stderr.log').read_text(), 'Initial registry refusal missing')
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
    c.publish(PACKET / 'audit.json', dict(checked_commit=commit, qualification_sha256=previous.QUALIFICATION,
        semantic_roles=15, complete_forged_role_set_refuses=True, extra_descriptor_refuses=True, actual_private_kernel_controls=True,
        unit_child_context_modeled=True, actual_process_layout_controls=True, original_live_join=False,
        current_executable_authenticated=False, credentials_manager_root_authenticated=False, signed_actual_build_enrolled=False,
        original_owner_fd_transfer=False, original_delivery=False, writers_cessation=False, admission_allowed=False,
        clock_changes=False, service_changes=False, server=False, product=False, performance_measurement=False))
    members = [dict(path=f.relative_to(PACKET).as_posix(), bytes=f.stat().st_size, sha256=c.sha(f)) for f in sorted(PACKET.rglob('*')) if f.is_file()]
    c.publish(PACKET / 'inventory.json', dict(files=members))
    print(json.dumps(dict(members=len(members), inventory_sha256=c.sha(PACKET / 'inventory.json')), sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checked-commit', required=True)
    run(parser.parse_args().checked_commit)
