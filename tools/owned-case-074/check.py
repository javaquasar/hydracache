"""Separate bounded compilation and one ordinary owned-case control execution."""
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]

TAGS = ('v3-output', 'v4-fd', 'v5-material', 'v6-material-bytes',
        'v7-frozen-input', 'v8-bootstrap-isolation', 'v9-full-ordinary')

def selected_artifact(lines, name, target):
    artifacts = []
    for line in lines:
        row = json.loads(line)
        if row.get('reason') == 'compiler-artifact' and row.get('target', {}).get('name') == name and row.get('executable'):
            artifacts.append(row['executable'])
    if len(artifacts) != 1:
        raise ValueError('One exact integration artifact required')
    path = PurePosixPath(artifacts[0])
    if str(path.parent) != target + '/release/deps' or not re.fullmatch(name + r'-[0-9a-f]+', path.name):
        raise ValueError('Unexpected native artifact path')
    return path

def selection(tag):
    if tag not in TAGS:
        raise ValueError('Distinct reviewed candidate required')
    lib = tag in ('v4-fd', 'v5-material', 'v6-material-bytes', 'v7-frozen-input', 'v9-full-ordinary')
    name = 'hydracache_long_run_supervisor_074' if lib else 'held_launcher'
    selectors = ['owned_case_controls::']
    if tag == 'v4-fd':
        selectors = ['cgroup_root_transport_every_bound_byte_refuses_and_closes_rights',
                     'held_material_kernel_drop_closes_all_six_resources_and_partial_creation_is_raii']
    elif tag in ('v5-material', 'v6-material-bytes', 'v7-frozen-input'):
        selectors = ['diagnostic_artifacts::linux::io_binding_tests::']
    elif tag in ('v8-bootstrap-isolation', 'v9-full-ordinary'):
        selectors = []
    if tag == 'v8-bootstrap-isolation':
        name = 'bootstrap_current_image'
    return lib, name, selectors
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
def snapshot():
    paths = [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock']
    frozen = ROOT / 'docs/testing/performance/0.74/local-runs/material-lock-frozen-input-20261024/objects/be46eaf8e97e507bda5e3f2b671d0d622e6a639ce1d4613769a4693723d62b19'
    if frozen.exists():
        paths.append(frozen)
    for folder in ('tools/long-run-supervisor-074', 'tools/owned-case-074', 'tools/material-lock-fixture-074'):
        paths += sorted(p for p in (ROOT / folder).rglob('*') if p.is_file() and '__pycache__' not in p.parts)
    return {p.relative_to(ROOT).as_posix(): sha(p) for p in paths}
def main(lane, tag):
    version = {'194': '1.94.0', '188': '1.88.0'}[lane]
    fd_lane, artifact_name, selectors = selection(tag)
    out = ROOT / ('target/status-field-scan-20261024/owned-case-' + lane + '-' + tag)
    out.mkdir()
    before = snapshot()
    prefix = ['wsl.exe','-d','Ubuntu','-u','artur','--exec','/usr/bin/timeout','--signal=TERM','--kill-after=5s']
    target = '/home/artur/.cache/hydracache-074-' + lane + '-check'
    build = prefix + ['1200s','/usr/bin/env','RUSTUP_AUTO_INSTALL=0','CARGO_INCREMENTAL=0',
        'CARGO_TARGET_DIR=' + target,'/home/artur/.cargo/bin/cargo','+' + version,
        'test','--offline','--locked','-p','hydracache-long-run-supervisor-074','--release',
        '--all-features']
    build += ['--lib'] if fd_lane else ['--test',artifact_name]
    build += ['--no-run','--message-format=json']
    def invoke(name, argv):
        (out/(name+'-intent.json')).write_text(json.dumps(dict(argv=argv,source=before,
            server=False,services=False,clock_changes=False,original_owner_manual_run=False,
            retry=False),indent=2),encoding='utf-8')
        with (out/(name+'.stdout.log')).open('xb') as stdout, (out/(name+'.stderr.log')).open('xb') as stderr:
            result = subprocess.run(argv,cwd=ROOT,stdout=stdout,stderr=stderr,check=False)
        (out/(name+'-exit.json')).write_text(json.dumps(dict(exit=result.returncode)),encoding='utf-8')
        if result.returncode:
            raise ValueError('First ' + name + ' refusal retained')
    failure, image = None, None
    try:
        invoke('build',build)
        if snapshot() != before:
            raise ValueError('Sources changed during compile')
        path=selected_artifact((out/'build.stdout.log').read_text().splitlines(), artifact_name, target)
        executable=str(path)
        native=Path(r'\\wsl.localhost\Ubuntu').joinpath(*path.parts[1:])
        image=dict(path=executable,sha256=sha(native),bytes=native.stat().st_size)
        shutil.copyfile(native,out/'image')
        if sha(out/'image')!=image['sha256']:
            raise ValueError('Pinned image copy mismatch')
        (out/'image.json').write_text(json.dumps(image,indent=2),encoding='utf-8')
        bound = '1800s' if tag == 'v9-full-ordinary' else '180s'
        invoke('run',prefix+[bound,executable]+selectors+['--nocapture','--test-threads=4'])
        if sha(native)!=image['sha256']:
            raise ValueError('Native integration image changed during execution')
    except Exception as error:
        failure=str(error)
    after=snapshot()
    receipt=dict(failure=failure,source_unchanged=before==after,source_after=after,image=image,
        admission=False,server=False,services=False,clock_changes=False,product=False,
        original_owner_manual_run=False)
    (out/'result.json').write_text(json.dumps(receipt,indent=2),encoding='utf-8')
    print(json.dumps({k:v for k,v in receipt.items() if k!='source_after'}),flush=True)
    raise SystemExit(int(failure is not None or before!=after))
if __name__=='__main__':
    main(sys.argv[1],sys.argv[2])
