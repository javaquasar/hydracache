"""Fixed complete-lane offline gates, preserving the original Job contract."""
import json,os,subprocess,sys,traceback
from pathlib import Path
import packet as p
s=p.s;c=p.c;m=p.m;ROOT=s.ROOT;HERE=Path(__file__).resolve().parent
DOCS=('docs/articles/007-optimization-starts-with-measurement.md','docs/testing/performance/0.74/README.md',
 'docs/testing/performance/0.74/release-closure-execution-plan-20261026.md','docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md',
 'docs/testing/performance/0.74/chitchat-security-package-plan-20261030.md','docs/testing/performance/0.74/chitchat-security-package-result-20261030.md',
 'docs/testing/performance/0.74/chitchat-security-package-complete-result-20261030.md')
MODES=('controls','syntax','byte-verify','restore-verify','doc-check','performance-contract')
FINAL=s.BASE/'final'

def commands(digest):
    return {
      'controls':[sys.executable,'-B',str(HERE/'test_review.py')],
      'syntax':[sys.executable,'-B','-c',"import ast,pathlib;[ast.parse(p.read_bytes(),filename=str(p)) for n in "+repr(p.DIRS)+" for p in (pathlib.Path('tools')/n).glob('*.py')]"],
      'byte-verify':[sys.executable,'-B',str(HERE/'packet.py'),'verify',digest],
      'restore-verify':[sys.executable,'-B',str(HERE/'packet.py'),'restore',digest,str(s.BASE/'source-only-review')],
      'doc-check':[str(ROOT/'target/debug/xtask.exe'),'doc-check'],
      'performance-contract':[str(ROOT/'target/debug/xtask.exe'),'performance-contract-check','--release','0.74']}

def source(digest):
    values=s.source()
    for n in p.DIRS:
        for f in p.p.walk(ROOT/'tools'/n):values[f.relative_to(ROOT).as_posix()]=c.sha(f.read_bytes())
    for n in DOCS:values[n]=c.sha((ROOT/n).read_bytes())
    c.need(c.sha((p.OUT/'index.json').read_bytes())==digest,'whole external primary digest')
    values[(p.OUT/'index.json').relative_to(ROOT).as_posix()]=digest
    return values

def main(mode,digest):
    c.need(mode in MODES,'fixed final offline mode');out=FINAL/mode;out.mkdir(parents=True,exist_ok=False)
    argv=commands(digest)[mode];before=source(digest);head=c.snapshot();image=c.sha(Path(argv[0]).read_bytes())
    if mode in ('doc-check','performance-contract'):c.need(image=='e49f4694c8dd18629d33cdeda08d7f2a617ccae14cc3be41f10b52ad233471a9','same held xtask')
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,image_sha256=image,bound=1200,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,release_admitted=False,product_execution=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),mode,digest,'worker'],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            own=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete original final offline owner')
            c.need(source(digest)==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==image,'whole full-lane final source/docs/ref/image');result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result);print(json.dumps(dict(mode=mode,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    mode,digest=sys.argv[1:3]
    if len(sys.argv)==4:
        c.need(sys.argv[3]=='worker' and mode in MODES,'fixed worker');_,env,_=s.environment('194');os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),commands(digest)[mode],str(ROOT),s.capacity,bound=1200);c.write(FINAL/mode,'execution.json',own);raise SystemExit(own['exit'])
    main(mode,digest)
