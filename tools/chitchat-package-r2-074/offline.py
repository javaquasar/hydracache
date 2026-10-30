"""Fixed offline source/evidence gates, original Job bounds and first outputs."""
import json,os,subprocess,sys,traceback
from pathlib import Path
import common as s
m=s.m;c=s.c
FMT=Path(os.environ['USERPROFILE'])/'.rustup/toolchains/1.94.0-x86_64-pc-windows-msvc/bin/rustfmt.exe'
COMMANDS={
 'controls':[sys.executable,'-B',str(m.HERE/'test_model.py')],
 'syntax':[sys.executable,'-B','-c',"import ast,pathlib; [ast.parse(p.read_bytes(),filename=str(p)) for p in pathlib.Path('tools/chitchat-package-r2-074').glob('*.py')]"],
 'format':[str(FMT),'--edition','2024','--check',str(m.HERE/'consumer.rs')],
 'doc-check':[str(m.ROOT/'target/debug/xtask.exe'),'doc-check'],
 'performance-contract':[str(m.ROOT/'target/debug/xtask.exe'),'performance-contract-check','--release','0.74']}

def main(mode):
    c.need(mode in COMMANDS,'fixed offline mode');out=m.BASE/'offline'/mode;out.mkdir(parents=True,exist_ok=False)
    before=s.source();head=c.snapshot();argv=COMMANDS[mode];image=c.sha(Path(argv[0]).read_bytes())
    if mode in ('doc-check','performance-contract'):c.need(image=='e49f4694c8dd18629d33cdeda08d7f2a617ccae14cc3be41f10b52ad233471a9','same held xtask image')
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,image_sha256=image,bound=1200,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,release_admitted=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),mode,'worker'],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            own=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete original offline owner')
            c.need(s.source()==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==image,'whole offline source/ref/image postflight');result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result);print(json.dumps(dict(mode=mode,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    mode=sys.argv[1]
    if len(sys.argv)==3:
        c.need(sys.argv[2]=='worker' and mode in COMMANDS,'fixed offline worker');_,env,_=s.environment();os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),COMMANDS[mode],str(m.ROOT),s.capacity,bound=1200);c.write(m.BASE/'offline'/mode,'execution.json',own);raise SystemExit(own['exit'])
    main(mode)
