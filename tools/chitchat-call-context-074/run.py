"""Windows C2 controls; fixed commands, independent targets and first outcomes."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import traceback
import recipe as p
c=p.c
LEGACY='serialize::borrowed_context_control::retains_context_for_its_writer'
PURE=('serialize::tests::','delta::tests::','message::tests::','digest::tests::','state::tests::',
 'security_review::','serialize::borrowed_context_control::buffer_error','serialize::borrowed_context_control::independent_writers')
OBS=('serialize::security_stream_review::observe_streams','delta::security_delta_review::supported_delta_operations')


def parts(mode):
    fields=mode.split('-');c.need(len(fields) in (2,3),'fixed compiler suffix')
    action,v=fields[:2];compiler=fields[2] if len(fields)==3 else '194'
    c.need(v in ('B0','T1','C2') and compiler in ('194','188'),'fixed inputs/compilers')
    return action,v,compiler


def label(action,v,compiler):return action+'-'+v+('-188' if compiler=='188' else '')


def source():
    value=c.source()
    for path in p.HERE.iterdir():
        if path.is_file():value[path.relative_to(c.ROOT).as_posix()]=c.sha(path.read_bytes())
    for path in (p.BASE/'fixtures').rglob('*'):
        c.need(not path.is_symlink() and not getattr(path,'is_junction',lambda:False)(),'regular actual input')
        if path.is_file():value[path.relative_to(c.ROOT).as_posix()]=c.sha(path.read_bytes())
    return value


def image(v,compiler):
    rows=[json.loads(s) for s in (p.BASE/label('build',v,compiler)/'stdout.log').read_text().splitlines() if s.startswith('{')]
    found=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('executable') and r['profile']['test'] and r['target']['kind']==['lib']]
    c.need([r['success'] for r in rows if r.get('reason')=='build-finished']==[True] and len(found)==1 and found[0]['fresh'] is False,'one fresh actual build')
    path=Path(found[0]['executable']);c.need(Path(found[0]['target']['src_path']).resolve()==(p.BASE/'fixtures'/v/'src/lib.rs').resolve(),'actual source identity')
    c.need(path.resolve().is_relative_to((p.BASE/'builds'/label('windows',v,compiler)).resolve()),'independent target identity')
    return path


def command(mode):
    action,v,compiler=parts(mode);tool,env,_=c.environment(compiler)
    args=['--manifest-path',str(p.BASE/'fixtures'/v/'Cargo.toml'),'--target-dir',str(p.BASE/'builds'/label('windows',v,compiler)),
      '--offline','--locked','--release','--all-features']
    if action=='build':argv=[str(tool/'cargo.exe'),'test',*args,'--lib','--no-run','--message-format=json']
    elif action in ('check','clippy'):
        argv=[str(tool/'cargo.exe'),action,*args,'--all-targets']
        if action=='clippy':argv+=['--','-D','warnings']
    elif action=='catalog':argv=[str(image(v,compiler)),'--list']
    elif action in ('control','panic','legacy'):
        test={'control':p.BOUNDARY,'panic':p.PANIC,'legacy':LEGACY}[action]
        argv=[str(image(v,compiler)),'--exact',test,'--test-threads=1','--show-output']
    elif action=='compat':
        filters=(*PURE,*OBS)+(('serialize::call_context_review::',) if v=='C2' else ())
        argv=[str(image(v,compiler)),*filters,'--test-threads=1','--show-output']
    else:raise ValueError('fixed actions')
    return argv,env,5 if action in ('control','panic','legacy') else 10 if action=='catalog' else 1200


def negative(mode):
    action,v,_=parts(mode)
    return action in ('control','panic') and v=='T1' or action=='legacy' and v in ('B0','C2')


def accounting(mode):
    action,v,compiler=parts(mode)
    if action not in ('control','panic','legacy','compat','catalog'):return {}
    raw=(p.BASE/mode/'stdout.log').read_text(encoding='utf8').replace('\r\n','\n')
    if action=='catalog':
        names=sorted(s[:-6] for s in raw.splitlines() if s.endswith(': test'))
        c.need(len(names)==len(set(names))==116 and p.BOUNDARY in names and p.PANIC in names and LEGACY in names and raw.splitlines()[-1]=='116 tests, 0 benchmarks','complete unchanged catalog plus seven new controls')
        return dict(catalog=names)
    catalog=json.loads((p.BASE/label('catalog',v,compiler)/'result.json').read_bytes())['catalog']
    if action=='compat':expected=[n for n in catalog if n.startswith(PURE) or n in OBS or v=='C2' and n.startswith('serialize::call_context_review::')]
    else:expected=[{'control':p.BOUNDARY,'panic':p.PANIC,'legacy':LEGACY}[action]]
    is_negative=negative(mode);rows=re.findall(r'^test (.+?) (?:- should panic )?\.\.\. (ok|FAILED)$',raw,re.M)
    c.need(sorted(n for n,_ in rows)==sorted(expected) and all(result==('FAILED' if is_negative else 'ok') for _,result in rows),'exact selected outcome identities')
    summaries=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',raw,re.M)
    c.need(summaries==[('FAILED' if is_negative else 'ok','0' if is_negative else str(len(expected)),'1' if is_negative else '0','0','0',str(len(catalog)-len(expected)))],'complete actual outcome counts')
    return dict(passed=0 if is_negative else len(expected),failed=1 if is_negative else 0,expected_failure=is_negative,selected=expected)


def main(mode):
    out=p.BASE/mode;out.mkdir(exist_ok=False);before=source();head=c.snapshot();argv,env,bound=command(mode)
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,image_sha256=c.sha(Path(argv[0]).read_bytes()),
      bound=bound,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,mtu_repaired=False,resources_qualified=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),mode,'worker'],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=env,timeout=bound+10)
            own=json.loads((out/'execution.json').read_bytes());expected=101 if negative(mode) else 0
            c.need(child.returncode==own['exit']==expected and own['retired'] and not own['errors'],'complete original owned outcome')
            c.need(source()==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==json.loads((out/'intent.json').read_bytes())['image_sha256'],'whole source/ref/image postflight')
            result.update(accounting(mode));result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result);print(json.dumps(dict(mode=mode,**{k:v for k,v in result.items() if k not in ('catalog','selected')})))
    raise SystemExit(0 if result['complete'] else 1)


if __name__=='__main__':
    c.need(os.name=='nt' and len(sys.argv) in (2,3),'fixed Windows controls')
    if len(sys.argv)==3:
        c.need(sys.argv[2]=='worker','fixed worker');argv,env,bound=command(sys.argv[1]);os.environ.clear();os.environ.update(env)
        _,v,_=parts(sys.argv[1]);own=c.owned._owned(c.owned.load_job().WindowsJob(),argv,str(p.BASE/'fixtures'/v),c.capacity,bound=bound)
        c.write(p.BASE/sys.argv[1],'execution.json',own);raise SystemExit(own['exit'])
    main(sys.argv[1])
