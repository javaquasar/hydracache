"""Fixed original owners for source controls and one public read-only observation."""
import json,os,subprocess,sys,traceback
from pathlib import Path
import contract as s
c=s.c
FMT=Path(os.environ['USERPROFILE'])/'.rustup/toolchains/1.94.0-x86_64-pc-windows-msvc/bin/rustfmt.exe'
COMMANDS={
 'controls':[sys.executable,'-B',str(s.HERE/'test_contract.py')],
 'syntax':[sys.executable,'-B','-c',"import ast,pathlib;[ast.parse(p.read_bytes(),filename=str(p)) for p in pathlib.Path('tools/chitchat-public-identity-074').glob('*.py')]"],
 'format':[str(FMT),'--edition','2024','--check',*[str(s.HERE/(v+'.rs')) for v in s.VARIANTS]],
 'registry-observation':[sys.executable,'-B',str(s.HERE/'observation.py')]}

def main(mode):
    c.need(mode in COMMANDS,'fixed offline command');out=s.BASE/'offline'/mode;out.mkdir(parents=True,exist_ok=False)
    before=s.source();head=c.snapshot();argv=COMMANDS[mode];image=c.sha(Path(argv[0]).read_bytes())
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,image_sha256=image,bound=1200,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,release_admitted=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),mode,'worker'],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            own=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete original nominal offline owner')
            c.need(s.source()==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==image,'whole offline source/ref/image postflight');result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result);print(json.dumps(dict(mode=mode,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    mode=sys.argv[1]
    if len(sys.argv)==3:
        c.need(sys.argv[2]=='worker' and mode in COMMANDS,'fixed original offline worker');_,env,_=s.environment();os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),COMMANDS[mode],str(s.ROOT),s.capacity,bound=1200);c.write(s.BASE/'offline'/mode,'execution.json',own);raise SystemExit(own['exit'])
    main(mode)
