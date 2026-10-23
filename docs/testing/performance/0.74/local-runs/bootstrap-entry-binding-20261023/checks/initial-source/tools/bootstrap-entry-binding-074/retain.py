"""Exclusively retain entry assertions, not child execution or live authentication."""
import argparse
import importlib.util
import json
import shutil
from pathlib import Path
import check as c

spec = importlib.util.spec_from_file_location('entry_previous_retainer', c.ROOT / 'tools/bootstrap-build-owner-074/retain.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
previous.c = c
PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/bootstrap-entry-binding-20261023'
need, read = previous.need, previous.read


def verify(folder, lane, sources):
    previous.verify(folder, lane, sources)
    count = 2 if lane == 'host' else 7
    if lane in ('host', '194', '188'):
        need(f'{count} passed; 0 failed; 0 ignored;' in (folder / 'entry.stdout.log').read_text(), 'Missing entry tests')
    if lane in ('194', '188'):
        need('9 passed; 0 failed; 0 ignored;' in (folder / 'images.stdout.log').read_text(), 'Missing previous image tests')


def run(commit):
    need(not PACKET.exists(), 'Never overwrite/reseal')
    sources = {n: c.sha(c.ROOT / n) for n in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == previous.QUALIFICATION, 'Qualification changed')
    for lane in ('host', '194', '188', 'metadata'): verify(c.OUTPUT / lane, lane, sources)
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
        descriptors=27, semantic_roles=15, entry_bytes=1024, modeled_original_context=True, actual_private_entry_fd_checks=True,
        original_live_join=False, authenticated_child_entry=False, fd_transfer=False, remapping=False, bootstrap_executed=False,
        original_delivery=False, writers_cessation=False, admission_allowed=False, clock_changes=False, service_changes=False,
        server=False, product=False, performance_measurement=False))
    members = [dict(path=f.relative_to(PACKET).as_posix(), bytes=f.stat().st_size, sha256=c.sha(f)) for f in sorted(PACKET.rglob('*')) if f.is_file()]
    c.publish(PACKET / 'inventory.json', dict(files=members))
    print(json.dumps(dict(members=len(members), inventory_sha256=c.sha(PACKET / 'inventory.json')), sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checked-commit', required=True)
    run(parser.parse_args().checked_commit)
