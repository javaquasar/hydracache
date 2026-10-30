"""Offline final gates only. Product and resource execution stays sealed."""
import ast
import json
import os
from pathlib import Path
import subprocess
import sys
import traceback
import archive as a
import material as m
c=m.c
SHA='e554bd8ee5bd7b4f0cc2963393ccbea922e93663f294962446d1caa651034403'
DOCS=('docs/articles/007-optimization-starts-with-measurement.md','docs/testing/performance/0.74/README.md',
 'docs/testing/performance/0.74/release-closure-execution-plan-20261026.md',
 'docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md',
 'docs/testing/performance/0.74/chitchat-fused-result-20261030.md','docs/testing/performance/0.74/chitchat-mtu-next-candidate-plan-20261030.md')
FMT=Path(os.environ['USERPROFILE'])/'.rustup/toolchains/1.94.0-x86_64-pc-windows-msvc/bin/rustfmt.exe'
COMMANDS={
 'controls':[sys.executable,'-B',str(a.HERE/'test_review.py')],
 'syntax':[sys.executable,'-B','-c',"import ast,pathlib; [ast.parse(p.read_bytes(),filename=str(p)) for n in ('chitchat-fused-074','chitchat-fused-result-074') for p in (pathlib.Path('tools')/n).glob('*.py')]"],
 'format':[str(FMT),'--edition','2024','--config','skip_children=true','--check',
   *[str(p) for folder in (m.p.HERE,a.HERE) for p in sorted(folder.glob('*.rs'))]],
 'byte-verify':[sys.executable,'-B',str(a.HERE/'archive.py'),'verify','--sha',SHA],
 'restore-verify':[sys.executable,'-B',str(a.HERE/'archive.py'),'restore','--sha',SHA,'--destination',str(c.ROOT/'target/chitchat-fused-source-review-20261030')],
 'doc-check':[str(c.ROOT/'target/debug/xtask.exe'),'doc-check'],
 'performance-contract':[str(c.ROOT/'target/debug/xtask.exe'),'performance-contract-check','--release','0.74']}

def source():
    value=c.source()
    for folder in (m.p.HERE,a.HERE):
        for path in folder.iterdir():
            if path.is_file():value[path.relative_to(c.ROOT).as_posix()]=c.sha(path.read_bytes())
    for name in DOCS:value[name]=c.sha((c.ROOT/name).read_bytes())
    raw=(a.PACKET/'index.json').read_bytes();c.need(c.sha(raw)==SHA,'unchanged declined primary index')
    value[(a.PACKET/'index.json').relative_to(c.ROOT).as_posix()]=SHA
    c.need((a.BASE/'STOP.json').is_file(),'execution seal remains')
    return value

COMMANDS['seal-controls']=COMMANDS['controls']
COMMANDS['seal-syntax']=COMMANDS['syntax']

def main(mode):
    c.need(mode in COMMANDS,'fixed offline commands');out=a.BASE/'final'/mode;out.mkdir(parents=True,exist_ok=False)
    argv=COMMANDS[mode];before=source();head=c.snapshot();image=c.sha(Path(argv[0]).read_bytes())
    if mode in ('doc-check','performance-contract'):c.need(image=='e49f4694c8dd18629d33cdeda08d7f2a617ccae14cc3be41f10b52ad233471a9','same held qualified xtask')
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,image_sha256=image,bound=1200,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,release_admitted=False,product_execution=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),mode,'worker'],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            own=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete offline owned outcome')
            c.need(source()==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==image,'whole source/docs/ref/image postflight')
            result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result);print(json.dumps(dict(mode=mode,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    if len(sys.argv)==3:
        mode=sys.argv[1];c.need(sys.argv[2]=='worker' and mode in COMMANDS,'fixed offline worker');_,env,_=c.environment('194');os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),COMMANDS[mode],str(c.ROOT),c.capacity,bound=1200)
        c.write(a.BASE/'final'/mode,'execution.json',own);raise SystemExit(own['exit'])
    main(sys.argv[1])
