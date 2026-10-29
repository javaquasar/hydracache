"""One changed-carrier context; reuse exact data by copy, never old results."""
import io
import json
import re
import shlex
import subprocess
import sys
import tarfile
from pathlib import Path
import dispatch as d
import prepare
import remote

FIRST=prepare.OUT;OUT=prepare.ROOT/'target/chitchat-host-cost-20261029-r1'


def review():
    frozen=json.loads((FIRST/'frozen.json').read_bytes());base=FIRST/'prepared/input'
    prepare.need(remote.inventory(base)==frozen['files']|{'inputs.json':frozen['manifest_sha256']},'unchanged first input graph')
    first=json.loads((FIRST/'collection/result.json').read_bytes());session=json.loads((FIRST/'collection/session-result.json').read_bytes())
    prepare.need(first['postflight_passed'] and not first['complete'] and session['first_refusal']['reason']=="name 'FIELDS' is not defined",'specific first carrier failure retained')
    owner=json.loads((FIRST/'collection/execution.json').read_bytes());prepare.need(owner['retired'] and not owner['errors'],'first owner retired')
    raw=(base/'remote.py').read_text();new=(prepare.HERE/'remote.py').read_text()
    # No executable/work/clock/threshold mutation; precisely one literal-data closure.
    new=new.replace('import ast\n','',1)
    start=new.index('def validation_constants(');end=new.index('\n\ndef validator(',start)
    new=new[:start]+new[end+2:]
    line="    namespace.update(validation_constants((root/'input/source/tools/chitchat-cost-074/analyze.py').read_bytes()))\n"
    prepare.need(new.count(line)==1 and new.replace(line,'')==raw,'only validator global binding changes')
    prepare.need(remote.validation_constants((base/'source/tools/chitchat-cost-074/analyze.py').read_bytes())==remote.validation_constants((prepare.ROOT/'tools/chitchat-cost-074/analyze.py').read_bytes()),'same original field sets')
    return frozen,base


def freeze():
    frozen,base=review();OUT.mkdir(exist_ok=False);patch=OUT/'patch';(patch/'input').mkdir(parents=True)
    (patch/'input/remote.py').write_bytes((prepare.HERE/'remote.py').read_bytes())
    manifest=json.loads((base/'inputs.json').read_bytes());manifest['files']['remote.py']=remote.sha(patch/'input/remote.py')
    manifest.update(revision='complete-validator-global-closure',parent_manifest_sha256=frozen['manifest_sha256'],
        parent_remote_root=json.loads((FIRST/'remote.json').read_bytes())['root'])
    remote.put(patch/'input/inputs.json',manifest)
    archive=OUT/'patch.tar.gz'
    with tarfile.open(archive,'x:gz') as tar:
        for p in sorted(patch.rglob('*')):
            if p.is_file():tar.add(p,arcname=p.relative_to(patch).as_posix(),recursive=False)
    remote.put(OUT/'frozen.json',dict(files=manifest['files'],manifest_sha256=remote.sha(patch/'input/inputs.json'),
        archive_sha256=remote.sha(archive),archive_bytes=archive.stat().st_size,parent_manifest_sha256=frozen['manifest_sha256'],
        parent_root=manifest['parent_remote_root'],producer={p.name:remote.sha(p) for p in prepare.HERE.iterdir() if p.is_file()},
        controls_only=True,protocol_changed=False,images_changed=False,retry=False))
    print(json.dumps(dict(frozen=True,archive_bytes=archive.stat().st_size,archive_sha256=remote.sha(archive),images_changed=False)))


def dispatch():
    prepare.need(not subprocess.check_output(['git','status','--porcelain'],cwd=prepare.ROOT).strip(),'committed clean revised carrier')
    review();frozen=json.loads((OUT/'frozen.json').read_bytes());d.OUT=OUT
    prepare.need({p.name:remote.sha(p) for p in prepare.HERE.iterdir() if p.is_file()}==frozen['producer'],'reviewed revised producer closure')
    remote.put(OUT/'dispatch-once.json',dict(head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=prepare.ROOT,text=True).strip(),retry=False,controls_only=True))
    d.call('preflight',d.SSH+['sudo -n /usr/bin/python3 -B -'],data=(prepare.HERE/'host.py').read_bytes())
    prepare.need(json.loads((OUT/'preflight.stdout.log').read_bytes())==json.loads((FIRST/'preflight.stdout.log').read_bytes()),'unchanged original host before revised control')
    d.call('upload-directory',d.SSH+['umask 077; mktemp -d /tmp/hc074-chitchat-control-upload-20261029.XXXXXX'])
    upload=(OUT/'upload-directory.stdout.log').read_text().strip()
    prepare.need(re.fullmatch(r'/tmp/hc074-chitchat-control-upload-20261029\.[A-Za-z0-9]{6}',upload),'exact patch upload')
    d.call('upload',d.SCP+[str(OUT/'patch.tar.gz'),'hydracache-ax42:'+upload+'/patch.tar.gz'])
    script='''set -eu
test "$(id -u):$(id -g)" = 0:0
umask 077
upload=UPLOAD
old=PARENT_ROOT
test ! -L "$upload" && test "$(stat -c '%a:%u:%g' "$upload")" = 700:1000:1000
test ! -L "$old" && test "$(stat -c '%a:%u:%g' "$old")" = 700:0:0
printf '%s  %s/input/inputs.json\\n' 'PARENT_SHA' "$old" | sha256sum --check
/usr/bin/python3 -B -c 'import json,sys; from pathlib import Path; r=Path(sys.argv[1]); a=json.loads((r/"result.json").read_bytes()); e=json.loads((r/"execution.json").read_bytes()); assert not a["complete"] and a["postflight_passed"] and e["retired"] and not e["errors"]' "$old"
root=$(mktemp -d /tmp/hc074-chitchat-control-root-20261029.XXXXXX)
cp -a "$old/input" "$root/input"
install -m 400 "$upload/patch.tar.gz" "$root/patch.tar.gz"
cd "$root"
printf '%s  patch.tar.gz\\n' 'PATCH_SHA' | sha256sum --check
/usr/bin/python3 -B -c 'import os,sys,tarfile; from pathlib import Path; old=Path(sys.argv[1]); a=tarfile.open("patch.tar.gz"); m=a.getmembers(); assert {x.name for x in m}=={"input/remote.py","input/inputs.json"} and len(m)==2 and all(x.isfile() for x in m); assert all(os.stat(x.name).st_ino!=os.stat(old/x.name).st_ino for x in m); a.extractall(".",members=m,filter="data")' "$old"
chmod 400 "$root/input/remote.py" "$root/input/inputs.json"
printf '%s  input/inputs.json\\n' 'MANIFEST_SHA' | sha256sum --check
printf 'ROOT=%s\\n' "$root"
nohup /usr/bin/env -i PATH=/usr/bin:/bin LANG=C.UTF-8 /usr/bin/python3 -B -I "$root/input/remote.py" owner MANIFEST_SHA >"$root/live.stdout.log" 2>"$root/live.stderr.log" </dev/null &
printf 'OWNER_PID=%s\\n' "$!"
'''
    for old,new in (('UPLOAD',shlex.quote(upload)),('PARENT_ROOT',shlex.quote(frozen['parent_root'])),('PARENT_SHA',frozen['parent_manifest_sha256']),
        ('PATCH_SHA',frozen['archive_sha256']),('MANIFEST_SHA',frozen['manifest_sha256'])):script=script.replace(old,new)
    (OUT/'stage.sh').write_text(script,encoding='ascii',newline='\n');d.call('stage',d.SSH+['sudo -n /bin/sh -s'],data=script.encode())
    matches=re.findall(r'^ROOT=(/tmp/hc074-chitchat-control-root-20261029\.[A-Za-z0-9]{6})$',(OUT/'stage.stdout.log').read_text(),re.M)
    prepare.need(len(matches)==1,'inspect recorded stage; never repeat it')
    remote.put(OUT/'remote.json',dict(root=matches[0],manifest_sha256=frozen['manifest_sha256'],retry=False));print(json.dumps(dict(root=matches[0],dispatched_once=True)))


if __name__=='__main__':
    prepare.need(sys.argv[1:] in (['review'],['freeze'],['dispatch'],['status'],['collect']),'fixed revised control modes')
    if sys.argv[1]=='review':review();print(json.dumps(dict(complete=True,only_validator_globals_changed=True)))
    elif sys.argv[1] in ('freeze','dispatch'):(freeze if sys.argv[1]=='freeze' else dispatch)()
    else:
        import collect
        d.OUT=OUT;(collect.status if sys.argv[1]=='status' else collect.collect)()
