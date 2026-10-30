"""Original full deny policy on each isolated dependency step, no exceptions."""
import json
import os
from pathlib import Path
import subprocess
import sys
import traceback
import scope as c

BASE=c.BASE/'policy'
DB=BASE/'advisory-dbs/advisory-db-3157b0e258782691'
EXPECTED={'B':['RUSTSEC-2026-0253','RUSTSEC-2026-0391','RUSTSEC-2026-0398','RUSTSEC-2026-0403'],
          'Z':['RUSTSEC-2026-0253'],'U':[]}

def database():
    c.need(not subprocess.check_output(['git','-C',str(DB),'status','--porcelain']),'clean actual advisory snapshot')
    names=subprocess.check_output(['git','-C',str(DB),'ls-files','-z']).split(b'\0')
    return dict(commit=subprocess.check_output(['git','-C',str(DB),'rev-parse','HEAD']).decode().strip(),
      files={n.decode():c.sha((DB/n.decode()).read_bytes()) for n in names if n})

def configuration():
    body=(c.ROOT/'deny.toml').read_bytes();c.need(c.sha(body)==c.PINS['deny.toml'],'exact Root deny')
    return body.replace(b'[advisories]',b'[advisories]\ndb-path="'+(BASE/'advisory-dbs').as_posix().encode()+b'"',1)

def command(variant):
    c.need(variant in EXPECTED,'three fixed variants');_,env,_=c.environment('194')
    return [str(Path.home()/'.cargo/bin/cargo-deny.exe'),'--manifest-path',str(c.BASE/'fixtures'/variant/'Cargo.toml'),
      '--all-features','--locked','--offline','--format','json','check','--config',str(BASE/'deny-location-only.toml'),'--disable-fetch','all'],env

def main(variant):
    out=BASE/variant;out.mkdir(exist_ok=False);before=c.source();head=c.snapshot();db=database()
    frozen=BASE/'snapshot.json'
    if not frozen.exists():c.write(BASE,'snapshot.json',db)
    c.need(json.loads(frozen.read_bytes())==db,'same complete frozen DB for all variants')
    config=BASE/'deny-location-only.toml'
    if not config.exists():config.write_bytes(configuration())
    c.need(config.read_bytes()==configuration(),'only advisory storage location changes')
    argv,env=command(variant);c.write(out,'intent.json',dict(argv=argv,source=before,git=head,database_sha256=c.sha(frozen.read_bytes()),config_sha256=c.sha(config.read_bytes()),bound=1200,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,release_admitted=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),variant,'worker'],stdin=subprocess.DEVNULL,
              stdout=stdout,stderr=stderr,env=env,timeout=1210)
            owned=json.loads((out/'execution.json').read_bytes());expected_exit=1 if EXPECTED[variant] else 0
            c.need(child.returncode==owned['exit']==expected_exit and owned['retired'] and not owned['errors'],'exact original owned dependency verdict')
            errors=[]
            for line in (out/'stderr.log').read_text(encoding='utf8').splitlines():
                row=json.loads(line).get('fields',{})
                if row.get('severity')=='error':errors.append(row)
            c.need(all('advisory' in e for e in errors) and sorted(e['advisory']['id'] for e in errors)==EXPECTED[variant],'only the exact expected advisories; other deny checks must pass')
            c.need(c.source()==before and c.snapshot()==head and database()==db and config.read_bytes()==configuration(),'whole Root/source/ref/DB/policy postflight')
            result.update(complete=True,policy_passed=not errors,errors=EXPECTED[variant])
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result);print(json.dumps(dict(variant=variant,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    c.need(os.name=='nt' and len(sys.argv) in (2,3),'fixed local policy producer')
    if len(sys.argv)==3:
        c.need(sys.argv[2]=='worker','fixed worker');argv,env=command(sys.argv[1]);os.environ.clear();os.environ.update(env)
        owned=c.owned._owned(c.owned.load_job().WindowsJob(),argv,str(c.ROOT),c.capacity,bound=1200)
        c.write(BASE/sys.argv[1],'execution.json',owned);raise SystemExit(owned['exit'])
    main(sys.argv[1])
