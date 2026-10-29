"""One explicitly approved new context, no replay of the original cohort."""
import hashlib
import io
import json
from pathlib import Path
import re
import shlex
import subprocess
import sys
import tarfile

ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent;OUT=ROOT/'target/chitchat-cohort-r3-20261030-r2'
sys.path.insert(0,str(ROOT/'tools/chitchat-host-cost-074'));import dispatch as d;import remote
d.OUT=OUT
NAMES=('coordinator.py','worker.py','plan.py','stage.py','namespace.py','recovery.py')
BASE_SHA='fcb8a53eba5ebc7e1364cbfa0d2bcb9e0ad1098f75b4e53dae56b7503c02ed38'

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()

def prepare():
    payload=OUT/'payload';payload.mkdir()
    for name in NAMES:(payload/name).write_bytes((HERE/name).read_bytes())
    raw=(ROOT/'tools/get-owner-scheduled-controls-074/maintenance_d3.py').read_bytes();assert hashlib.sha256(raw).hexdigest()==BASE_SHA
    (payload/'maintenance_base.py').write_bytes(raw)
    lines=(OUT/'preflight.stdout.log').read_text().splitlines();host=json.loads(lines[0]);caps=json.loads(lines[1].removeprefix('CAPABILITIES:'))
    assert caps['setns'] and caps['unshare']=='/usr/bin/unshare' and caps['free']>=4*1024**3
    assert dict(x.split('=',1) for x in host['supervisor'].splitlines())['MainPID']=='411840'
    remote.put(payload/'anchor.json',dict(host=host,operator_approved=True,approval='new corrections and matrix 2026-10-29',
        predecessor_index_sha256='5e4b0f0282d3cd7e329a25f2abe236201d023d6e3a682a79915bcbd9abdde1fa'))
    frame=json.loads((ROOT/'target/chitchat-host-cost-20261029/prepared/input/frame.json').read_bytes())
    for n in ('Cargo.toml','Cargo.lock','.cargo/config.toml','deny.toml','docs/testing/performance/0.74/qualification-manifest.toml',
        'scripts/ci/watchdog_linux_group.py','scripts/ci/watchdog_windows_job.py','tools/windows-workspace-preflight-074/owned.py'):assert sha(ROOT/n)==frame[n]
    archive=OUT/'payload.tar.gz'
    with tarfile.open(archive,'x:gz') as t:
        for path in sorted(payload.iterdir()):t.add(path,arcname=path.name,recursive=False)
    remote.put(OUT/'prepared.json',dict(files={p.name:sha(p) for p in payload.iterdir()},archive_sha256=sha(archive),
        archive_bytes=archive.stat().st_size,protocol_sha256=sha(ROOT/'tools/chitchat-cost-074/protocol.json'),operator_approved=True))
    print(json.dumps(dict(prepared=True,archive_bytes=archive.stat().st_size)))

def stage():
    assert not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT).strip(),'complete committed source before dispatch'
    p=json.loads((OUT/'prepared.json').read_bytes());assert sha(OUT/'payload.tar.gz')==p['archive_sha256']
    for n in NAMES:assert sha(HERE/n)==p['files'][n]
    remote.put(OUT/'stage-once.json',dict(head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),retry=False))
    d.call('upload-directory',d.SSH+['umask 077; mktemp -d /tmp/hc074-chitchat-r3-upload-20261030.XXXXXX'])
    upload=(OUT/'upload-directory.stdout.log').read_text().strip();assert re.fullmatch(r'/tmp/hc074-chitchat-r3-upload-20261030\.[A-Za-z0-9]{6}',upload)
    d.call('upload',d.SCP+[str(OUT/'payload.tar.gz'),'hydracache-ax42:'+upload+'/payload.tar.gz'])
    script='''set -eu
umask 077
test "$(id -u):$(id -g)" = 0:0
upload=UPLOAD
test ! -L "$upload" && test "$(stat -c '%a:%u:%g' "$upload")" = 700:1000:1000
root=$(mktemp -d /tmp/hc074-chitchat-r3-package-20261030.XXXXXX)
install -m 400 "$upload/payload.tar.gz" "$root/payload.tar.gz"
cd "$root"
printf '%s  payload.tar.gz\\n' 'ARCHIVE' | sha256sum --check
/usr/bin/python3 -B -c 'import tarfile;t=tarfile.open("payload.tar.gz");a=t.getmembers();assert len(a)==8 and len({x.name for x in a})==8 and all(x.isfile() and "/" not in x.name and x.name.endswith((".py",".json")) for x in a);t.extractall(".",members=a,filter="data")'
find "$root" -type f -exec chmod 400 {} +
printf 'ROOT=%s\\n' "$root"
'''.replace('UPLOAD',shlex.quote(upload)).replace('ARCHIVE',p['archive_sha256'])
    (OUT/'stage.sh').write_text(script,encoding='ascii',newline='\n');d.call('stage',d.SSH+['sudo -n /bin/sh -s'],data=script.encode())
    matches=re.findall(r'^ROOT=(/tmp/hc074-chitchat-r3-package-20261030\.[A-Za-z0-9]{6})$',(OUT/'stage.stdout.log').read_text(),re.M);assert len(matches)==1
    remote.put(OUT/'remote.json',dict(root=matches[0],retry=False));print(matches[0])

def package():
    name=json.loads((OUT/'remote.json').read_bytes())['root'];assert re.fullmatch(r'/tmp/hc074-chitchat-r3-package-20261030\.[A-Za-z0-9]{6}',name);return name

def phase(mode):
    assert mode in ('faults','worker-controls','context-start','context-check','prepare','launch')
    d.call('remote-'+mode,d.SSH+['sudo -n /usr/bin/python3 -B -I '+package()+'/stage.py '+mode],bound=90)
    print((OUT/('remote-'+mode+'.stdout.log')).read_text())

def matrix():
    assert not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT).strip(),'committed reviewed matrix caller'
    prepared=json.loads((OUT/'prepared.json').read_bytes())
    for n in NAMES:assert sha(HERE/n)==prepared['files'][n]
    remote.put(OUT/'matrix-once.json',dict(retry=False,operator_approved=True,protocol_sha256=prepared['protocol_sha256']))
    phase('prepare');v=json.loads((OUT/'remote-prepare.stdout.log').read_bytes());remote.put(OUT/'numerical.json',v)
    d.call('numeric-plan',d.SSH+['sudo -n cat -- '+v['root']+'/plan.json'])
    raw=(OUT/'numeric-plan.stdout.log').read_bytes();assert hashlib.sha256(raw).hexdigest()==v['plan_sha256'];p=json.loads(raw)
    (OUT/'numeric-plan.json').write_bytes(raw)
    assert [(a['mode'],a['variant']) for a in p['attempts']]==[(m,x) for m in ('cpu','memory') for x in ('B','P','Q','Q','P','B')*5]
    assert p['operator_approved'] and p['supervisor']=='hydracache-performance-supervisor-074.service' and p['fence_identity']==[2306,18101991]
    assert [p[k] for k in ('outer_bound_seconds','session_bound_seconds','capacity_poll_seconds','cleanup_seconds')]==[1200,1100,5,5]
    assert len(p['inputs'])==5750 and p['namespace']['outer_bound_seconds']==1200 and p['namespace']['identity'][1]>0
    assert p['cpu_profile']==dict(online='0-7',system_slice_effective='0-7') and p['protocol_sha256']==prepared['protocol_sha256']
    for n in (*NAMES,'maintenance_base.py'):assert p['inputs'][v['root']+'/'+n]==prepared['files'][n]
    remote.put(OUT/'prelaunch-review.json',dict(plan_sha256=v['plan_sha256'],approved=True,inputs=5750,processes=60,partial_cohort_spliced=False))
    phase('launch')

def readback(mode):
    code='''import json,os,pathlib,sys,tarfile
p=pathlib.Path(PACKAGE)
if MODE=='status':
 r=pathlib.Path(json.loads((p/'numerical.json').read_bytes())['root']);ns=('cohort-complete.json','all-writers-stopped.json','restored.json','host-postflight.json','postflight-result.json','namespace-retired.json','terminal-complete.json')
 print(json.dumps(dict(root=str(r),retired=len(list(r.glob('[0-9]*.retired.json'))),files={n:(r/n).is_file() for n in ns},postflight=json.loads((r/'postflight-result.json').read_bytes()) if (r/'postflight-result.json').is_file() else None)))
else:
 roots=[p]
 for f in ('faults-once/result.json','worker-controls-once/result.json','context-once/result.json','numerical.json'):
  if (p/f).is_file():
   v=json.loads((p/f).read_bytes());roots.extend(pathlib.Path(x['root']) for x in v['cases']) if 'cases' in v else roots.append(pathlib.Path(v['root']))
 with tarfile.open(fileobj=sys.stdout.buffer,mode='w|') as t:
  for r in roots:
   for folder,dirs,names in os.walk(r):
    folder=pathlib.Path(folder)
    if folder==r:dirs[:]=[x for x in dirs if x not in ('input','fixture-mount')]
    for name in sorted(names):
     if name.endswith('.tar.gz'):continue
     path=folder/name;assert path.is_file() and not path.is_symlink();t.add(path,arcname=r.name+'/'+path.relative_to(r).as_posix(),recursive=False)
'''.replace('PACKAGE',repr(package())).replace('MODE',repr(mode))
    if mode=='status':
        x=subprocess.run(d.SSH+['sudo -n /usr/bin/python3 -B -'],input=code.encode(),capture_output=True,timeout=45);assert x.returncode==0;print(x.stdout.decode());return
    d.call('collect',d.SSH+['sudo -n /usr/bin/python3 -B -'],data=code.encode());dest=OUT/'collection';dest.mkdir()
    with tarfile.open(fileobj=io.BytesIO((OUT/'collect.stdout.log').read_bytes())) as t:
        names=t.getmembers();assert len(names)==len({x.name for x in names})
        for x in names:
            assert x.isfile() and not x.name.startswith('/') and '..' not in x.name.split('/');path=dest/x.name;path.parent.mkdir(parents=True,exist_ok=True)
            with path.open('xb') as f:f.write(t.extractfile(x).read())
    print(json.dumps(dict(collected_files=len(names))))

if __name__=='__main__':
    assert len(sys.argv)==2;mode=sys.argv[1]
    if mode=='prepare':prepare()
    elif mode=='stage':stage()
    elif mode in ('faults','worker-controls','context-start','context-check'):phase(mode)
    elif mode=='matrix':matrix()
    elif mode in ('status','collect'):readback(mode)
    else:raise ValueError('fixed modes only')
