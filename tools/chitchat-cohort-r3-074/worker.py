"""One measured image in the original owned writer cgroup, exact validator."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import types
import math

sys.dont_write_bytecode=True


def fenced_spawn(self,command,cwd,log_handle,deadline):
    # Original LinuxGroup.spawn, with only the held fence added to pass_fds.
    if self.closed or self.attempted:
        raise GroupFailure('closed or already-used CI group')
    self.attempted=True
    process=subprocess.Popen(command,cwd=cwd,stdout=log_handle,
        stderr=subprocess.STDOUT if log_handle is not None else None,
        start_new_session=True,creationflags=0,pass_fds=(self.fence,))
    self.child=LinuxChild(self,process)
    return self.child


def load(path,name):
    spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m


def main():
    root=Path(sys.argv[1]);seq=int(sys.argv[2]);pin=sys.argv[3];fence=int(sys.argv[4])
    c=load(root/'coordinator.py','hc074_writer_coordinator');p=c.read_plan(root,pin);m=c.backend(root)
    record=json.loads((root/(str(seq)+'.intent.json')).read_bytes());attempt=p['attempts'][seq]
    deadline=record['deadline_ns']/10**9
    state=os.fstat(fence);c.need([state.st_dev,state.st_ino]==p['fence_identity'] and fence==record['fence_fd'],'original inherited fence descriptor')
    c.need(Path('/proc/self/cgroup').read_text().strip()=='0::'+record['group'].removeprefix('/sys/fs/cgroup'),'original writer placement')
    c.need(os.getuid()==os.getgid()==0 and set(os.sched_getaffinity(0))=={1,2,3,4},'same approved root/CPU profile as controls')
    c.frame(p,root,m);adapter=load(root/'input/remote.py','hc074_writer_adapter')
    manifest=json.loads((root/'input/inputs.json').read_bytes());adapter.verify(root,manifest)
    c.need(time.monotonic()<deadline,'original deadline before measured image')
    group=load(root/'input/group.py','hc074_original_worker_group')
    globals().update(GroupFailure=group.GroupFailure,LinuxChild=group.LinuxChild)
    namespace=dict(time=time,math=math,CapacityError=adapter.CapacityError,prepare=types.SimpleNamespace(require=c.need))
    exec(compile((root/'input/owned_function.py').read_bytes(),'original_owned_function.py','exec'),namespace)
    argv=[str(root/'input/images'/attempt['variant']),attempt['mode']]
    # LinuxGroup's retained child is observed without replacing the subprocess API.
    class Observed(group.LinuxGroup):
        def __init__(self):
            super().__init__();self.fence=fence
        def spawn(self,*args):
            child=fenced_spawn(self,*args)
            m.publish(root,str(seq)+'.worker-child.json',dict(pid=child.pid,
                start_ticks=Path('/proc/'+str(child.pid)+'/stat').read_text().rsplit(')',1)[1].split()[19]))
            return child
    value=namespace['_owned'](Observed(),argv,str(root),adapter.capacity,bound=1200,deadline=deadline)
    m.publish(root,str(seq)+'.execution.json',value)
    c.need(value['exit']==0 and value['retired'] and not value['errors'],'complete original image ownership')
    c.frame(p,root,m);adapter.verify(root,manifest)
    return value['exit']


if __name__=='__main__':raise SystemExit(main())
