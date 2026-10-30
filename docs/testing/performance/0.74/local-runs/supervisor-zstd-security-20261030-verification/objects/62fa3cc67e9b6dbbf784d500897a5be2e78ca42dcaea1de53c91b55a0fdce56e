"""Fixed owned offline checks; no product or server execution authority."""
import json
import os
from pathlib import Path
import subprocess
import sys
import traceback
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'supervisor-zstd-security-074'))
import common as c

HERE=Path(__file__).resolve().parent
COMMANDS={
 'controls':[sys.executable,'-B','-m','unittest','discover','-s',str(HERE),'-p','test_*.py','-v'],
 'history-controls':[sys.executable,'-B',str(HERE.parent/'supervisor-zstd-security-074/test_review.py')],
 'byte-verify':[sys.executable,'-B',str(HERE/'verify.py')],
 'syntax':[sys.executable,'-B','-c',"import ast,pathlib; [ast.parse(p.read_bytes(),filename=str(p)) for name in ('supervisor-zstd-security-074','supervisor-zstd-policy-074') for p in (pathlib.Path('tools')/name).glob('*.py')]"],
 'doc-check':[str(c.ROOT/'target/debug/xtask.exe'),'doc-check'],
 'performance-contract':[str(c.ROOT/'target/debug/xtask.exe'),'performance-contract-check','--release','0.74'],
}
DOCS=('docs/articles/007-optimization-starts-with-measurement.md','docs/testing/performance/0.74/README.md',
 'docs/testing/performance/0.74/release-closure-execution-plan-20261026.md',
 'docs/testing/performance/0.74/supervisor-zstd-security-result-20261030.md',
 'docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md')

def source():
    values=c.source()
    for p in HERE.iterdir():
        if p.is_file():values[p.relative_to(c.ROOT).as_posix()]=c.sha(p.read_bytes())
    for n in DOCS:values[n]=c.sha((c.ROOT/n).read_bytes())
    return values

def main(mode):
    c.need(mode in COMMANDS,'fixed offline modes');argv=COMMANDS[mode]
    out=c.BASE/'final'/mode;out.mkdir(parents=True,exist_ok=False)
    before=source();head=c.snapshot();image=c.sha(Path(argv[0]).read_bytes())
    if mode in ('doc-check','performance-contract'):
        c.need(image=='e49f4694c8dd18629d33cdeda08d7f2a617ccae14cc3be41f10b52ad233471a9','same original owned xtask image as sealed r3 verification')
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,image_sha256=image,bound=1200,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,release_admitted=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),mode,'worker'],cwd=c.ROOT,
              stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            owner=json.loads((out/'execution.json').read_bytes())
            c.need(child.returncode==owner['exit']==0 and owner['retired'] and not owner['errors'],'complete original offline ownership')
            c.need(source()==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==image,'whole source/docs/ref/image postflight')
            result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result);print(json.dumps(dict(mode=mode,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    c.need(os.name=='nt' and len(sys.argv) in (2,3) and sys.argv[1] in COMMANDS,'fixed Windows offline producer')
    if len(sys.argv)==3:
        c.need(sys.argv[2]=='worker','fixed worker');_,env,_=c.environment('194');os.environ.clear();os.environ.update(env)
        result=c.owned._owned(c.owned.load_job().WindowsJob(),COMMANDS[sys.argv[1]],str(c.ROOT),c.capacity,bound=1200)
        c.write(c.BASE/'final'/sys.argv[1],'execution.json',result);raise SystemExit(result['exit'])
    main(sys.argv[1])
