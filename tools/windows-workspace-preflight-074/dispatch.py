"""Fixed Windows Cargo commands. Default preflight only; no filter/retry/limit options."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

import accounting
import custody
import owned
import prepare as p

CONTROL=p.ROOT/'target/security-review-20261028/controls'


def controls(root=p.ROOT):
    folder=root/CONTROL.relative_to(p.ROOT)
    intent=json.loads((folder/'intent.json').read_bytes())
    result=json.loads((folder/'result.json').read_bytes())
    p.require(result['exit']==0 and result['source_unchanged'] and result['head_unchanged'],'dispatcher controls not green')
    expected=[sys.executable,'-B','-m','unittest','discover','-s','tools/windows-workspace-preflight-074','-p','test_*.py','-v']
    p.require(intent['argv']==expected,'controls selection drift')
    for name,digest in custody.helpers(root).items():
        p.require(intent['source'][name]==result['source_after'][name]==digest,'stale controlled helper: '+name)


def execute(out,mode,*,run=False,root=p.ROOT):
    p.require(out.is_absolute() and out.resolve()==out and out.is_relative_to(root/'target'),'exclusive output under original target')
    out.mkdir(parents=True,exist_ok=False)
    spec=owned.specification(mode)
    p.write(out,'intent.json',dict(mode=mode,specification=spec,execute=run,retry=False,
        system_settings=False,foreign_process_stops=False,server=False,product_load=False,qualification=False))
    # Streams exist before every preflight; preflight failures get a separate receipt.
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        process=None
        try:
            p.require(os.name=='nt','Windows-only dispatcher')
            controls(root)
            before=custody.frame(root)
            p.write(out,'frame-before.json',before)
            free=shutil.disk_usage(root).free
            p.require(free>=spec['start_free_bytes'],'original initial capacity prerequisite')
            p.write(out,'capacity-before.json',dict(free_bytes=free,initial_bytes=spec['start_free_bytes'],floor_bytes=spec['floor_bytes']))
            if not run:
                p.write(out,'result.json',dict(preflight_complete=True,gate_dispatched=False,gate_pass=False,release_admitted=False))
                return 0
            identity=p.sha(json.dumps(dict(sources=before['sources'],specification=spec,
                versions=before['versions'],environment=before['environment']),sort_keys=True).encode())
            claims=root/'target/windows-fixed-cargo-20261027/claims'
            claims.mkdir(parents=True,exist_ok=True)
            # A failed/cancelled/preflight-refused execution claim stays consumed.
            p.write(claims,identity+'-'+mode+'.json',dict(head=before['head'],mode=mode,output=str(out),retry=False))
            # Startup/checking time is INCLUDED, not added to the original command's budget.
            deadline=time.monotonic()+spec['bound_seconds']
            request=dict(mode=mode,specification=spec,frame=before,deadline=deadline)
            p.write(out,'request.json',request)
            request_sha=p.sha((out/'request.json').read_bytes())
            command=[sys.executable,'-B',str(root/'tools/windows-workspace-preflight-074/worker.py'),str(out),request_sha,mode]
            process=subprocess.Popen(command,cwd=root,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,
                env=dict(os.environ,**spec['environment']))
            p.write(out,'worker.json',dict(argv=command,pid=process.pid,deadline=deadline,cleanup_seconds=5,request_sha256=request_sha))
            worker_lost=False
            try:
                process.wait(timeout=max(0,deadline+5-time.monotonic()))
            except subprocess.TimeoutExpired:
                # Original Popen handle only. Job KILL_ON_CLOSE contains Cargo; missing
                # retirement receipt is runner loss, never evidence of a successful gate.
                process.kill()
                process.wait(timeout=5)
                worker_lost=True
            stdout.flush();stderr.flush()
            after=custody.frame(root)
            p.write(out,'frame-after.json',after)
            custody.equivalent(before,after)
            p.require(not worker_lost,'original worker exceeded command plus cleanup bounds')
            receipt=json.loads((out/'controller.json').read_bytes())
            p.require(receipt['request_sha256']==request_sha==p.sha((out/'request.json').read_bytes()) and
                receipt['mode']==mode and receipt['specification']==spec and receipt['worker_pid']==process.pid,
                'original worker receipt identity')
            execution=receipt['execution']
            p.require(execution['exit']==process.returncode and execution['bound_seconds']==spec['bound_seconds'] and
                execution['cleanup_seconds']==5 and receipt['original_job_sha256']==owned.JOB_SHA,
                'original Job receipt/exit/budget drift')
            result=accounting.account(mode,(out/'stdout.log').read_text(encoding='utf-8'),
                (out/'stderr.log').read_text(encoding='utf-8'),execution,before['targets'])
            p.write(out,'accounting.json',result)
            passed=(execution['classification']=='success' and not execution['errors'] and
                    result.get('package_pass' if mode=='package' else 'workspace_pass',False))
            p.write(out,'result.json',dict(mode=mode,gate_dispatched=True,gate_pass=passed,
                package_pass=passed if mode=='package' else False,workspace_pass=passed if mode=='workspace' else False,
                source_refs_configuration_environment_unchanged=True,release_admitted=False))
            return int(not passed)
        except BaseException as error:
            if process is not None and process.poll() is None:
                process.kill()  # Only this call's original worker handle, never another process.
                process.wait(timeout=5)
            if not (out/'accounting.json').exists():
                p.write(out,'accounting.json',dict(complete=False,not_assessed=True,reason=str(error),
                    package_pass=False,workspace_pass=False,raw_streams_retained=True))
            p.write(out,'refusal.json',dict(type=type(error).__name__,reason=str(error),
                gate_dispatched=process is not None,gate_pass=False,release_admitted=False))
            raise


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('out',type=Path)
    parser.add_argument('--mode',choices=('package','workspace'),required=True)
    parser.add_argument('--execute',action='store_true')
    args=parser.parse_args()
    raise SystemExit(execute(args.out,args.mode,run=args.execute))
