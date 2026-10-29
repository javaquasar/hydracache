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

def user_tree(root):
    """Move the SAME private user mounts under own mountpoint dentries.

    Keep each original filesystem-root FD throughout setup. MS_MOVE relocates
    an attached private mount object; its kernel mount ID and root inode must
    remain identical. No detached-FD bind, file copy or global mount operation.
    """
    parent=Path('/run/user');held=[]
    def mounts():
        return {x.split()[4]:x.split() for x in Path('/proc/self/mountinfo').read_text().splitlines()}
    records=mounts()
    try:
        for child in sorted(parent.iterdir()):
            need(child.name.isdigit() and child.is_dir() and not child.is_symlink(),'bounded numeric user directory')
            need(str(child) in records and records[str(child)][3]=='/','each admitted user root is its own filesystem mount')
            state=child.stat();fd=os.open(child,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW|os.O_CLOEXEC)
            opened=os.fstat(fd);need((opened.st_dev,opened.st_ino)==(state.st_dev,state.st_ino),'held original user root')
            staging=root/('user-root-'+child.name);staging.mkdir(mode=0o700)
            held.append((child.name,fd,state,staging,records[str(child)][0]))
            subprocess.run(['/usr/bin/mount','--move',str(child),str(staging)],check=True,timeout=10)
            moved=staging.stat();need((moved.st_dev,moved.st_ino)==(opened.st_dev,opened.st_ino),'same original root after first private mount move')
            need(mounts()[str(staging)][0]==records[str(child)][0],'same kernel mount ID after first private move')
        need(not any(x.startswith('/run/user/') for x in mounts()),'all original user mounts relocated before own parent creation')
        state=parent.stat()
        subprocess.run(['/usr/bin/mount','-t','tmpfs','-o','mode='+oct(state.st_mode&0o777)[2:]+',uid='+str(state.st_uid)+',gid='+str(state.st_gid)+',nosuid,nodev','hc074-private-user-parent',str(parent)],check=True,timeout=10)
        receipt=[]
        for name,fd,original,staging,mount_id in held:
            target=parent/name;target.mkdir(mode=original.st_mode&0o777);os.chown(target,original.st_uid,original.st_gid)
            subprocess.run(['/usr/bin/mount','--move',str(staging),str(target)],check=True,timeout=10)
            now=target.stat();held_state=os.fstat(fd)
            need((now.st_dev,now.st_ino)==(held_state.st_dev,held_state.st_ino)==(original.st_dev,original.st_ino),'same held user filesystem root after relocation')
            need(mounts()[str(target)][0]==mount_id,'same original kernel mount ID after final private move')
            staging.rmdir()
            receipt.append(dict(user=name,root_identity=[now.st_dev,now.st_ino],original_private_mount_id=mount_id,final_private_mount_id=mounts()[str(target)][0]))
        return receipt
    finally:
        for _,fd,_,_,_ in held:os.close(fd)


def keeper(root):
    need(os.getuid()==os.getgid()==0,'private namespace owner root')
    os.sched_setaffinity(0,{0,5,6,7})
    state=Path('/proc/self/ns/mnt').stat();parent=Path('/proc/1/ns/mnt').stat()
    need((state.st_dev,state.st_ino)!=(parent.st_dev,parent.st_ino),'own private namespace, never host mount context')
    users=user_tree(root)
    # Only a new owned fixture mount, inside the already private namespace.
    fixture=root/'fixture-mount';fixture.mkdir()
    subprocess.run(['/usr/bin/mount','--bind',str(fixture),str(fixture)],check=True,timeout=10)
    meta=dict(pid=os.getpid(),start_ticks=identity(os.getpid()),identity=[state.st_dev,state.st_ino],
        deadline_ns=time.monotonic_ns()+1200*10**9,outer_bound_seconds=1200,user_roots=users,host=host(root))
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
