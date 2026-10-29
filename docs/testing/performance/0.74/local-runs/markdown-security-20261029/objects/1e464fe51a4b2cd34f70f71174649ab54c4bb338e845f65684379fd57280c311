"""One nonqualifying, read-only proc attribution of the retained held-startup fault.

The original test body, input image, 5/10/45-second bounds and assertions stay.
An independent observer reads only the owned command's descendants. It never
signals a sampled PID or uses a proc census as a retirement proof. No ptrace,
clock/service/configuration change, original delivery or product workload.
"""
from pathlib import Path
import json
import os
import subprocess
import sys
import time

import common as c
import native as n

OUT = c.BASE/'held-startup-attribution'
CASE = 'installed_controls::held_controls::held_entry_valid_readiness_then_duplicate_refuses_without_worker'
LINUX = '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-root-package-074/attribute_startup.py'


def proc(pid):
    path = Path('/proc')/str(pid)
    raw = (path/'stat').read_text()
    fields = raw.rsplit(')',1)[1].split()
    io = {}
    try:
        for line in (path/'io').read_text().splitlines():
            key, value = line.split(':',1)
            io[key] = int(value)
    except (OSError, ValueError) as error:
        io = dict(unavailable=type(error).__name__)
    try:
        argv = (path/'cmdline').read_bytes().decode().strip('\0').split('\0')
        executable = os.readlink(path/'exe')
    except OSError as error:
        argv = []
        executable = type(error).__name__
    children = set()
    for task in (path/'task').iterdir():
        try:
            children.update(int(v) for v in (task/'children').read_text().split())
        except (OSError, ValueError):
            pass
    return dict(pid=pid, ppid=int(fields[1]), start_ticks=int(fields[19]),
        state=fields[0], utime_ticks=int(fields[11]), stime_ticks=int(fields[12]),
        io=io, argv=argv, executable=executable), children


def native_worker():
    review = json.loads((c.BASE/'freeze/runtime-reviewed.json').read_bytes())
    c.guard(review)
    image = next(v for v in json.loads((c.BASE/'focus-194/images.json').read_bytes())
                 if v['kind']==['test'] and v['name']=='bootstrap_current_image')
    c.need(c.sha(Path(image['path']).read_bytes())==image['sha256'], 'same actual refused image')
    path = c.ROOT/'scripts/ci/watchdog_linux_group.py'
    c.need(c.sha(path.read_bytes())==n.GROUP_SHA, 'original exclusive single-threaded CI owner')
    owner = c.load(path,'startup_attribution_original_owner')
    class Observed:
        def __init__(self):
            self.inner = owner.LinuxGroup()
        def __getattr__(self,name):
            return getattr(self.inner,name)
        def spawn(self,*args):
            child = self.inner.spawn(*args)
            c.write(OUT,'leader.json',dict(pid=child.pid))
            return child
    _,env = n.environment('194')
    os.environ.clear()
    os.environ.update(env)
    argv = [image['path'],'--exact',CASE,'--test-threads=1','--color=never']
    c.write(OUT,'test-intent.json',dict(argv=argv,cwd=str(c.ROOT/c.PKG),image=image,
        attribution_only=True,acceptance_pass=False,bound=60,original_wait_seconds=5,
        original_activation_seconds=10,original_owned_case_seconds=45,retry=False))
    execution = c.owned()._owned(Observed(),argv,str(c.ROOT/c.PKG),n.capacity,bound=60)
    c.write(OUT,'linux-execution.json',execution)
    raise SystemExit(execution['exit'])


def observe():
    review = json.loads((c.BASE/'freeze/runtime-reviewed.json').read_bytes())
    c.guard(review)
    boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    _,env = n.environment('194')
    held = {}
    errors = []
    rows = 0
    with (OUT/'test.stdout.log').open('xb') as stdout, (OUT/'test.stderr.log').open('xb') as stderr, \
         (OUT/'proc.jsonl').open('x',encoding='utf8',newline='\n') as samples:
        worker = subprocess.Popen([sys.executable,'-B',str(Path(__file__)),'native-worker'],
            cwd=c.ROOT,env=env,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,start_new_session=True)
        deadline = time.monotonic()+120
        try:
            while worker.poll() is None:
                c.need(time.monotonic()<deadline,'bounded nonqualifying observer')
                frontier = [worker.pid]
                seen = set()
                while frontier:
                    pid = frontier.pop()
                    if pid in seen:
                        continue
                    seen.add(pid)
                    c.need(len(seen)<=128,'bounded owned descendant observation')
                    try:
                        before, children = proc(pid)
                        key = (pid,before['start_ticks'])
                        if key not in held:
                            c.need(len(held)<128,'bounded retained observation handles')
                            fd = os.pidfd_open(pid,0)
                            try:
                                after,_ = proc(pid)
                            except BaseException:
                                os.close(fd)
                                raise
                            if after['start_ticks'] != before['start_ticks']:
                                os.close(fd)
                                continue
                            held[key] = fd
                        samples.write(json.dumps(dict(at=time.monotonic(),**before))+'\n')
                        rows += 1
                        frontier.extend(children)
                    except (OSError,ValueError) as error:
                        errors.append(dict(pid=pid,error=type(error).__name__))
                samples.flush()
                time.sleep(0.05)
            worker.wait(timeout=5)
        finally:
            for fd in held.values():
                os.close(fd)
    execution = json.loads((OUT/'linux-execution.json').read_bytes())
    c.need(execution['retired'] and not execution['errors'],'actual ECHILD retirement, never proc inference')
    c.guard(review)
    images = []
    for line in (c.BASE/'focus-194/build/stdout.log').read_text().splitlines():
        row = json.loads(line)
        if row.get('reason')=='compiler-artifact' and row.get('executable') and not row['profile']['test'] \
                and row['target']['name']=='diagnostic_held_bootstrap':
            path = Path(row['executable'])
            images.append(dict(path=str(path),bytes=path.stat().st_size,sha256=c.sha(path.read_bytes())))
    c.write(OUT,'native-result.json',dict(attribution_complete=True,acceptance_pass=False,
        test_exit=execution['exit'],source_unchanged=True,rows=rows,observation_errors=errors,
        ticks_per_second=os.sysconf('SC_CLK_TCK'),bootstrap_images=images,
        boot_before=boot,boot_after=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
        original_wait_seconds=5,original_activation_seconds=10,original_owned_case_seconds=45,
        server=False,services=False,clocks_changed=False,performance_claim=False))


def windows_worker():
    argv = ['wsl.exe','-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B',LINUX,'observe']
    value = c.owned()
    result = value._owned(value.load_job().WindowsJob(),argv,str(c.ROOT),c.capacity,bound=180)
    c.write(OUT,'windows-execution.json',result)
    raise SystemExit(result['exit'])


def main():
    OUT.mkdir(exist_ok=False)
    (OUT/'producer.py').write_bytes(Path(__file__).read_bytes())
    with (OUT/'stdout.log').open('xb') as stdout, (OUT/'stderr.log').open('xb') as stderr:
        review = json.loads((c.BASE/'freeze/runtime-reviewed.json').read_bytes())
        c.guard(review)
        before = c.git_snapshot()
        c.write(OUT,'intent.json',dict(attribution_only=True,acceptance_pass=False,retry=False,
            source=before,freeze_sha256=c.sha((c.BASE/'freeze/runtime-reviewed.json').read_bytes()),
            observer='read-only owned descendants; no ptrace, signal or product operation'))
        child = subprocess.Popen([sys.executable,'-B',str(Path(__file__)),'windows-worker'],
            cwd=c.ROOT,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
        code = child.wait(timeout=190)
        execution = json.loads((OUT/'windows-execution.json').read_bytes())
        c.need(code==0 and execution['retired'] and not execution['errors'],'complete bounded attribution delivery')
        c.guard(review)
        c.need(before==c.git_snapshot(),'source/HEAD unchanged')
        result = json.loads((OUT/'native-result.json').read_bytes())
        c.need(result['source_unchanged'] and result['boot_before']==result['boot_after'],'attribution postflight')
        c.write(OUT,'result.json',dict(attribution_complete=True,acceptance_pass=False,
            observed_test_exit=result['test_exit'],source_unchanged=True,performance_claim=False))
    print(json.dumps(dict(attribution_complete=True,acceptance_pass=False)))


if __name__ == '__main__':
    mode = sys.argv[1:]
    if mode == ['native-worker']:
        native_worker()
    elif mode == ['observe']:
        observe()
    elif mode == ['windows-worker']:
        windows_worker()
    else:
        c.need(not mode,'one bounded attribution attempt')
        main()
