"""Small fixed context-lifetime step; same CI owner and no timing claims."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import traceback
import recipe as p
c=p.c

TEST='serialize::borrowed_context_control::retains_context_for_its_writer'

def source():
    values=c.source()
    for path in p.HERE.iterdir():
        if path.is_file():values[path.relative_to(c.ROOT).as_posix()]=c.sha(path.read_bytes())
    for path in (p.BASE/'fixtures').rglob('*'):
        c.need(not path.is_symlink(),'regular context input')
        if path.is_file():values[path.relative_to(c.ROOT).as_posix()]=c.sha(path.read_bytes())
    return values

def image(variant):
    rows=[json.loads(s) for s in (p.BASE/('build-'+variant)/'stdout.log').read_text().splitlines() if s.startswith('{')]
    found=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('executable') and r['profile']['test'] and r['target']['kind']==['lib']]
    c.need([r['success'] for r in rows if r.get('reason')=='build-finished']==[True] and len(found)==1,'actual complete library build')
    c.need(Path(found[0]['target']['src_path']).resolve()==(p.BASE/'fixtures'/variant/'src/lib.rs').resolve(),'exact actual source artifact')
    return Path(found[0]['executable'])

def command(mode):
    action,variant=mode.rsplit('-',1);c.need(variant in ('B0','C1'),'two isolated source inputs')
    tool,env,suffix=c.environment('194');base=['--manifest-path',str(p.BASE/'fixtures'/variant/'Cargo.toml'),'--target-dir',str(p.BASE/'builds/windows-194'),'--locked','--offline','--release','--all-features']
    if action=='build':argv=[str(tool/'cargo.exe'),'test',*base,'--lib','--no-run','--message-format=json']
    elif action in ('check','clippy'):
        argv=[str(tool/'cargo.exe'),action,*base,'--all-targets']
        if action=='clippy':argv+=['--','-D','warnings']
    elif action=='catalog':argv=[str(image(variant)),'--list']
    elif action=='control':argv=[str(image(variant)),'--exact',TEST,'--test-threads=1','--show-output']
    elif action=='compat':
        argv=[str(image(variant)),*('serialize::tests::','delta::tests::','message::tests::','digest::tests::','state::tests::',
          'serialize::security_stream_review::observe_streams','delta::security_delta_review::supported_delta_operations',
          'security_review::','serialize::borrowed_context_control::buffer_error','serialize::borrowed_context_control::independent_writers'),
          '--test-threads=1','--show-output']
    else:raise ValueError('fixed context commands')
    return argv,env,5 if action=='control' else 10 if action=='catalog' else 1200

def accounting(mode):
    action,variant=mode.rsplit('-',1)
    if action not in ('control','compat','catalog'):return {}
    raw=(p.BASE/mode/'stdout.log').read_text(encoding='utf8').replace('\r\n','\n')
    if action=='catalog':
        names=sorted(s[:-6] for s in raw.splitlines() if s.endswith(': test'))
        c.need(len(names)==len(set(names))==109 and TEST in names and raw.splitlines()[-1]=='109 tests, 0 benchmarks','all original cases plus exactly three context controls')
        return dict(catalog=names)
    catalog=json.loads((p.BASE/('catalog-'+variant)/'result.json').read_bytes())['catalog']
    expected=[TEST] if action=='control' else [n for n in catalog if n.startswith(('serialize::tests::','delta::tests::','message::tests::','digest::tests::','state::tests::','security_review::','serialize::borrowed_context_control::buffer_error','serialize::borrowed_context_control::independent_writers')) or n in ('serialize::security_stream_review::observe_streams','delta::security_delta_review::supported_delta_operations')]
    rows=re.findall(r'^test (.+?) (?:- should panic )?\.\.\. (ok|FAILED)$',raw,re.M)
    negative=mode=='control-B0'
    c.need(sorted(n for n,_ in rows)==sorted(expected) and all(v==('FAILED' if negative else 'ok') for _,v in rows),'all exact selected case outcomes')
    summaries=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',raw,re.M)
    c.need(summaries==[('FAILED' if negative else 'ok','0' if negative else str(len(expected)),'1' if negative else '0','0','0',str(len(catalog)-len(expected)))],'complete native counts')
    return dict(passed=0 if negative else len(expected),failed=1 if negative else 0,expected_failure=negative,selected=expected,mtu_repaired=False)

def main(mode):
    out=p.BASE/mode;out.mkdir(exist_ok=False);before=source();head=c.snapshot();argv,env,bound=command(mode)
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,bound=bound,floor=4*1024**3,poll=5,cleanup=5,image_sha256=c.sha(Path(argv[0]).read_bytes()),retry=False))
    result=dict(complete=False,root_adopted=False,mtu_repaired=False,resources_qualified=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),mode,'worker'],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=env,timeout=bound+10)
            owner=json.loads((out/'execution.json').read_bytes());expected=101 if mode=='control-B0' else 0
            c.need(child.returncode==owner['exit']==expected and owner['retired'] and not owner['errors'],'complete original owned context step')
            c.need(source()==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==json.loads((out/'intent.json').read_bytes())['image_sha256'],'whole source/ref/image postflight')
            result.update(accounting(mode));result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result)
    print(json.dumps(dict(mode=mode,**{k:v for k,v in result.items() if k not in ('catalog','selected')})))
    raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    c.need(os.name=='nt' and len(sys.argv) in (2,3),'fixed local Windows context step')
    if len(sys.argv)==3:
        c.need(sys.argv[2]=='worker','fixed worker');argv,env,bound=command(sys.argv[1]);os.environ.clear();os.environ.update(env)
        owner=c.owned._owned(c.owned.load_job().WindowsJob(),argv,str(p.BASE/'fixtures'/sys.argv[1].rsplit('-',1)[-1]),c.capacity,bound=bound)
        c.write(p.BASE/sys.argv[1],'execution.json',owner);raise SystemExit(owner['exit'])
    main(sys.argv[1])
