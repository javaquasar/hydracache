"""Fixed current caller controls under the original owned Windows Job."""
import json,os,subprocess,sys,traceback
from pathlib import Path
import caller_context as s
c=s.c
COMMANDS={
 'controls':[sys.executable,'-B',str(s.HERE/'test_caller.py')],
 'controls-download':[sys.executable,'-B',str(s.HERE/'test_caller.py')],
 'syntax-download':[sys.executable,'-B','-c',"import ast,pathlib;[ast.parse(p.read_bytes(),filename=str(p)) for p in pathlib.Path('tools/chitchat-copied-root-caller-074').glob('*.py')]"],
 'syntax':[sys.executable,'-B','-c',"import ast,pathlib;[ast.parse(p.read_bytes(),filename=str(p)) for p in pathlib.Path('tools/chitchat-copied-root-caller-074').glob('*.py')]"]}

def main(mode):
    c.need(mode in COMMANDS,'fixed actual caller offline mode');out=s.BASE/'offline'/mode;out.mkdir(parents=True,exist_ok=False)
    argv=COMMANDS[mode];before=s.source();head=c.snapshot();image=c.sha(Path(argv[0]).read_bytes())
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,image_sha256=image,bound=1200,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,release_admitted=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),mode,'worker'],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            own=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete original caller offline owner')
            c.need(s.source()==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==image,'whole current caller source/ref/image postflight');result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result);print(json.dumps(dict(mode=mode,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    mode=sys.argv[1]
    if len(sys.argv)==3:
        c.need(sys.argv[2]=='worker' and mode in COMMANDS,'fixed current-source worker');_,env,_=s.environment('194');os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),COMMANDS[mode],str(s.ROOT),s.parent.capacity,bound=1200);c.write(s.BASE/'offline'/mode,'execution.json',own);raise SystemExit(own['exit'])
    main(mode)
