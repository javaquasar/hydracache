"""Fixed unit and 60-process plan; this module has no service effect."""
import hashlib
import json
from pathlib import Path

ORDER=('B','P','Q','Q','P','B')*5
ENV={'PATH':'/usr/bin:/bin','LANG':'C.UTF-8','HOME':'/nonexistent'}


def need(ok,why):
    if not ok:raise ValueError(why)


def make(root,state,files,fence,identity,synthetic=False,case=None,argv=None):
    need(root.parent==Path('/tmp') and root.name.startswith('hc074-chitchat-cost-20261029.'),'fixed root family')
    fields=dict(line.split('=',1) for line in state['supervisor'].splitlines())
    fields.pop('InvocationID',None)
    attempts=[]
    if synthetic:
        need(case in ('healthy','deadline','owner-kill','frame-denial','frame-denial-owner-kill','namespace-owner-kill') and argv,'fixed fault case');attempts=[dict(argv=argv)]
    else:
        for mode in ('cpu','memory'):
            for variant in ORDER:
                seq=len(attempts);attempts.append(dict(variant=variant,mode=mode,
                    argv=['/usr/bin/python3','-B','-I',str(root/'worker.py'),str(root),str(seq),'@PLAN_SHA@','@FENCE_FD@']))
    return dict(schema_version=1,unit=root.name+'.service',supervisor=root.name+'-standin.service' if synthetic else 'hydracache-performance-supervisor-074.service',
        supervisor_before=fields,synthetic=synthetic,synthetic_deadline_seconds=2 if case=='deadline' else 60,
        expected_host=state,boot=state['boot'],protected_files=state['protected'],inputs=files,private_material=None,
        fence=str(fence),fence_identity=identity,attempts=attempts,child_env=ENV,operator_approved=not synthetic,
        approval='2026-10-29 user: Приступай к исправлениям и запуску новой матрицы',
        protocol_sha256=files[str(root/'input/protocol.json')],runtime_methods_changed=False,release_installation=False,
        outer_bound_seconds=1200,session_bound_seconds=1100,capacity_poll_seconds=5,cleanup_seconds=5)


def launch(root,pin,synthetic=False):
    unit=root.name+'.service';standin=root.name+'-standin.service'
    return ['/usr/bin/systemd-run','--quiet','--no-ask-password','--unit='+unit,'--service-type=exec',
        '--property=Delegate=yes','--property=DelegateSubgroup=control','--property=RuntimeMaxSec='+('30' if synthetic else '1200'),
        '--property=KillMode=control-group','--property=KillSignal=SIGKILL','--property=SendSIGKILL=yes',
        '--property=Restart=no','--property=TimeoutStopSec=30','--property=NoNewPrivileges=yes',
        *(['--property=Wants='+standin,'--property=After='+standin] if synthetic else []),
        '--property=ExecStopPost=/usr/bin/python3 -B -I '+str(root/'coordinator.py')+' post '+str(root)+' '+pin,
        '/usr/bin/python3','-B','-I',str(root/'coordinator.py'),'main',str(root),pin]
