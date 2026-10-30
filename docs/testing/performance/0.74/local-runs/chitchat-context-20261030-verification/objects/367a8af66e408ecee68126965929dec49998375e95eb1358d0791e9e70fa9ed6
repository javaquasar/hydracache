"""Fixed post-capture controls; original local CI bounds and source custody."""
import json
import os
from pathlib import Path
import subprocess
import sys
import traceback
import analyze as a
sys.path.insert(0,str(a.ROOT/'tools/chitchat-borrowed-074'))
import recipe as p
c=p.c
HERE=Path(__file__).resolve().parent
DOCS=('docs/articles/007-optimization-starts-with-measurement.md',
 'docs/testing/performance/0.74/README.md','docs/testing/performance/0.74/release-closure-execution-plan-20261026.md',
 'docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md',
 'docs/testing/performance/0.74/chitchat-context-result-20261030.md',
 'docs/testing/performance/0.74/chitchat-mtu-next-candidate-plan-20261030.md')
RUSTFMT=Path(os.environ['USERPROFILE'])/'.rustup/toolchains/1.94.0-x86_64-pc-windows-msvc/bin/rustfmt.exe'
COMMANDS={
 'controls':[sys.executable,'-B','-m','unittest','discover','-s',str(HERE),'-p','test_*.py','-v'],
 'syntax':[sys.executable,'-B','-c',"import ast,pathlib; [ast.parse(p.read_bytes(),filename=str(p)) for n in ('chitchat-borrowed-074','chitchat-context-result-074') for p in (pathlib.Path('tools')/n).glob('*.py')]"],
 'format':[str(RUSTFMT),'--edition','2021','--check',str(p.HERE/'context_tests.rs'),str(p.HERE/'native_tests.rs')],
 'byte-verify':[sys.executable,'-B',str(HERE/'verify.py')],
 'restore-verify':[sys.executable,'-B',str(HERE/'check_restore.py')],
 'doc-check':[str(c.ROOT/'target/debug/xtask.exe'),'doc-check'],
 'performance-contract':[str(c.ROOT/'target/debug/xtask.exe'),'performance-contract-check','--release','0.74']}


def source():
    value=c.source()
    for folder in (p.HERE,HERE):
        for path in folder.iterdir():
            if path.is_file():value[path.relative_to(c.ROOT).as_posix()]=c.sha(path.read_bytes())
    for name in DOCS:value[name]=c.sha((c.ROOT/name).read_bytes())
    for name in ('chitchat-context-20261030','chitchat-security-only-20261030','chitchat-security-only-20261030-verification'):
        path=c.ROOT/'docs/testing/performance/0.74/local-runs'/name/'index.json'
        value[path.relative_to(c.ROOT).as_posix()]=c.sha(path.read_bytes())
    return value


def main(mode):
    c.need(mode in COMMANDS,'fixed offline modes');out=a.BASE/'final'/mode;out.mkdir(parents=True,exist_ok=False)
    argv=COMMANDS[mode];before=source();head=c.snapshot();image=c.sha(Path(argv[0]).read_bytes())
    if mode in ('doc-check','performance-contract'):c.need(image=='e49f4694c8dd18629d33cdeda08d7f2a617ccae14cc3be41f10b52ad233471a9','held unchanged qualified xtask')
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,image_sha256=image,bound=1200,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,release_admitted=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),mode,'worker'],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            own=json.loads((out/'execution.json').read_bytes())
            c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete original final Job outcome')
            c.need(source()==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==image,'whole source/docs/ref/image postflight')
            result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result);print(json.dumps(dict(mode=mode,**result)));raise SystemExit(0 if result['complete'] else 1)


if __name__=='__main__':
    c.need(os.name=='nt' and len(sys.argv) in (2,3),'fixed Windows offline producer')
    if len(sys.argv)==3:
        c.need(sys.argv[2]=='worker' and sys.argv[1] in COMMANDS,'fixed offline worker');_,env,_=c.environment('194');os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),COMMANDS[sys.argv[1]],str(c.ROOT),c.capacity,bound=1200)
        c.write(a.BASE/'final'/sys.argv[1],'execution.json',own);raise SystemExit(own['exit'])
    main(sys.argv[1])
