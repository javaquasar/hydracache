"""One explicit isolated WSL diagnostic attempt. No installation of the release."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'target/bootstrap-original-service-20261023'
DISTRO='HydraCache074Native'
QUAL='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
BOOTSTRAP=Path(r'\\wsl.localhost\Ubuntu\home\artur\.cache\hydracache-074-194-check\x86_64-unknown-linux-gnu\release\diagnostic_held_bootstrap')

def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def linux(path):
    value=path.resolve().as_posix()
    if not value.startswith('C:/Workspace/prj/jq/cashe/hydracache-074/'):
        raise ValueError('Only this workspace is admitted')
    return '/mnt/c/'+value[3:]

def run(binary):
    if os.name != 'nt': raise ValueError('Windows orchestrator only')
    if sha(ROOT/'docs/testing/performance/0.74/qualification-manifest.toml') != QUAL:
        raise ValueError('Qualification manifest changed')
    if not binary.is_file() or binary.parent != BOOTSTRAP.parents[2]/'release/deps':
        raise ValueError('Only the freshly selected GNU libtest path is admitted')
    lane=OUT/'native-first'
    lane.mkdir(parents=True,exist_ok=False)  # No retry-to-pass or first-result overwrite.
    bundle=ROOT/'target/diagnostic-start-material-bundle-20261017'
    if not (bundle/'timing-controls-074').is_file(): raise ValueError('Missing original bundle')
    for name, source in [('bootstrap',BOOTSTRAP),('libtest',binary)]:
        shutil.copyfile(source,lane/name)
    intent={'distro':DISTRO,'bootstrap':sha(lane/'bootstrap'),'libtest':sha(lane/'libtest'),
        'qualification':QUAL,'original_windows':[300,60,10],'server':False,'product':False,
        'release_install':False,'existing_services_changed':False,'retry':False,
        'git':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()}
    (lane/'intent.json').write_text(json.dumps(intent,indent=2)+'\n',encoding='utf-8')
    with (lane/'intent.json').open('rb') as receipt: os.fsync(receipt.fileno())
    command=['wsl.exe','-d',DISTRO,'-u','root','--cd','/','--exec','/bin/sh',
        linux(ROOT/'tools/bootstrap-original-service-074/native_fixture.sh'),
        linux(lane),linux(bundle),intent['bootstrap'],intent['libtest']]
    # The inner timeout owns only its fresh process group. No outer re-dispatch.
    with (lane/'session.stdout.log').open('xb') as out, (lane/'session.stderr.log').open('xb') as err:
        result=subprocess.run(command,stdout=out,stderr=err,check=False)
    receipt={'returncode':result.returncode,'stdout_sha256':sha(lane/'session.stdout.log'),
        'stderr_sha256':sha(lane/'session.stderr.log'),'retry':False}
    live=(lane/'live.stdout.log').read_text(errors='replace') if (lane/'live.stdout.log').exists() else ''
    receipt['original_send_observed']='original_send=true manager_ack=true' in live
    receipt['readiness_observed']='readiness=true release=false completion=false' in live
    if result.returncode == 0 and (not receipt['readiness_observed'] or live.count('running 1 test') != 1):
        receipt['validation_error']='No exact positive original readiness result'
        result.returncode=74
    (lane/'result.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(receipt))
    raise SystemExit(result.returncode)

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--libtest',type=Path,required=True)
    parser.add_argument('--run-isolated-once',action='store_true',required=True)
    args=parser.parse_args()
    run(args.libtest)
