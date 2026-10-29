"""One autonomous dedicated-host CONTROL session; no numerical dispatcher."""
import hashlib
import ast
import importlib.util
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import types

sys.dont_write_bytecode=True
FLOOR=4*1024**3


def need(ok,reason):
    if not ok:raise ValueError(reason)


def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()


def put(path,value):
    with path.open('x',encoding='utf8',newline='\n') as out:
        json.dump(value,out,indent=2,sort_keys=True);out.write('\n');out.flush();os.fsync(out.fileno())


def load(path,name):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module


def inventory(root):
    result={}
    for folder,dirs,names in os.walk(root,followlinks=False):
        for name in dirs+names:need(not (Path(folder)/name).is_symlink(),'no linked input')
        for name in names:
            path=Path(folder)/name;need(path.is_file(),'regular input');result[path.relative_to(root).as_posix()]=sha(path)
    return result


def verify(root,manifest):
    actual=inventory(root/'input');actual.pop('inputs.json',None)
    need(actual==manifest['files'],'fresh complete source/registry/image/config input inventory')
    for name in manifest['files']:
        state=(root/'input'/name).stat()
        need(state.st_uid==state.st_gid==0 and state.st_mode&0o222==0,'root-owned immutable input '+name)
    state=(root/'input/inputs.json').stat()
    need(state.st_uid==state.st_gid==0 and state.st_mode&0o222==0,'immutable manifest')


def capacity():
    if shutil.disk_usage('/tmp').free<FLOOR:raise CapacityError('original physical four-GiB floor')


class CapacityError(RuntimeError):pass


def fixed_argv(root,variant):
    need(variant in ('B','P','Q'),'fixed control variant')
    return ['/usr/bin/env','-i','PATH=/usr/bin:/bin','LANG=C.UTF-8','HOME=/nonexistent',str(root/'input/images'/variant),'control']


def context(root,expected):
    need(root.parent==Path('/tmp') and root.name.startswith('hc074-chitchat-control-root-20261029.'),'exact private temporary scope')
    state=root.stat();need(not root.is_symlink() and state.st_uid==state.st_gid==0 and state.st_mode&0o7777==0o700,'private root owner/mode')
    need(sha(root/'input/inputs.json')==expected,'externally pinned manifest')
    manifest=json.loads((root/'input/inputs.json').read_bytes());verify(root,manifest)
    need(sha(root/'input/remote.py')==sha(Path(__file__)),'executing reviewed original runner')
    host=load(root/'input/host.py','hc074_control_host')
    group=load(root/'input/group.py','hc074_control_group')
    namespace=dict(time=time,math=math,CapacityError=CapacityError,prepare=types.SimpleNamespace(require=need))
    # Exact current, byte-pinned reviewed helper function, not an archived producer.
    exec(compile((root/'input/owned_function.py').read_bytes(),'owned_function.py','exec'),namespace)
    return manifest,host,group,namespace['_owned']


def stable(root,manifest,host):
    verify(root,manifest);capacity();state=host.snapshot();host.admit(state)
    need(state==manifest['expected_host'],'fresh original host/boot/supervisor/campaign/clock/mount frame')
    return state


def validation_constants(raw):
    result={}
    for node in ast.parse(raw).body:
        if isinstance(node,ast.Assign) and len(node.targets)==1 and isinstance(node.targets[0],ast.Name) and node.targets[0].id in ('FIELDS','HEAP'):
            name=node.targets[0].id;need(name not in result,'unique validator global')
            value=ast.literal_eval(node.value)
            need(type(value) is tuple and value and all(type(v) is str for v in value) and len(value)==len(set(value)),'literal original field names')
            result[name]=value
    need(set(result)=={'FIELDS','HEAP'},'complete original validator global closure');return result


def validator(root):
    namespace=dict(json=json,c=types.SimpleNamespace(BASE=root/'input/semantics',need=need,
        protocol=lambda:json.loads((root/'input/protocol.json').read_bytes())))
    namespace.update(validation_constants((root/'input/source/tools/chitchat-cost-074/analyze.py').read_bytes()))
    exec(compile((root/'input/validate_function.py').read_bytes(),'validate_function.py','exec'),namespace)
    return namespace['validate']


def session(root,expected):
    result=dict(complete=False,CPU_qualified=False,RSS_qualified=False,exclusive_host=False,cohort_dispatched=False)
    deadline=time.monotonic()+1100
    try:
        manifest,host,_,_=context(root,expected)
        before=stable(root,manifest,host);put(root/'session-preflight.json',before)
        for variant in ('B','P','Q'):
            stable(root,manifest,host);folder=root/('control-'+variant);folder.mkdir(exist_ok=False)
            put(folder/'intent.json',dict(argv=fixed_argv(root,variant),deadline=deadline,bound=1200,retry=False,
                image_sha256=manifest['files']['images/'+variant],manifest_sha256=expected))
            with (folder/'stdout.log').open('xb') as stdout,(folder/'stderr.log').open('xb') as stderr:
                child=subprocess.run([sys.executable,'-B','-I',str(root/'input/remote.py'),'command',variant,str(deadline),expected],
                    cwd=root,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=max(0,deadline-time.monotonic())+10)
            owner=json.loads((folder/'execution.json').read_bytes())
            need(child.returncode==owner['exit']==0 and owner['retired'] and not owner['errors'],'complete first original owned control')
            stable(root,manifest,host)
            raw=(folder/'stdout.log').read_text();rows=[line.removeprefix('HC074_CLEAN_COST:') for line in raw.splitlines() if line.startswith('HC074_CLEAN_COST:')]
            need(len(rows)==1,'one complete actual body');body=json.loads(rows[0]);put(folder/'raw-body.json',body)
            validator(root)(body,variant,'control')
            need(body['boot_id']==manifest['expected_host']['boot'],'actual original process boot')
            put(folder/'result.json',dict(complete=True,control_pass=True,CPU_qualified=False,RSS_qualified=False))
        need(time.monotonic()<deadline,'original full 1100-second session')
        put(root/'session-postflight.json',stable(root,manifest,host));result['complete']=True
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error))
    put(root/'session-result.json',result);return 0 if result['complete'] else 1


def owner(root,expected):
    put(root/'started.json',dict(retry=False,controls_only=True,exclusive_host=False,cohort_dispatched=False,
        service_changes=False,clock_changes=False,installation=False,product_workload=False,manifest_sha256=expected,
        owner_pid=os.getpid(),owner_start_ticks=Path('/proc/self/stat').read_text().rsplit(') ',1)[1].split()[19],
        boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip()))
    result=dict(complete=False,CPU_qualified=False,RSS_qualified=False,root_adopted=False,cohort_dispatched=False,exclusive_host=False)
    before=None
    try:
        manifest,host,group,owned=context(root,expected)
        before=stable(root,manifest,host);put(root/'preflight.json',before)
        execution=owned(group.LinuxGroup(),[sys.executable,'-B','-I',str(root/'input/remote.py'),'session',expected],str(root),capacity,bound=1200)
        put(root/'execution.json',execution)
        done=json.loads((root/'session-result.json').read_bytes())
        result['complete']=execution['exit']==0 and execution['retired'] and not execution['errors'] and done['complete']
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error))
    finally:
        try:
            manifest,host,_,_=context(root,expected);after=stable(root,manifest,host);put(root/'postflight.json',after)
            result['postflight_passed']=before is not None and after==before
            need(result['postflight_passed'],'unchanged original host after retirement')
        except BaseException as error:
            result.update(complete=False,postflight_passed=False,postflight_refusal=dict(type=type(error).__name__,reason=str(error)))
        put(root/'result.json',result)
    return 0 if result['complete'] else 1


def main():
    root=Path(__file__).resolve().parent.parent
    if len(sys.argv)==3 and sys.argv[1] in ('owner','session'):
        return (owner if sys.argv[1]=='owner' else session)(root,sys.argv[2])
    if len(sys.argv)==5 and sys.argv[1]=='command':
        variant,deadline,expected=sys.argv[2:];manifest,host,group,owned=context(root,expected)
        stable(root,manifest,host)
        execution=owned(group.LinuxGroup(),fixed_argv(root,variant),str(root),capacity,bound=1200,deadline=float(deadline))
        put(root/('control-'+variant)/'execution.json',execution);return execution['exit']
    raise ValueError('fixed owner/session/control command modes only; no numerical dispatch')


if __name__=='__main__':raise SystemExit(main())
