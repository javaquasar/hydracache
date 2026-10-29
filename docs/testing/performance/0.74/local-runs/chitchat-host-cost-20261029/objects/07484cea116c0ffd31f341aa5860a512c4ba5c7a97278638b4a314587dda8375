"""Read-only dedicated-host identity; no lock, service or clock mutation."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess

UNIT='hydracache-performance-supervisor-074.service'
MARKERS=('campaigns/active-campaign','campaigns/active-diagnostic.json','campaigns/.active-diagnostic.pending',
    'campaigns/diagnostic-requests.json','campaigns/.diagnostic-requests.pending','campaign-lifecycle-smoke-v1.json',
    'controller-loss-smoke-v1.json','diagnostic-intents-074','worker-authority-074')
PROTECTED=('/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074',
    '/var/lib/hydracache-performance/provisioning-receipt-074.json')


def need(ok,reason):
    if not ok:raise ValueError(reason)


def query(*argv):return subprocess.check_output(argv,text=True,timeout=10).strip()


def snapshot():
    need(os.geteuid()==os.getegid()==0,'read-only root access for protected host metadata')
    return dict(boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
        supervisor=query('/usr/bin/systemctl','show',UNIT,'-p','ActiveState','-p','SubState','-p','MainPID','-p','NRestarts','-p','InvocationID'),
        protected={name:hashlib.sha256(Path(name).read_bytes()).hexdigest() for name in PROTECTED},
        markers={name:os.path.lexists('/var/lib/hydracache-performance/'+name) for name in MARKERS},
        leftovers=sorted(str(p) for base in ('/tmp','/sys/fs/cgroup') for p in Path(base).iterdir()
            if p.name.startswith(('hc074-owned-root-','hc074-root-fixture-'))),
        kernel=platform.release(),machine=platform.machine(),libc=query('/usr/bin/getconf','GNU_LIBC_VERSION'),
        page_size=os.sysconf('SC_PAGE_SIZE'),manager=query('/usr/bin/systemctl','--version').splitlines()[0],
        mounts=Path('/proc/self/mountinfo').read_text(),
        namespaces={name:os.readlink('/proc/1/ns/'+name) for name in ('mnt','user')},
        clock_services=query('/usr/bin/systemctl','show','systemd-timesyncd.service','chrony.service','ntp.service',
            '-p','Id','-p','LoadState','-p','ActiveState','-p','MainPID'))


def admit(state):
    fields=dict(s.split('=',1) for s in state['supervisor'].splitlines())
    need(fields['ActiveState']=='active' and fields['SubState']=='running' and int(fields['MainPID'])>0,'stable running supervisor required; no mutation')
    need(not any(state['markers'].values()) and not state['leftovers'],'active campaign/lifecycle fixture context')
    need(state['machine']=='x86_64' and state['libc']=='glibc 2.39' and state['page_size']==4096,'reviewed exact probe ABI')
    need(state['protected'][PROTECTED[0]]=='f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62','installed supervisor pin')
    need(state['protected'][PROTECTED[1]]=='b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525','installed receipt pin')


if __name__=='__main__':
    value=snapshot();print(json.dumps(value,sort_keys=True))
