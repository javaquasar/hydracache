"""One exact original ignored nonroot private-bus matrix; never system manager."""
from pathlib import Path
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
import time

ROOT=Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
BASE=ROOT/'target/dbus-native-20261028/r'
NATIVE=Path('/home/artur/.cache/hydracache-074-dbus-native/20261028-a3')
OUT=BASE/'private-bus'
LEAF='dispatch_negotiated_private_bus_fault_matrix'


def need(ok,why):
    if not ok:raise ValueError(why)


def guard():
    proof=json.loads((BASE/'private-inputs.json').read_bytes())
    for name,digest in proof['files'].items():
        need(hashlib.sha256((ROOT/name).read_bytes()).hexdigest()==digest,'exact private-bus driver')
    need(hashlib.sha256((BASE/'freeze/result.json').read_bytes()).hexdigest()==proof['freeze_sha256'],
         'same typed source receipt')


def choose(catalog):
    need(len(catalog)==len(set(catalog))==1593,'complete actual image catalog')
    found=[name for name in catalog if name.endswith('::'+LEAF)]
    need(len(found)==1 and 'original_service::' not in found[0],'one original private-bus identity')
    return found[0]


def configured():
    guard()
    path=ROOT/'tools/dbus-linux-074/native.py'
    need(hashlib.sha256(path.read_bytes()).hexdigest()=='09ab4fb404b93af4ec5581b189d3e7fe54562fff14626bfabcbc07c11c9f51b1',
         'same original CI owner')
    spec=importlib.util.spec_from_file_location('native_private_bus_owner',path)
    owner=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    owner.BASE,owner.NATIVE,owner.OUT=BASE,NATIVE,OUT
    owner.prepared=lambda:BASE/'freeze/result.json'
    original=owner.command
    def command(label,review):
        need(label in ('fd-194-list','fd-194-test'),'first compiler/private matrix only')
        argv,env,artifact,target,folder,action=original(label,review)
        if action=='test':
            listed=json.loads((OUT/'fd-194-list/result.json').read_bytes())
            selected=choose(listed['inventory'])
            argv+=['--ignored','--exact',selected]
        return argv,env,artifact,target,folder,action
    owner.command=command
    return owner


def session():
    owner=configured()
    review=json.loads(owner.prepared().read_bytes())
    deadline=time.monotonic()+1100
    os.umask(0o077)
    result=dict(passed=False,server=False,system_bus=False,services=False,clocks_changed=False,
        tests_executed=False,original_send=False,release_admitted=False,phases=[],
        boot_before=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),clocks_before=owner.clocks())
    try:
        need(os.getuid()==os.getgid()==1000,'nonroot private bus only')
        need(Path('/usr/bin/dbus-daemon').is_file() and os.access('/usr/bin/dbus-daemon',os.X_OK),
             'preexisting private fixture binary; never install a package')
        owner.write(OUT,'daemon-input.json',dict(path='/usr/bin/dbus-daemon',
                    bytes=Path('/usr/bin/dbus-daemon').stat().st_size,
                    sha256=owner.sha(Path('/usr/bin/dbus-daemon').read_bytes())))
        qualified=json.loads((BASE/'units/native-result.json').read_bytes())
        need(qualified['passed'] and qualified['source_unchanged'] and qualified['boot_unchanged']
             and qualified['passes_per_compiler']==64,'complete actual paired compatibility first')
        built=json.loads((BASE/'units/fd-194-build/result.json').read_bytes())
        folder=OUT/'fd-194-build'
        folder.mkdir()
        owner.write(folder,'result.json',built)
        for action in ('list','test'):
            need(time.monotonic()<deadline,'original absolute private session origin')
            guard();owner.sources(review);owner.capacity()
            label='fd-194-'+action
            out=OUT/label
            out.mkdir()
            argv,env,artifact,target,folder,_=owner.command(label,review)
            owner.write(out,'intent.json',dict(argv=argv,environment=env,artifact=artifact,
                absolute_deadline=deadline,bound=1200,session_bound=1100,floor=4*1024**3,
                poll=5,retirement=5,retry=False,private_bus_only=True,matrix_inner_seconds=60))
            with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
                child=subprocess.Popen(['/usr/bin/python3','-B',str(Path(__file__)),'worker',label,str(deadline)],
                    stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,start_new_session=True)
                code=child.wait(timeout=max(0,deadline-time.monotonic())+10)
            execution=json.loads((out/'execution.json').read_bytes())
            need(execution['retired'] and not execution['errors'],'owned private fixture retirement')
            owner.sources(review)
            raw=(out/'stdout.log').read_text()
            value=dict(exit=code,artifact=artifact,source_unchanged=True)
            if action=='list' and code==0:
                catalog=sorted(line[:-6] for line in raw.splitlines() if line.endswith(': test'))
                value.update(inventory=catalog,selected=choose(catalog))
            if action=='test':
                result['tests_executed']=True
                summaries=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
                    r'(\d+) ignored; (\d+) measured; (\d+) filtered out;',raw,re.M)
                value.update(summary=summaries,case_records=[line for line in raw.splitlines()
                    if line.startswith('HC074-NEGOTIATED ')])
                need(len(summaries)==1 and summaries[0][0] in ('ok','FAILED')
                     and summaries[0][3:] == ('0','0','1592')
                     and int(summaries[0][1])+int(summaries[0][2])==1,'one actual matrix verdict')
            owner.write(out,'result.json',value)
            need(owner.sha(Path(artifact['path']).read_bytes())==artifact['sha256'],'same actual native image')
            result['phases'].append(dict(label=label,exit=code))
            print(json.dumps(dict(phase=label,exit=code)),flush=True)
            need(code==0,'first actual private matrix refusal: '+label)
        result['passed']=True
    except BaseException as error:
        result['first_refusal']=dict(type=type(error).__name__,reason=str(error))
    finally:
        try:owner.sources(review);guard();result['source_unchanged']=True
        except BaseException as error:result.update(passed=False,source_unchanged=False,source_refusal=str(error))
        result.update(boot_after=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),clocks_after=owner.clocks())
        result['boot_unchanged']=result['boot_before']==result['boot_after']
        owner.write(OUT,'native-result.json',result)
    raise SystemExit(0 if result['passed'] and result['boot_unchanged'] else 1)


if __name__=='__main__':
    if len(sys.argv)==4 and sys.argv[1]=='worker':
        configured().worker(sys.argv[2],float(sys.argv[3]));raise SystemExit('worker exits')
    need(sys.argv[1:]==['session'],'fixed single private-bus session')
    session()
