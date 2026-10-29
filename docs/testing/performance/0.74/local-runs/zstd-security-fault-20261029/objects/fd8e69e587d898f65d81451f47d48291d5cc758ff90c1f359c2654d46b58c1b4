"""Retain exact local readiness controls, not original execution admission."""
import argparse
import importlib.util
import json
import re
import shutil
import subprocess
import check as c
spec = importlib.util.spec_from_file_location('delivery_retention', c.ROOT / 'tools/bootstrap-held-delivery-074/retain.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
previous.c = c
need, logs, count = previous.need, previous.logs, previous.count
PACKET = c.ROOT / 'docs/testing/performance/0.74/local-runs/bootstrap-held-entry-20261023'
QUALIFICATION = previous.QUALIFICATION
METHOD = ['tools/bootstrap-held-entry-074/' + n for n in ('retain.py', 'test_retain.py', 'README.md')]

def read(path): return json.loads(path.read_bytes())

def verify(folder, lane, sources):
    expected = c.commands(lane)
    result = logs(folder)
    need(result == dict(lane=lane, failure=None, steps=[n for n, _ in expected], original_attempt=False, server=False, product=False), 'Failed/incomplete lane')
    need(read(folder / 'source-before.json') == read(folder / 'source-after.json') == sources, 'Source drift')
    for name, argv in expected:
        row = read(folder / (name + '.json'))
        need(row['exit'] == 0 and row['argv'] == argv, 'Command drift')
    # Individual commands are checked above; require actual tests, not an empty filter.
    for name, argv in expected:
        if 'test' in argv:
            need(count(folder, name) > 0, 'Empty test filter')
    if lane.split('-', 1)[0] == 'metadata':
        need((folder / 'registry.stdout.log').read_text().strip() == 'gated-test-check: OK (188 discovered gates, all registered)', 'Registry drift')
    kind = lane.split('-', 1)[0]
    if kind == 'host':
        need(count(folder,'canaries') == 70, 'Source-contract count drift')
        need(sum(count(folder,n) for n in ('codec','child-units','child-cli','entry','build')) == 26, 'Host count drift')
    if kind in ('194','188'):
        need(count(folder,'held-entry') == 4 and count(folder,'image-cli') == 17, 'Entry/fault count drift')
        names = ('held-entry','held','parser','sandbox','installed','image-cli','census-cli','child-units','child-cli','entry','images','build','selection','owner','enrollment','old-source','old-replacement','old-keys')
        need(sum(count(folder,n) for n in names) == 153, 'Selected count drift')

def run(commit, lanes):
    need(not PACKET.exists(), 'Never overwrite/reseal')
    need(re.fullmatch('[0-9a-f]{40}', commit) is not None, 'Exact checked commit required')
    need(len(lanes) == 4 and {n.split('-', 1)[0] for n in lanes} == {'host','194','188','metadata'}, 'Wrong final lane set')
    sources = {n:c.sha(c.ROOT / n) for n in c.FILES}
    need(sources['docs/testing/performance/0.74/qualification-manifest.toml'] == QUALIFICATION, 'Qualification drift')
    for lane in lanes: verify(c.OUTPUT / lane, lane, sources)
    prior = []
    for folder in sorted(c.OUTPUT.iterdir()):
        need(folder.is_dir() and folder.name in c.LANES, 'Unknown output')
        result = logs(folder)
        prior.append(dict(lane=folder.name, failure=result['failure'], steps=result['steps']))
    baseline = c.OUTPUT / 'baseline'
    need(logs(baseline)['failure'] is not None and read(baseline / 'missing.json')['exit'] == 101, 'Missing first baseline')
    for name in c.FILES + METHOD:
        blob = subprocess.run(['git','--no-replace-objects','show',commit+':'+name],cwd=c.ROOT,capture_output=True,check=True).stdout
        need(blob.replace(b'\r\n',b'\n') == (c.ROOT/name).read_bytes().replace(b'\r\n',b'\n'), 'Commit drift')
    PACKET.mkdir(parents=True)
    shutil.copytree(c.OUTPUT, PACKET / 'checks')
    for name in METHOD:
        dest = PACKET/'method'/name
        dest.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(c.ROOT/name,dest)
    c.publish(PACKET/'audit.json',dict(checked_commit=commit,final_lanes=lanes,source_count=len(sources),
        qualification_sha256=QUALIFICATION,prior_lanes=prior,integrated_readiness_entry=True,
        old_modes_unchanged=True,local_owned_child_fault_controls=True,
        positive_original_manager_delivery=False,authenticated_original_start=False,
        loaded_unit_hardening_proof=False,new_path_full_sigkill_matrix=False,
        writer_cessation=False,completion=False,admission_allowed=False,
        original_attempt=False,clock_changes=False,service_changes=False,namespace_changes=False,
        server=False,product=False,performance_measurement=False))
    rows=[dict(path=p.relative_to(PACKET).as_posix(),bytes=p.stat().st_size,sha256=c.sha(p)) for p in sorted(PACKET.rglob('*')) if p.is_file()]
    c.publish(PACKET/'inventory.json',dict(files=rows))
    print(json.dumps(dict(members=len(rows),inventory_sha256=c.sha(PACKET/'inventory.json')),sort_keys=True))

if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checked-commit',required=True)
    parser.add_argument('--lanes',nargs=4,choices=c.LANES,required=True)
    args=parser.parse_args()
    run(args.checked_commit,args.lanes)
