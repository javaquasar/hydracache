"""Retirement remains strict when numerical admission refuses a host frame."""
import os
from pathlib import Path
import shutil

def need(ok,why):
    if not ok:raise RuntimeError(why)

def mount_delta(before,after,target,synthetic=False):
    def parts(raw):
        records=[x.split() for x in raw.splitlines()];leaf=[x for x in records if x[4]==target]
        need(len(leaf)<=1 and not any(x[4].startswith(target+'/') for x in records),'at most one leaf mount, no nested replacement')
        return [x for x in records if x[4]!=target],leaf[0] if leaf else None
    a,old=parts(before);b,new=parts(after);need(a==b,'all other mount records remain exact')
    need(old is not None,'original leaf must exist')
    if new is None:
        at=old.index('-');need(not synthetic and target=='/run/user/1000' and old[3]=='/' and old[at+1:at+3]==['tmpfs','tmpfs'],'only recorded unrelated user tmpfs leaf disappearance permits recovery')
        return dict(before=' '.join(old),after=None,measurement_frame_pass=False)
    if synthetic:
        changed=list(old);options=changed[5].split(',');need('rw' in options,'original owned fixture read-write')
        changed[5]=','.join('ro' if x=='rw' else x for x in options)
        need(changed==new,'only predeclared synthetic fixture bind remount read-only')
    else:
        for x in (old,new):
            at=x.index('-');need(x[3]=='/' and x[at+1:at+3]==['tmpfs','tmpfs'],'exact user tmpfs')
        def signature(x):return [v for i,v in enumerate(x) if i not in (0,2) and not v.startswith('shared:')]
        need(signature(old)==signature(new) and old!=new,'only unrelated tmpfs kernel identities may differ for restoration')
    return dict(before=' '.join(old),after=' '.join(new),measurement_frame_pass=False)

def core(p,root,m,ns,external=False):
    # These checks are additional recovery checks. No numeric predicate is waived.
    m.static_frame(p)
    if not external:ns.current(p)
    cpu=ns.load(root/'coordinator.py','hc074_restoration_cpu').cpu_profile()
    need(cpu==p['cpu_profile'] and shutil.disk_usage(root).free>=4*1024**3,'original CPU/capacity predicates during restoration')
    actual=ns.host(root);expected=p['external_host'] if external else p['expected_host']
    need({k:v for k,v in actual.items() if k not in ('mounts','supervisor')}==
         {k:v for k,v in expected.items() if k not in ('mounts','supervisor')},'protected original host context for restoration')
    if actual['mounts']!=expected['mounts']:
        fixture=p['synthetic'] and not external and str(p.get('synthetic_case','')).startswith('frame-denial')
        target=str(root/'fixture-mount') if fixture else '/run/user/1000'
        mount_delta(expected['mounts'],actual['mounts'],target,fixture)
    return actual

def post(p,root,m,ns,frame,types_base,external=False):
    original=m.static_frame;outcomes=[]
    def validation(plan):
        try:
            if external:raise RuntimeError('original mount namespace unavailable at terminal postflight')
            frame(plan,root,types_base(original))
            outcomes.append(dict(full_measurement_frame_pass=True))
        except Exception as error:
            # Store the refusal before any restoration effect. The unchanged
            # base post still requires every original cgroup inode/populated=0,
            # retired timer and exclusive original fence before service start.
            reason=dict(type=type(error).__name__,reason=str(error),full_measurement_frame_pass=False)
            m.publish(root,'post-frame-refusal-'+str(len(outcomes))+'.json',reason);outcomes.append(reason)
            core(plan,root,types_base(original),ns,external)
    m.static_frame=validation
    try:
        m.post(p,root)
        m.publish(root,'host-postflight.json',ns.host(root))
        m.publish(root,'postflight-result.json',dict(restoration_verified=True,full_measurement_frames_pass=all(x['full_measurement_frame_pass'] for x in outcomes),
            frames=outcomes,admission=False,external_recovery=external))
    finally:m.static_frame=original
