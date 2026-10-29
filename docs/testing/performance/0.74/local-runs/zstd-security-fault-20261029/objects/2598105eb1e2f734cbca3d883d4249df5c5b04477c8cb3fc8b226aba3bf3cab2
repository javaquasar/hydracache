"""Retain the failed bind gate separately from bootstrap selector checks."""
import argparse
import json
import shutil
from pathlib import Path
import check as c

PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/held-bind-bootstrap-20261023'
QUALIFICATION = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'


def need(value, message):
    if not value: raise ValueError(message)


def read(path): return json.loads(path.read_bytes())


def verify_lane(folder, lane, sources):
    result = read(folder / 'result.json')
    expected = c.commands(lane)
    need(result['failure'] is None and result['lane'] == lane and result['steps'] == [n for n, _ in expected], 'Failed/incomplete checks')
    need(not any(result[key] for key in ('server', 'product', 'original_attempt')), 'Wrong scope')
    need(read(folder / 'source-before.json') == read(folder / 'source-after.json') == sources, 'Source drift')
    for name, argv in expected:
        row = read(folder / (name + '.json'))
        need(row['exit'] == 0 and row['argv'] == argv, 'Changed/failed command')
        for suffix in ('stdout', 'stderr'):
            need(c.sha(folder / (name + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'Changed log')


def verify_negative(folder, sources):
    need(read(folder / 'red.json')['exit'] == 101, 'Bind feature gate must stay red')
    need(read(folder / 'bootstrap-red.json')['exit'] == 101, 'Bootstrap integration red baseline missing')
    probe = read(folder / 'probe.json')
    need(probe['exit'] == 1 and probe['source_sha256'] == sources['tools/held-bind-alias-074/probe.py'], 'Changed sealed-source refusal')
    need(b'Errno 22' in (folder / 'probe.stderr.log').read_bytes() and
         not (folder / 'probe.stdout.log').read_bytes(), 'Expected first open_tree refusal missing')
    regular = read(folder / 'regular.json')
    need(regular['exit'] == 1, 'Do not promote failed regular-file control')
    for name, digest in regular['source_sha256'].items():
        need(digest == sources['tools/held-bind-alias-074/' + name], 'Changed diagnostic source')
    observation = read(folder / 'regular.stdout.log')
    need(not observation['source_sealed'] and not observation['original_candidate_pass'] and
         not observation['eligible_original_launcher'], 'Wrong source or promoted gate')
    need(observation['creator_sigkill'] and observation['mount_survived'] and observation['source_fd_closed'], 'Lost partial observation')
    need(b'Device or resource busy' in (folder / 'regular.stderr.log').read_bytes(), 'First cleanup failure missing')


def run(commit):
    need(not PACKET.exists(), 'Never overwrite or reseal')
    sources = {name: c.sha(c.ROOT / name) for name in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == QUALIFICATION, 'Qualification changed')
    verify_negative(c.OUTPUT, sources)
    for lane in ('host', '194', '188', 'metadata'): verify_lane(c.OUTPUT / lane, lane, sources)
    for lane in ('194', '188'):
        for name, count in (('bootstrap', 5), ('held-source', 1), ('held-replacement', 1), ('held-keys', 1)):
            need(f'{count} passed; 0 failed; 0 ignored;' in (c.OUTPUT / lane / (name + '.stdout.log')).read_text(), 'Wrong selected tests')
    for name in c.FILES:
        raw = c.logger.subprocess.run(['git', '--no-replace-objects', 'show', commit + ':' + name],
                                     cwd=c.ROOT, capture_output=True, check=True).stdout
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
    rejected = c.ROOT / 'docs/testing/performance/0.74/rejected-candidates/held-bind-alias-20261023'
    for file in rejected.iterdir():
        dest = PACKET / 'rejected' / file.name
        dest.parent.mkdir(exist_ok=True); shutil.copyfile(file, dest)
    c.publish(PACKET / 'audit.json', dict(checked_commit=commit, qualification_sha256=QUALIFICATION,
        sealed_memfd_bind_gate=False, regular_file_whole_gate=False, regular_file_partial_observation=True,
        bootstrap_protected_selector_checks=True, bootstrap_execution=False, server_enrollment=False,
        original_delivery=False, writers_cessation=False, admission_allowed=False, clock_changes=False,
        service_changes=False, product=False, server=False, original_attempt=False))
    files = [dict(path=file.relative_to(PACKET).as_posix(), bytes=file.stat().st_size, sha256=c.sha(file))
             for file in sorted(PACKET.rglob('*')) if file.is_file()]
    c.publish(PACKET / 'inventory.json', dict(files=files))
    print(json.dumps(dict(members=len(files), inventory_sha256=c.sha(PACKET / 'inventory.json')), sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checked-commit', required=True)
    run(parser.parse_args().checked_commit)
