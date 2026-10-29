"""Seal component evidence, never execution enrollment or a retry-to-pass result."""
import argparse
import json
import re
import shutil
import subprocess
import check as c
PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/bootstrap-held-delivery-20261023'
QUALIFICATION = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
METHOD = ['tools/bootstrap-held-delivery-074/' + name for name in ('retain.py', 'test_retain.py', 'README.md')]

def need(ok, reason):
    if not ok: raise ValueError(reason)

def read(path): return json.loads(path.read_bytes())

def logs(folder):
    result = read(folder / 'result.json')
    sources = read(folder / 'source-before.json')
    for name, digest in sources.items():
        need(c.sha(folder / 'source' / name) == digest, 'Snapshot drift')
    for name in result['steps']:
        row = read(folder / (name + '.json'))
        for suffix in ('stdout', 'stderr'):
            need(c.sha(folder / (name + '.' + suffix + '.log')) == row[suffix + '_sha256'], 'Log drift')
    return result

def count(folder, name):
    return sum(map(int, re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;',
        (folder / (name + '.stdout.log')).read_text())))

def verify(folder, lane, sources):
    expected = c.commands(lane)
    result = logs(folder)
    need(result == dict(lane=lane, failure=None, steps=[n for n, _ in expected], original_attempt=False, server=False, product=False), 'Failed/incomplete lane')
    need(read(folder / 'source-before.json') == read(folder / 'source-after.json') == sources, 'Source drift')
    for name, argv in expected:
        row = read(folder / (name + '.json'))
        need(row['exit'] == 0 and row['argv'] == argv, 'Command drift')
    kind = lane.split('-', 1)[0]
    if kind == 'host':
        need(count(folder, 'canaries') == 68, 'Source contract count drift')
        need(sum(count(folder, n) for n in ('codec', 'child-units', 'child-cli', 'entry', 'build')) == 24, 'Host control drift')
    if kind in ('194', '188'):
        need(count(folder, 'held') == 12 and count(folder, 'parser') == 4, 'New control count drift')
        names = ('held', 'parser', 'sandbox', 'installed', 'image-cli', 'census-cli', 'child-units', 'child-cli', 'entry', 'images', 'build', 'selection', 'owner', 'enrollment', 'old-source', 'old-replacement', 'old-keys')
        need(sum(count(folder, n) for n in names) == 128, 'Selected control count drift')
    if kind == 'metadata':
        need((folder / 'registry.stdout.log').read_text().strip() == 'gated-test-check: OK (188 discovered gates, all registered)', 'Registry drift')

def run(commit, lanes):
    need(not PACKET.exists(), 'Never overwrite/reseal')
    need(re.fullmatch('[0-9a-f]{40}', commit) is not None, 'Exact checked commit required')
    need(len(lanes) == 4 and {n.split('-', 1)[0] for n in lanes} == {'host', '194', '188', 'metadata'}, 'Wrong final lane set')
    sources = {n: c.sha(c.ROOT / n) for n in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == QUALIFICATION, 'Qualification drift')
    for lane in lanes: verify(c.OUTPUT / lane, lane, sources)
    prior = []
    for folder in sorted(c.OUTPUT.iterdir()):
        need(folder.is_dir() and folder.name in c.LANES, 'Unknown output')
        result = logs(folder)
        prior.append(dict(lane=folder.name, failure=result['failure'], steps=result['steps']))
    # Pin both the selected compilation sources and these retention tools to Git.
    for name in c.FILES + METHOD:
        blob = subprocess.run(['git', '--no-replace-objects', 'show', commit + ':' + name], cwd=c.ROOT, capture_output=True, check=True).stdout
        need(blob.replace(b'\r\n', b'\n') == (c.ROOT / name).read_bytes().replace(b'\r\n', b'\n'), 'Commit drift')
    PACKET.mkdir(parents=True)
    shutil.copytree(c.OUTPUT, PACKET / 'checks')
    for name in METHOD:
        dest = PACKET / 'method' / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(c.ROOT / name, dest)
    c.publish(PACKET / 'audit.json', dict(checked_commit=commit, final_lanes=lanes,
        source_count=len(sources), qualification_sha256=QUALIFICATION, prior_lanes=prior,
        owner_derived_constructor_compiled=True, exact_manager_transport_compiled=True,
        local_kernel_peer_exchange_control=True, one_shot_refusal_controls=True,
        positive_original_manager_delivery=False, integrated_child_entry=False,
        loaded_unit_hardening_proof=False, authenticated_original_start=False,
        new_path_sigkill_matrix=False, writer_cessation=False, completion=False,
        admission_allowed=False, original_attempt=False, clock_changes=False,
        service_changes=False, namespace_changes=False, server=False, product=False,
        performance_measurement=False, raw_absence_baseline_saved=False,
        initial_lifetime_diagnostic_raw_saved=False))
    rows = [dict(path=p.relative_to(PACKET).as_posix(), bytes=p.stat().st_size, sha256=c.sha(p)) for p in sorted(PACKET.rglob('*')) if p.is_file()]
    c.publish(PACKET / 'inventory.json', dict(files=rows))
    print(json.dumps(dict(members=len(rows), inventory_sha256=c.sha(PACKET / 'inventory.json')), sort_keys=True))

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checked-commit', required=True)
    parser.add_argument('--lanes', nargs=4, choices=c.LANES, default=['host-reviewed', '194-reviewed', '188-reviewed', 'metadata-reviewed'])
    args = parser.parse_args()
    run(args.checked_commit, args.lanes)
