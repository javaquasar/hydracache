"""One-shot transport and read-only follow-up for approved finite maintenance."""
import hashlib
import json
from pathlib import Path
import re
import shlex
import subprocess
import sys
import tarfile

ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
OUT=ROOT/'target/chitchat-cohort-20261029'
sys.path.insert(0,str(ROOT/'tools/chitchat-host-cost-074'))
import dispatch as d
import remote
d.OUT=OUT

BASE_SHA='fcb8a53eba5ebc7e1364cbfa0d2bcb9e0ad1098f75b4e53dae56b7503c02ed38'


def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare():
    OUT.mkdir(exist_ok=False);payload=OUT/'payload';payload.mkdir()
    for name in ('coordinator.py','worker.py','plan.py','stage.py'):(payload/name).write_bytes((HERE/name).read_bytes())
    raw=(ROOT/'tools/get-owner-scheduled-controls-074/maintenance_d3.py').read_bytes()
    assert hashlib.sha256(raw).hexdigest()==BASE_SHA;(payload/'maintenance_base.py').write_bytes(raw)
    frame=json.loads((ROOT/'target/chitchat-host-cost-20261029/prepared/input/frame.json').read_bytes())
    for name in ('Cargo.toml','Cargo.lock','.cargo/config.toml','deny.toml','docs/testing/performance/0.74/qualification-manifest.toml',
        'scripts/ci/watchdog_linux_group.py','scripts/ci/watchdog_windows_job.py','tools/windows-workspace-preflight-074/owned.py'):
        assert sha(ROOT/name)==frame[name],name
    archive=OUT/'payload.tar.gz'
    with tarfile.open(archive,'x:gz') as tar:
        for path in sorted(payload.iterdir()):tar.add(path,arcname=path.name,recursive=False)
    remote.put(OUT/'prepared.json',dict(files={p.name:sha(p) for p in payload.iterdir()},archive_sha256=sha(archive),archive_bytes=archive.stat().st_size,
        operator_approved=True,approval='2026-10-29 limited maintenance user consent',protocol_sha256=sha(ROOT/'tools/chitchat-cost-074/protocol.json')))
    host=(ROOT/'tools/chitchat-host-cost-074/host.py').read_text()
    source="import json,os,stat\nns={'__name__':'read_only_host'}\nexec(compile("+repr(host)+",'reviewed_host.py','exec'),ns)\ns=ns['snapshot']();ns['admit'](s)\np='/var/lib/hydracache-performance/campaigns/.host-execution.lock'\nf=os.open(p,os.O_RDONLY|os.O_NOFOLLOW|os.O_CLOEXEC)\na=os.fstat(f);os.close(f)\nassert stat.S_ISREG(a.st_mode) and a.st_uid==0 and a.st_nlink==1\nprint(json.dumps(dict(host=s,fence_identity=[a.st_dev,a.st_ino],available_cpu=sorted(os.sched_getaffinity(0)),controllers=open('/sys/fs/cgroup/cgroup.controllers').read().strip(),read_only=True)))\n"
    (OUT/'preflight-producer.py').write_text(source,encoding='utf8',newline='\n')
    d.call('preflight',d.SSH+['sudo -n /usr/bin/python3 -B -'],data=source.encode())
    print(json.dumps(dict(prepared=True,archive_bytes=archive.stat().st_size,archive_sha256=sha(archive))))


def stage():
    assert not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT).strip(),'commit complete caller first'
    prepared=json.loads((OUT/'prepared.json').read_bytes());assert sha(OUT/'payload.tar.gz')==prepared['archive_sha256']
    assert {p.name:sha(p) for p in (OUT/'payload').iterdir()}==prepared['files']
    for name in ('coordinator.py','worker.py','plan.py','stage.py'):assert sha(OUT/'payload'/name)==sha(HERE/name),'current reviewed controller differs from payload'
    remote.put(OUT/'stage-once.json',dict(head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),retry=False))
    d.call('upload-directory',d.SSH+['umask 077; mktemp -d /tmp/hc074-chitchat-cohort-upload-20261029.XXXXXX'])
    upload=(OUT/'upload-directory.stdout.log').read_text().strip();assert re.fullmatch(r'/tmp/hc074-chitchat-cohort-upload-20261029\.[A-Za-z0-9]{6}',upload)
    d.call('upload',d.SCP+[str(OUT/'payload.tar.gz'),'hydracache-ax42:'+upload+'/payload.tar.gz'])
    script='''set -eu
test "$(id -u):$(id -g)" = 0:0
umask 077
upload=UPLOAD
test ! -L "$upload" && test "$(stat -c '%a:%u:%g' "$upload")" = 700:1000:1000
root=$(mktemp -d /tmp/hc074-chitchat-cohort-package-20261029.XXXXXX)
install -m 400 "$upload/payload.tar.gz" "$root/payload.tar.gz"
cd "$root"
printf '%s  payload.tar.gz\\n' 'ARCHIVE_SHA' | sha256sum --check
/usr/bin/python3 -B -c 'import tarfile; t=tarfile.open("payload.tar.gz"); a=t.getmembers(); assert len(a)==5 and len({x.name for x in a})==5 and all(x.isfile() and "/" not in x.name and x.name.endswith(".py") for x in a); t.extractall(".",members=a,filter="data")'
find "$root" -type f -exec chmod 400 {} +
printf 'ROOT=%s\\n' "$root"
'''.replace('UPLOAD',shlex.quote(upload)).replace('ARCHIVE_SHA',prepared['archive_sha256'])
    (OUT/'stage.sh').write_text(script,encoding='ascii',newline='\n');d.call('stage',d.SSH+['sudo -n /bin/sh -s'],data=script.encode())
    roots=re.findall(r'^ROOT=(/tmp/hc074-chitchat-cohort-package-20261029\.[A-Za-z0-9]{6})$',(OUT/'stage.stdout.log').read_text(),re.M);assert len(roots)==1
    remote.put(OUT/'remote.json',dict(root=roots[0],retry=False));print(json.dumps(dict(root=roots[0],staged=True)))


def package():
    root=json.loads((OUT/'remote.json').read_bytes())['root'];assert re.fullmatch(r'/tmp/hc074-chitchat-cohort-package-20261029\.[A-Za-z0-9]{6}',root);return root


def phase(mode):
    assert mode in ('faults','prepare','launch');root=package()
    if mode=='launch':
        plan=json.loads((OUT/'numeric-plan.json').read_bytes());before=json.loads((OUT/'preflight.stdout.log').read_bytes())
        assert len(plan['attempts'])==60 and plan['fence_identity']==before['fence_identity'] and plan['expected_host']==before['host']
        assert plan['operator_approved'] and plan['supervisor']=='hydracache-performance-supervisor-074.service'
        from_order=('B','P','Q','Q','P','B')*5
        assert [(v['mode'],v['variant']) for v in plan['attempts']]==[(m,v) for m in ('cpu','memory') for v in from_order]
        remote.put(OUT/'numerical-launch-once.json',dict(plan_sha256=sha(OUT/'numeric-plan.json'),retry=False,operator_approved=True))
    d.call('remote-'+mode,d.SSH+['sudo -n /usr/bin/python3 -B -I '+root+'/stage.py '+mode],bound=90)
    if mode=='prepare':
        value=json.loads((OUT/'remote-prepare.stdout.log').read_bytes());remote.put(OUT/'numerical.json',value)
        d.call('numeric-plan',d.SSH+['sudo -n cat -- '+value['root']+'/plan.json'])
        assert sha(OUT/'numeric-plan.stdout.log')==value['plan_sha256']
        (OUT/'numeric-plan.json').write_bytes((OUT/'numeric-plan.stdout.log').read_bytes())
    print((OUT/('remote-'+mode+'.stdout.log')).read_text())


def collect(mode):
    root=package();script='''import io,json,os,pathlib,sys,tarfile
p=pathlib.Path(PACKAGE)
if MODE=='status':
 r=json.loads((p/'numerical.json').read_bytes());root=pathlib.Path(r['root']);names=('cohort-complete.json','all-writers-stopped.json','restored.json','host-postflight.json')
 print(json.dumps(dict(root=str(root),files={n:(root/n).is_file() for n in names})))
else:
 roots=[p]
 if (p/'faults-once/result.json').is_file():roots += [pathlib.Path(x['root']) for x in json.loads((p/'faults-once/result.json').read_bytes())['cases']]
 if (p/'numerical.json').is_file():roots.append(pathlib.Path(json.loads((p/'numerical.json').read_bytes())['root']))
 with tarfile.open(fileobj=sys.stdout.buffer,mode='w|') as t:
  for root in roots:
   for folder,dirs,names in os.walk(root):
    folder=pathlib.Path(folder)
    if folder==root:dirs[:]=[n for n in dirs if n!='input']
    for name in sorted(names):
     if name.endswith('.tar.gz'):continue
     path=folder/name;assert path.is_file() and not path.is_symlink();t.add(path,arcname=root.name+'/'+path.relative_to(root).as_posix(),recursive=False)
'''.replace('PACKAGE',repr(root)).replace('MODE',repr(mode))
    if mode=='status':
        result=subprocess.run(d.SSH+['sudo -n /usr/bin/python3 -B -'],input=script.encode(),capture_output=True,timeout=45)
        assert result.returncode==0;print(result.stdout.decode());return
    d.call('collect',d.SSH+['sudo -n /usr/bin/python3 -B -'],data=script.encode())
    dest=OUT/'collection';dest.mkdir(exist_ok=False)
    import io
    with tarfile.open(fileobj=io.BytesIO((OUT/'collect.stdout.log').read_bytes())) as t:
        names=t.getmembers();assert len(names)==len({m.name for m in names})
        for item in names:
            assert item.isfile() and not item.name.startswith('/') and '..' not in item.name.split('/')
            path=dest/item.name;path.parent.mkdir(parents=True,exist_ok=True)
            with path.open('xb') as out:out.write(t.extractfile(item).read())
    print(json.dumps(dict(collected_files=len(names))))


if __name__=='__main__':
    assert len(sys.argv)==2
    action=sys.argv[1]
    if action=='prepare':prepare()
    elif action=='stage':stage()
    elif action in ('faults','numeric-prepare','launch'):phase('prepare' if action=='numeric-prepare' else action)
    elif action in ('status','collect'):collect(action)
    else:raise ValueError('fixed modes only')
