"""Whole real worker smoke under an owned stand-in, never the stable service."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import time

sys.dont_write_bytecode=True


def load(path,name):
    spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m


def run(package):
    stage=load(package/'stage.py','hc074_checked_stage');before=stage.original()
    record=package/'worker-controls-once';record.mkdir()
    root,_,_=stage.build(package,True,'healthy')
    p=json.loads((root/'plan.json').read_bytes());(root/'plan.json').rename(root/'unused-synthetic-plan.json')
    p['attempts']=[dict(variant=v,mode='control',argv=['/usr/bin/python3','-B','-I',str(root/'worker.py'),str(root),str(i),'@PLAN_SHA@','@FENCE_FD@']) for i,v in enumerate(('B','P','Q'))]
    assert p['synthetic'] and p['supervisor']==root.name+'-standin.service'
    stage.put(root/'plan.json',p);(root/'plan.json').chmod(0o400);pin=stage.sha(root/'plan.json')
    plan=load(root/'plan.py','hc074_standin_worker_plan');argv=plan.launch(root,pin,True)
    stage.put(root/'worker-launch-intent.json',dict(argv=argv,retry=False,controls_only=True));stage.command(argv)
    at=time.monotonic()
    while not (root/'restored.json').exists() and time.monotonic()-at<20:time.sleep(.05)
    passed=(root/'restored.json').exists() and (root/'cohort-complete.json').exists();bodies=[]
    try:
        assert passed
        c=load(root/'coordinator.py','hc074_whole_worker_control')
        for i,attempt in enumerate(p['attempts']):
            body=c.validate_receipt(root,i,attempt,p)
            execution=json.loads((root/(str(i)+'.execution.json')).read_bytes())
            assert execution['exit']==0 and execution['retired'] and not execution['errors']
            bodies.append(dict(variant=body['variant'],pid=body['pid'],start_ticks=body['start_ticks'],semantic_rows=len(body['rows'])))
    except BaseException as error:
        passed=False;stage.put(record/'refusal.json',dict(type=type(error).__name__,reason=str(error)))
    journal=subprocess.run(['/usr/bin/journalctl','--no-pager','-u',root.name+'.service','-n','80','-o','short-monotonic'],capture_output=True,timeout=10)
    (root/'journal.log').write_bytes(journal.stdout)
    for unit in (root.name+'.service',root.name+'-standin.service'):
        stopped=subprocess.run(['/usr/bin/systemctl','stop',unit],capture_output=True,timeout=15)
        state=stage.command(['/usr/bin/systemctl','show',unit,'-p','MainPID','-p','LoadState'])
        assert 'MainPID=0' in state or 'LoadState=not-found' in state
        assert stopped.returncode==0 or stopped.returncode==5 and 'LoadState=not-found' in state
    after=stage.original();result=dict(complete=passed,root=str(root),bodies=bodies,stable_supervisor_unchanged=before==after,CPU_qualified=False,RSS_qualified=False)
    stage.put(record/'result.json',result);print(json.dumps(result))


if __name__=='__main__':
    assert len(sys.argv)==3
    package=Path(sys.argv[1]);expected=sys.argv[2]
    assert package.parent==Path('/tmp') and package.name.startswith('hc074-chitchat-cohort-package-20261029.')
    import hashlib
    assert hashlib.sha256((package/'stage.py').read_bytes()).hexdigest()==expected
    run(package)
