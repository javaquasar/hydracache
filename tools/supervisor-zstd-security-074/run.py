"""Fixed local phases: no free-form Cargo, filters, deadlines or native retries."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import traceback
import re
import common as c

def image(stage,kind,compiler):
    raw=(c.phase(stage+'-'+kind+'-build-'+compiler)/'stdout.log').read_text(encoding='utf8')
    matches=[]
    for line in raw.splitlines():
        if not line.startswith('{'):continue
        row=json.loads(line)
        if row.get('reason')=='compiler-artifact' and row.get('executable') and row['profile']['test']:
            if kind=='archive' and row['target']['name']=='archive' and row['target']['kind']==['test'] or kind=='probe' and row['target']['name']=='hc074_supervisor_zstd_wire':matches.append(row)
    c.need(len(matches)==1,'exact compiled native image')
    path=Path(matches[0]['executable']);c.need(path.is_file() and not path.is_symlink(),'actual compiled executable')
    return path

def command(label):
    if label in ('native-supply-194','native-supply-markdown-194'):
        return [str(Path(os.environ['SystemRoot'])/'System32/wsl.exe'),'-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B','/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/supervisor-zstd-security-074/supply.py','seed' if label=='native-supply-194' else 'seed-markdown'],dict(os.environ)
    if label.startswith('gnu-'):
        c.need(os.name=='nt','original Windows outer owner')
        allowed=('metadata-r4-194','check-194','check-188','clippy-194','clippy-188','format-194','candidate-archive-build-194','candidate-archive-build-188',*[f'candidate-archive-{s}-{v}' for s in ['list',*map(str,range(4))] for v in ['194','188']])
        inner=label[4:];c.need(inner in allowed,'fixed native phases')
        return [str(Path(os.environ['SystemRoot'])/'System32/wsl.exe'),'-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B','/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/supervisor-zstd-security-074/run.py',inner],dict(os.environ)
    parts=label.split('-');compiler=parts[-1];tool,env,suffix=c.environment(compiler)
    target=c.ROOT/'target' if os.name=='nt' else c.NATIVE/'builds'/compiler
    base=['--offline','--locked','--manifest-path',str(c.ROOT/'Cargo.toml'),'--target-dir',str(target),'-p','hydracache-long-run-supervisor-074']
    action='-'.join(parts[:-1])
    if action in ('baseline-archive-build','candidate-archive-build'):argv=[str(tool/('cargo'+suffix)),'test',*base,'--test','archive','--no-run','--message-format=json']
    elif action=='check':argv=[str(tool/('cargo'+suffix)),'check',*base,'--all-targets','--all-features']
    elif action=='clippy':argv=[str(tool/('cargo'+suffix)),'clippy',*base,'--all-targets','--all-features','--','-D','warnings']
    elif action in ('metadata','metadata-r2','metadata-r3','metadata-r4'):argv=[str(tool/('cargo'+suffix)),'metadata','--manifest-path',str(c.ROOT/'Cargo.toml'),'--all-features','--format-version','1','--offline','--locked']
    elif action=='format':argv=[str(tool/('cargo'+suffix)),'fmt','--manifest-path',str(c.ROOT/'Cargo.toml'),'--all','--','--check']
    elif label=='probe-resolve-r2-194':argv=[str(tool/('cargo'+suffix)),'generate-lockfile','--manifest-path',str(c.BASE/'probe/Cargo.toml'),'--offline']
    elif len(parts)==4 and parts[0] in ('baseline','candidate','candidate2') and parts[1] in ('archive','probe') and parts[2]=='build':
        argv=[str(tool/('cargo'+suffix)),'test','--manifest-path',str(c.BASE/'probe/Cargo.toml'),'--target-dir',str(target),'--offline','--locked','--no-run','--message-format=json']
    elif len(parts)==4 and parts[0] in ('baseline','candidate','candidate2') and parts[1] in ('archive','probe'):
        argv=[str(image(parts[0],parts[1],compiler))]
        if parts[2]=='list':argv+=['--list']
        else:
            names=c.TESTS if parts[1]=='archive' else ['actual_archive_wire_is_deterministic_and_cross_decodes']
            c.need(parts[2] in [str(i) for i in range(len(names))],'exact original catalog case')
            argv+=['--exact',names[int(parts[2])],'--test-threads=1','--nocapture']
    else:raise ValueError('fixed actions only')
    return argv,env

def bound(label):
    p=label.split('-')
    return (10 if p[2]=='list' else 5) if len(p)==4 and p[0] in ('baseline','candidate','candidate2') and p[1] in ('archive','probe') and p[2]!='build' else 1200

def account(label):
    p=label.split('-')
    if len(p)!=4 or p[0] not in ('baseline','candidate','candidate2') or p[1] not in ('archive','probe') or p[2]=='build':return {}
    names=c.TESTS if p[1]=='archive' else ['actual_archive_wire_is_deterministic_and_cross_decodes']
    raw=(c.phase(label)/'stdout.log').read_text(encoding='utf8')
    if p[2]=='list':
        actual=[s[:-6] for s in raw.splitlines() if s.endswith(': test')]
        count=f'{len(names)} '+('test' if len(names)==1 else 'tests')+', 0 benchmarks'
        c.need(sorted(actual)==names and raw.splitlines()[-1]==count,'complete unchanged catalog')
        return dict(catalog=actual)
    rows=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',raw)
    c.need(rows==[('1','0','0','0',str(len(names)-1))],'exact case accounting; complete suite accounted separately')
    return dict(passed=1,case=names[int(p[2])],filtered=len(names)-1)

def worker(label):
    argv,env=command(label);os.environ.clear();os.environ.update(env)
    group=c.owned.load_job().WindowsJob() if os.name=='nt' else c.load(c.ROOT/'scripts/ci/watchdog_linux_group.py','direct_zstd_linux_owner').LinuxGroup()
    deadline=json.loads((c.phase(label)/'intent.json').read_bytes())['deadline']
    value=c.owned._owned(group,argv,str(c.ROOT),c.capacity,bound=bound(label),deadline=min(deadline,time.monotonic()+bound(label)))
    c.write(c.phase(label),'execution.json',value);raise SystemExit(value['exit'])

def main(label):
    # Every command keeps separated streams and a first-result latch.
    out=c.phase(label);out.mkdir(parents=True,exist_ok=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        deadline=time.monotonic()+(1100 if os.name=='posix' else 1200)
        before=c.source();head=c.snapshot();argv,env=command(label)
        c.write(out,'intent.json',dict(argv=argv,environment={k:v for k,v in env.items() if k in ('RUSTC','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_NET_OFFLINE','CARGO_INCREMENTAL','CARGO_BUILD_JOBS','RUSTUP_TOOLCHAIN','CARGO_HOME')},source=before,git=head,deadline=deadline,bound_seconds=bound(label),session_seconds=1100 if os.name=='posix' else None,floor_bytes=4*1024**3,poll_seconds=5,cleanup_seconds=5,retry=False,server=False,image_sha256=c.sha(Path(argv[0]).read_bytes())))
        value=dict(complete=False,release_admitted=False)
        try:
            boot=Path('/proc/sys/kernel/random/boot_id').read_text() if os.name=='posix' else None
            vendor=c.registry(deadline) if os.name=='posix' else None
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),label,'worker'],cwd=c.ROOT,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=bound(label)+10)
            owner=json.loads((out/'execution.json').read_bytes())
            c.need(child.returncode==owner['exit']==0 and owner['retired'] and not owner['errors'],'first complete original ownership')
            after=c.source()
            if label=='probe-resolve-r2-194':
                lock=(c.BASE/'probe/Cargo.lock').relative_to(c.ROOT).as_posix()
                c.need(lock not in before,'first fixture-only resolution')
                value['probe_lock_sha256']=after.pop(lock)
            c.need(after==before and c.snapshot()==head,'whole source/ref postflight')
            if os.name=='posix':
                c.need(vendor==c.registry(deadline),'complete unchanged original registry/config custody')
                c.need(boot==Path('/proc/sys/kernel/random/boot_id').read_text(),'unchanged original boot')
                c.need(time.monotonic()<deadline,'original complete native session deadline')
                value['registry']=vendor
            if label.startswith('gnu-'):c.need(json.loads((c.BASE/'gnu'/label[4:]/'result.json').read_bytes())['complete'],'complete original native phase')
            value.update(account(label))
            value['complete']=True
        except BaseException as error:value['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
        c.write(out,'result.json',value)
    print(json.dumps(dict(phase=label,**value)));raise SystemExit(0 if value['complete'] else 1)

if __name__=='__main__':
    if len(sys.argv)==3 and sys.argv[2]=='worker':worker(sys.argv[1])
    c.need(len(sys.argv)==2,'one fixed phase');main(sys.argv[1])
