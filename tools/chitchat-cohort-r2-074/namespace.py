"""Bounded private mount context; no host mount or existing service changes."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import time

def need(ok,why):
    if not ok:raise RuntimeError(why)

def load(path,name):
    spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m

def identity(pid):
    return Path('/proc/'+str(pid)+'/stat').read_text().rsplit(')',1)[1].split()[19]

def host(root):return load(root/'input/host.py','hc074_private_host').snapshot()

def metadata(root):return json.loads((root/'namespace-ready.json').read_bytes())

def live(meta):
    need(identity(meta['pid'])==meta['start_ticks'],'original namespace owner lifetime')
    state=Path('/proc/'+str(meta['pid'])+'/ns/mnt').stat()
    need([state.st_dev,state.st_ino]==meta['identity'],'original held namespace inode')
    need(time.monotonic_ns()<meta['deadline_ns'],'original namespace outer deadline')

def join(root,p):
    meta=p['namespace'];live(meta)
    fd=os.open('/proc/'+str(meta['pid'])+'/ns/mnt',os.O_RDONLY|os.O_CLOEXEC)
    try:
        state=os.fstat(fd);need([state.st_dev,state.st_ino]==meta['identity'],'opened original namespace descriptor')
        live(meta);os.setns(fd,0)
    finally:os.close(fd)
    verify(p)

def verify(p):
    meta=p['namespace'];live(meta);current(p)

def current(p):
    meta=p['namespace']
    state=Path('/proc/self/ns/mnt').stat()
    need([state.st_dev,state.st_ino]==meta['identity'],'same original private mount namespace')

def keeper(root):
    need(os.getuid()==os.getgid()==0,'private namespace owner root')
    os.sched_setaffinity(0,{0,5,6,7})
    state=Path('/proc/self/ns/mnt').stat();parent=Path('/proc/1/ns/mnt').stat()
    need((state.st_dev,state.st_ino)!=(parent.st_dev,parent.st_ino),'own private namespace, never host mount context')
    # Only a new owned fixture mount, inside the already private namespace.
    fixture=root/'fixture-mount';fixture.mkdir()
    subprocess.run(['/usr/bin/mount','--bind',str(fixture),str(fixture)],check=True,timeout=10)
    meta=dict(pid=os.getpid(),start_ticks=identity(os.getpid()),identity=[state.st_dev,state.st_ino],
        deadline_ns=time.monotonic_ns()+1200*10**9,outer_bound_seconds=1200,host=host(root))
    need(not any(' shared:' in line or ' master:' in line for line in meta['host']['mounts'].splitlines()),'recursively private mount propagation')
    m=load(root/'maintenance_base.py','hc074_keeper_receipt');m.publish(root,'namespace-ready.json',meta);(root/'namespace-ready.json').chmod(0o400)
    while time.monotonic_ns()<meta['deadline_ns']:time.sleep(.2)

def stop(root,p,m):
    unit=p['namespace_unit'];state=m.service_state(unit)
    need(state['MainPID']==str(p['namespace']['pid']) and state['ActiveState']=='active','only original namespace owner unit may stop')
    live(p['namespace']);m.command(['/usr/bin/systemctl','stop',unit],10)
    state=m.service_state(unit);need(state['MainPID']=='0' and state['ActiveState']=='inactive','owned namespace owner retired')
    m.publish(root,'namespace-retired.json',dict(unit=unit,original_identity=p['namespace']['identity'],state=state))

if __name__=='__main__':
    need(len(sys.argv)==3 and sys.argv[1]=='keeper','fixed keeper invocation');keeper(Path(sys.argv[2]))
