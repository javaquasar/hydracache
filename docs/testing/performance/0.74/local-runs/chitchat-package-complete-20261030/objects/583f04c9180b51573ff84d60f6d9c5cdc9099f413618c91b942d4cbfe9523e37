"""Fixed original owner for preflight controls, separate from any library run."""
import json,os,subprocess,sys,traceback
import context as s
c=s.c
BASE=s.ROOT/'target/chitchat-package-complete-controls-20261030'
COMMANDS={
 'controls':[sys.executable,'-B',str(s.HERE/'test_context.py')],
 'syntax':[sys.executable,'-B','-c',"import ast,pathlib;[ast.parse(p.read_bytes(),filename=str(p)) for p in pathlib.Path('tools/chitchat-package-complete-074').glob('*.py')]"]}

def source():
    values=s.prior.source()
    for p in s.HERE.iterdir():
        if p.is_file():values[p.relative_to(s.ROOT).as_posix()]=c.sha(p.read_bytes())
    return values

def main(mode):
    c.need(mode in COMMANDS,'fixed preflight control');out=BASE/mode;out.mkdir(parents=True,exist_ok=False)
    before=source();head=c.snapshot();argv=COMMANDS[mode]
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,image_sha256=c.sha(s.Path(argv[0]).read_bytes()),bound=1200,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,release_admitted=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(s.Path(__file__)),mode,'worker'],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            own=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete original preflight owner')
            c.need(source()==before and c.snapshot()==head,'whole source/ref postflight');result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result);print(json.dumps(dict(mode=mode,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    mode=sys.argv[1]
    if len(sys.argv)==3:
        c.need(sys.argv[2]=='worker' and mode in COMMANDS,'fixed control worker');_,env,_=s.environment('194');os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),COMMANDS[mode],str(s.ROOT),s.capacity,bound=1200);c.write(BASE/mode,'execution.json',own);raise SystemExit(own['exit'])
    main(mode)
