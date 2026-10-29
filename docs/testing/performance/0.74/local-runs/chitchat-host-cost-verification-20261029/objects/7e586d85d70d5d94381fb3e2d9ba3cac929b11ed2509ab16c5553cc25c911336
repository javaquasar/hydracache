"""Freeze one reviewed payload, then stage once; collection never redispatches."""
import json
import re
import shlex
import subprocess
import tarfile
import sys
from pathlib import Path
import prepare
import remote
import review

ROOT=prepare.ROOT;OUT=prepare.OUT;HERE=prepare.HERE
SSH=['ssh','-T','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=15',
    '-o','ServerAliveInterval=5','-o','ServerAliveCountMax=2','hydracache-ax42']
SCP=['scp','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=15']


def call(label,argv,data=None,bound=45):
    remote.put(OUT/(label+'-intent.json'),dict(argv=argv,bound_seconds=bound,retry=False))
    with (OUT/(label+'.stdout.log')).open('xb') as stdout,(OUT/(label+'.stderr.log')).open('xb') as stderr:
        value=dict(complete=False,retry=False)
        try:
            child=subprocess.run(argv,input=data,stdout=stdout,stderr=stderr,cwd=ROOT,timeout=bound)
            value.update(exit=child.returncode,complete=child.returncode==0)
        except BaseException as error:value['refusal']=dict(type=type(error).__name__,reason=str(error))
        remote.put(OUT/(label+'-result.json'),value)
    prepare.need(value['complete'],'first '+label+' refusal retained; inspect same attempt, never redispatch')


def freeze():
    result=review.review();root=OUT/'prepared/input'
    files=remote.inventory(root)
    remote.put(root/'inputs.json',dict(files=files,expected_host=json.loads((OUT/'preflight.stdout.log').read_bytes()),
        protocol=json.loads((root/'protocol.json').read_bytes()),controls_only=True,exclusive_host=False))
    archive=OUT/'payload.tar.gz'
    with tarfile.open(archive,'x:gz') as tar:
        for path in sorted(root.rglob('*')):
            if path.is_file():tar.add(path,arcname='input/'+path.relative_to(root).as_posix(),recursive=False)
    producer={p.name:remote.sha(p) for p in HERE.iterdir() if p.is_file()}
    remote.put(OUT/'frozen.json',dict(review=result,files=files,producer=producer,manifest_sha256=remote.sha(root/'inputs.json'),
        archive_sha256=remote.sha(archive),archive_bytes=archive.stat().st_size,tar_files=len(files)+1,retry=False))
    print(json.dumps(dict(frozen=True,files=len(files),archive_bytes=archive.stat().st_size,archive_sha256=remote.sha(archive))))


def dispatch():
    prepare.need(not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT).strip(),'committed clean caller before remote dispatch')
    frozen=json.loads((OUT/'frozen.json').read_bytes());root=OUT/'prepared/input';review.review()
    prepare.need(remote.inventory(root)==frozen['files']|{'inputs.json':frozen['manifest_sha256']},'prepared input drift')
    prepare.need({p.name:remote.sha(p) for p in HERE.iterdir() if p.is_file()}==frozen['producer'],'reviewed complete producer drift')
    remote.put(OUT/'dispatch-once.json',dict(head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        retry=False,controls_only=True,manifest_sha256=frozen['manifest_sha256']))
    call('dispatch-preflight',SSH+['sudo -n /usr/bin/python3 -B -'],data=(HERE/'host.py').read_bytes())
    before=json.loads((OUT/'dispatch-preflight.stdout.log').read_bytes());host_expected=json.loads((root/'inputs.json').read_bytes())['expected_host']
    prepare.need(before==host_expected,'original host changed before dispatch')
    call('upload-directory',SSH+['umask 077; mktemp -d /tmp/hc074-chitchat-control-upload-20261029.XXXXXX'])
    upload=(OUT/'upload-directory.stdout.log').read_text().strip()
    prepare.need(re.fullmatch(r'/tmp/hc074-chitchat-control-upload-20261029\.[A-Za-z0-9]{6}',upload),'exact upload directory')
    call('upload',SCP+[str(OUT/'payload.tar.gz'),'hydracache-ax42:'+upload+'/payload.tar.gz'],bound=90)
    script='''set -eu
test "$(id -u):$(id -g)" = 0:0
umask 077
upload=UPLOAD
test ! -L "$upload"
test "$(stat -c '%a:%u:%g' "$upload")" = 700:1000:1000
test -f "$upload/payload.tar.gz" && test ! -L "$upload/payload.tar.gz"
root=$(mktemp -d /tmp/hc074-chitchat-control-root-20261029.XXXXXX)
install -m 400 "$upload/payload.tar.gz" "$root/payload.tar.gz"
cd "$root"
printf '%s  payload.tar.gz\\n' 'ARCHIVE_SHA' | sha256sum --check
/usr/bin/python3 -B -c 'import tarfile; t=tarfile.open("payload.tar.gz"); a=t.getmembers(); assert len(a)==COUNT and len({x.name for x in a})==len(a) and all(x.isfile() and x.name.startswith("input/") and ".." not in x.name.split("/") for x in a); t.extractall(".",members=a,filter="data")'
find "$root/input" -type f -exec chmod 400 {} +
chmod 500 "$root/input/images/B" "$root/input/images/P" "$root/input/images/Q"
printf '%s  input/remote.py\\n' 'RUNNER_SHA' | sha256sum --check
printf '%s  input/inputs.json\\n' 'MANIFEST_SHA' | sha256sum --check
printf 'ROOT=%s\\n' "$root"
nohup /usr/bin/env -i PATH=/usr/bin:/bin LANG=C.UTF-8 /usr/bin/python3 -B -I "$root/input/remote.py" owner MANIFEST_SHA >"$root/live.stdout.log" 2>"$root/live.stderr.log" </dev/null &
printf 'OWNER_PID=%s\\n' "$!"
'''
    for before,after in (('UPLOAD',shlex.quote(upload)),('ARCHIVE_SHA',frozen['archive_sha256']),('COUNT',str(frozen['tar_files'])),
        ('RUNNER_SHA',remote.sha(root/'remote.py')),('MANIFEST_SHA',frozen['manifest_sha256'])):script=script.replace(before,after)
    (OUT/'stage.sh').write_text(script,encoding='ascii',newline='\n')
    call('stage',SSH+['sudo -n /bin/sh -s'],data=script.encode())
    raw=(OUT/'stage.stdout.log').read_text();matches=re.findall(r'^ROOT=(/tmp/hc074-chitchat-control-root-20261029\.[A-Za-z0-9]{6})$',raw,re.M)
    prepare.need(len(matches)==1,'read same stage output; never stage twice')
    remote.put(OUT/'remote.json',dict(root=matches[0],manifest_sha256=frozen['manifest_sha256'],retry=False))
    print(json.dumps(dict(root=matches[0],controls_only=True,dispatched_once=True)))


if __name__=='__main__':
    prepare.need(sys.argv[1:] in (['freeze'],['dispatch']),'fixed freeze/one dispatch only')
    (freeze if sys.argv[1]=='freeze' else dispatch)()
