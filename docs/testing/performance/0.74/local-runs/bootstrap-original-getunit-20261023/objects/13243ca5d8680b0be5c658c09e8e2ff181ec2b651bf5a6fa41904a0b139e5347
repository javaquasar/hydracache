"""One separately authorized original-owner attempt of the unchanged GetUnit candidate."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import native_fixture as old

ROOT=old.ROOT
OUTPUT=ROOT/'target/bootstrap-original-getunit-20261023'
CHECKS=OUTPUT/'checks'
LANE=OUTPUT/'native-once'
BASE='8fb0fb5a'
ARCHIVE=ROOT/'docs/testing/performance/0.74/local-runs/bootstrap-original-service-20261023/index.json'
ARCHIVE_SHA='d3858133295f5d202be21c9dcce48d87051f5690d84a110ee131796f80059f82'
TEST='privileged_original_manager_service_once'

def read(path): return json.loads(path.read_bytes())
def git(*args): return subprocess.check_output(['git',*args],cwd=ROOT,text=True).strip()
def prior_refusal(folder):
    result=read(folder/'result.json')
    if result['returncode']!=101 or result['original_send_observed'] or result['readiness_observed']:
        raise ValueError('Predecessor was not the retained pre-send refusal')
    stdout=(folder/'live.stdout.log').read_text()
    stderr=(folder/'live.stderr.log').read_text()
    if 'never reuse an existing unit' not in stderr or 'HC074-ORIGINAL-SCOPE-INTENT' in stdout:
        raise ValueError('Predecessor progressed past absence guard')
    facts=json.loads(stdout.split('HC074-ORIGINAL-SERVICE-FIRST ',1)[1].splitlines()[0])
    if facts['causal']['observations']!=0 or any(s['calls'] for s in facts['age']['sites']):
        raise ValueError('Predecessor reached original admission')
    if (folder/'postflight-result.txt').read_text().strip()!='test_rc=101 cleanup_rc=0':
        raise ValueError('Predecessor cleanup not confirmed')
    if (folder/'preflight.txt').read_bytes()!=(folder/'postflight.txt').read_bytes():
        raise ValueError('Predecessor context changed')

def selected_binary():
    step=read(CHECKS/'native-libtest.json')
    if step['exit']!=0 or old.sha(CHECKS/'native-libtest.stdout.log')!=step['stdout_sha256']:
        raise ValueError('Build receipt mismatch')
    artifacts=[json.loads(line) for line in (CHECKS/'native-libtest.stdout.log').read_text().splitlines() if line.startswith('{')]
    paths=[v['executable'] for v in artifacts if v.get('reason')=='compiler-artifact' and v.get('executable') and v.get('target',{}).get('name')=='hydracache_long_run_supervisor_074' and v.get('profile',{}).get('test')]
    if len(paths)!=1 or not re.fullmatch('/home/artur/.cache/hydracache-074-194-check/release/deps/hydracache_long_run_supervisor_074-[0-9a-f]+',paths[0]):
        raise ValueError('Ambiguous GNU libtest')
    return Path(r'\\wsl.localhost\Ubuntu')/paths[0].lstrip('/')

def record_build():
    result=read(CHECKS/'result.json')
    before=read(CHECKS/'source-before.json')
    if result['failure'] or before!=read(CHECKS/'source-after.json'):
        raise ValueError('Checks not green and source-stable')
    if any(old.sha(ROOT/name)!=sha for name,sha in before.items()):
        raise ValueError('Checked source changed')
    binary=selected_binary()
    old.durable_json(OUTPUT/'build-proof.json',{'libtest':str(binary),'libtest_sha256':old.sha(binary),
        'bootstrap_sha256':old.sha(old.BOOTSTRAP),'checked_sources':before})

def verify_candidate():
    if git('status','--porcelain'): raise ValueError('Candidate must be committed and clean')
    for path in ('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests','Cargo.lock','docs/COMPAT.md'):
        if git('diff',BASE,'--',path): raise ValueError('Original runtime/fixture changed: '+path)
    if old.sha(ROOT/'docs/testing/performance/0.74/qualification-manifest.toml')!=old.QUAL or old.sha(ARCHIVE)!=ARCHIVE_SHA:
        raise ValueError('Frozen qualification or prior evidence changed')
    prior_refusal(old.OUT/'native-account-compatible-once')
    proof=read(OUTPUT/'build-proof.json')
    if any(old.sha(ROOT/name)!=sha for name,sha in proof['checked_sources'].items()):
        raise ValueError('Checked candidate changed')
    if proof['libtest']!=str(selected_binary()) or old.sha(Path(proof['libtest']))!=proof['libtest_sha256'] or old.sha(old.BOOTSTRAP)!=proof['bootstrap_sha256']:
        raise ValueError('Built artifact changed')
    return proof

def outcome(code,live):
    value={'returncode':code,'retry':False,
        'original_send_observed':'original_send=true manager_ack=true' in live,
        'readiness_observed':'readiness=true release=false completion=false' in live}
    if code==0 and (not value['original_send_observed'] or not value['readiness_observed'] or live.count('running 1 test')!=1):
        value['validation_error']='No exact positive original readiness result'
        value['returncode']=74
    return value

def run():
    if os.name!='nt': raise ValueError('Windows orchestrator only')
    proof=verify_candidate()
    LANE.mkdir(parents=True,exist_ok=False)
    for name,source,expected in [('bootstrap',old.BOOTSTRAP,proof['bootstrap_sha256']),('libtest',Path(proof['libtest']),proof['libtest_sha256'])]:
        shutil.copyfile(source,LANE/name)
        if old.sha(LANE/name)!=expected: raise ValueError('Artifact copy changed')
    bundle=ROOT/'target/diagnostic-start-material-bundle-20261017'
    if not (bundle/'timing-controls-074').is_file(): raise ValueError('Missing original bundle')
    intent={'git':git('rev-parse','HEAD'),'baseline':BASE,'distro':old.DISTRO,'bootstrap':proof['bootstrap_sha256'],
        'libtest':proof['libtest_sha256'],'original_windows':[300,60,10],'qualification':old.QUAL,
        'server':False,'product':False,'retry':False,'release_install':False,'existing_services_changed':False}
    old.durable_json(LANE/'intent.json',intent)
    command=['wsl.exe','-d',old.DISTRO,'-u','root','--cd','/','--exec','/bin/sh',
        old.linux(ROOT/'tools/bootstrap-original-service-074/native_fixture.sh'),old.linux(LANE),old.linux(bundle),intent['bootstrap'],intent['libtest']]
    with (LANE/'session.stdout.log').open('xb') as out, (LANE/'session.stderr.log').open('xb') as err:
        child=subprocess.run(command,stdout=out,stderr=err,check=False)
    live=(LANE/'live.stdout.log').read_text(errors='replace') if (LANE/'live.stdout.log').exists() else ''
    result=outcome(child.returncode,live)
    result.update(stdout_sha256=old.sha(LANE/'session.stdout.log'),stderr_sha256=old.sha(LANE/'session.stderr.log'))
    old.durable_json(LANE/'result.json',result)
    print(json.dumps(result))
    raise SystemExit(result['returncode'])

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode',choices=['record-build','run-once'])
    record_build() if parser.parse_args().mode=='record-build' else run()
