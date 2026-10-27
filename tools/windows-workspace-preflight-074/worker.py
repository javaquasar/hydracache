"""Fixed Cargo worker. Inherited separate files; original Job only, no PID fallback."""
import hashlib
import json
import os
from pathlib import Path
import sys
import time
from contextlib import contextmanager


@contextmanager
def gate_lock(root):
    import msvcrt
    path=root/'target/windows-fixed-cargo-20261027/active.lock'
    path.parent.mkdir(parents=True,exist_ok=True)
    with path.open('a+b') as stream:
        stream.seek(0)
        msvcrt.locking(stream.fileno(),msvcrt.LK_NBLCK,1)
        try:
            yield
        finally:
            stream.seek(0)
            msvcrt.locking(stream.fileno(),msvcrt.LK_UNLCK,1)


def run(out,expected_sha,mode):
    # Validate all current helper bodies before importing them. Never load archive code.
    root=Path(__file__).resolve().parents[2]
    if out.resolve()!=out or not out.is_relative_to(root/'target'):
        raise ValueError('original worker output custody')
    request_raw=(out/'request.json').read_bytes()
    if hashlib.sha256(request_raw).hexdigest()!=expected_sha:
        raise ValueError('original parent request bytes drift')
    request=json.loads(request_raw)
    if set(request)!= {'mode','specification','frame','deadline'} or request['mode']!=mode:
        raise ValueError('original parent request schema/mode')
    if not isinstance(request['deadline'],(int,float)) or not __import__('math').isfinite(request['deadline']):
        raise ValueError('finite original absolute deadline')
    for name,digest in request['frame']['helpers'].items():
        path=root/name
        if not path.resolve().is_relative_to(root) or path.is_symlink() or path.is_junction():
            raise ValueError('worker helper custody')
        if hashlib.sha256(path.read_bytes()).hexdigest()!=digest:
            raise ValueError('worker helper bytes drift')
    import owned
    import prepare as p
    import custody
    p.require(os.name=='nt' and out.resolve()==out and out.is_relative_to(root/'target'), 'worker output custody')
    spec=owned.specification(request['mode'])
    p.require(request['specification']==spec,'fixed specification drift')
    p.require(all(os.environ.get(k)==v for k,v in spec['environment'].items()),'worker environment drift')
    custody.equivalent(request['frame'],custody.frame(root))
    p.require(time.monotonic()<request['deadline'],'worker startup exhausted original bound')
    job=owned.load_job(root).WindowsJob()
    lows=[]
    def capacity():
        import shutil
        free=shutil.disk_usage(root).free
        lows.append(free)
        if free<spec['floor_bytes']:raise owned.CapacityError('original physical 4 GiB floor')
    receipt=owned._owned(job,spec['argv'],root,capacity,bound=spec['bound_seconds'],deadline=request['deadline'])
    p.write(out,'controller.json',dict(mode=request['mode'],request_sha256=hashlib.sha256(request_raw).hexdigest(),
        specification=spec,execution=receipt,low_observed_free_bytes=min(lows) if lows else None,
        original_job_sha256=owned.JOB_SHA,worker_pid=os.getpid(),capacity_is_not_reservation=True))
    return receipt['exit']


if __name__=='__main__':
    out=Path(sys.argv[1])
    root=Path(__file__).resolve().parents[2]
    if not out.is_absolute() or out.resolve()!=out or not out.is_relative_to(root/'target'):
        raise ValueError('refuse output before writing any worker receipt')
    try:
        with gate_lock(Path(__file__).resolve().parents[2]):
            raise SystemExit(run(out,sys.argv[2],sys.argv[3]))
    except Exception as error:
        with (out/'worker-refusal.json').open('x',encoding='utf-8') as stream:
            json.dump(dict(type=type(error).__name__,reason=str(error),gate_pass=False),stream)
        raise
