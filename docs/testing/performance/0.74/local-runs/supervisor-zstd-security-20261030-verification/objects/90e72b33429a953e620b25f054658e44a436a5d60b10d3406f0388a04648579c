"""Fresh dependency-only review; it cannot skip a failed workspace stage."""
import json
import os
from pathlib import Path
import subprocess
import sys
import traceback

HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'supervisor-zstd-security-074'))
import common as c

BASE=c.BASE/'policy'
DB=BASE/'advisory-dbs/advisory-db-3157b0e258782691'
DB_COMMIT='ee1a49a2769a24a596afc1c20cb33748d2769586'

def custody():
    c.need(subprocess.check_output(['git','-C',str(DB),'rev-parse','HEAD']).decode().strip()==DB_COMMIT,'fresh exact advisory snapshot')
    c.need(not subprocess.check_output(['git','-C',str(DB),'status','--porcelain']),'unchanged advisory tree')
    names=subprocess.check_output(['git','-C',str(DB),'ls-files','-z']).split(b'\0')
    return {n.decode():c.sha((DB/n.decode()).read_bytes()) for n in names if n}

def configuration():
    raw=(c.ROOT/'deny.toml').read_bytes()
    c.need(c.sha(raw)==c.PINS['deny.toml'] and raw.count(b'[advisories]')==1,'unchanged complete Root policy')
    return raw.replace(b'[advisories]',b'[advisories]\ndb-path = "'+(BASE/'advisory-dbs').as_posix().encode()+b'"',1)

def command(scope):
    c.need(scope in ('supervisor','root'),'two fixed scopes only')
    tool,env,suffix=c.environment('194')
    binary=Path.home()/'.cargo/bin/cargo-deny.exe'
    argv=[str(binary),'--manifest-path',str(c.ROOT/(c.PKG+'/Cargo.toml' if scope=='supervisor' else 'Cargo.toml')),
          '--all-features','--locked','--offline','--format','json']
    if scope=='root':argv+=['--workspace']
    return argv+['check','--config',str(BASE/'deny-location-only.toml'),'--disable-fetch','advisories'],env

def main(scope):
    out=BASE/scope;out.mkdir(exist_ok=False)
    argv,env=command(scope);before=c.source();head=c.snapshot();db=custody()
    config=BASE/'deny-location-only.toml'
    if not config.exists():config.write_bytes(configuration())
    c.need(config.read_bytes()==configuration(),'only advisory storage path differs')
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,database_commit=DB_COMMIT,database_files=db,
      executable_sha256=c.sha(Path(argv[0]).read_bytes()),producer_sha256=c.sha(Path(__file__).read_bytes()),
      config_sha256=c.sha(config.read_bytes()),bound_seconds=1200,floor_bytes=4*1024**3,poll_seconds=5,cleanup_seconds=5,retry=False))
    result=dict(complete=False,workspace_gate_passed=False,release_admitted=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),scope,'worker'],cwd=c.ROOT,
              stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            owned=json.loads((out/'execution.json').read_bytes())
            c.need(child.returncode==owned['exit'] and owned['retired'] and not owned['errors'],'complete original Job retirement')
            c.need(owned['exit'] in (0,1),'an actual dependency verdict, not infrastructure refusal')
            c.need(c.source()==before and c.snapshot()==head and custody()==db and config.read_bytes()==configuration(),'whole source/Root/ref/policy/advisory postflight')
            result.update(complete=True,policy_passed=owned['exit']==0,exit=owned['exit'])
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result);print(json.dumps(dict(scope=scope,**result)))
    raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    c.need(os.name=='nt' and len(sys.argv) in (2,3),'fixed local Windows policy producer')
    if len(sys.argv)==3:
        c.need(sys.argv[2]=='worker','fixed worker')
        argv,env=command(sys.argv[1]);os.environ.clear();os.environ.update(env)
        result=c.owned._owned(c.owned.load_job().WindowsJob(),argv,str(c.ROOT),c.capacity,bound=1200)
        c.write(BASE/sys.argv[1],'execution.json',result);raise SystemExit(result['exit'])
    main(sys.argv[1])
