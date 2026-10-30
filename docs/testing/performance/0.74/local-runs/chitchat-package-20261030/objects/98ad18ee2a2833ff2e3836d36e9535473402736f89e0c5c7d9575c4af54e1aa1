"""Exact new RustSec cut; same complete policy under owned and upstream names."""
import json,os,shutil,subprocess,sys,traceback,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'tools/chitchat-package-r3-074'));import common as s
m=s.m;c=s.c;BASE=m.BASE/'policy';DB=BASE/'advisory-dbs/advisory-db-3157b0e258782691'
DB_SHA='2295466c4e9df26647fd67d8fa1c705ac2fc7f81'

def source():
    values=s.source()
    for p in HERE.iterdir():
        if p.is_file():values[p.relative_to(ROOT).as_posix()]=c.sha(p.read_bytes())
    return values

def database():
    def git(*args):return subprocess.check_output(['git','-C',str(DB),*args],cwd=ROOT)
    c.need(git('rev-parse','HEAD').decode().strip()==DB_SHA and not git('status','--porcelain'),'exact clean fresh snapshot')
    values={};codec=m.codec()
    for name in git('ls-files','-z').split(b'\0'):
        if not name:continue
        n=name.decode();codec.safe_name(n);p=DB/n
        c.need(p.is_file() and not p.is_symlink() and not p.is_junction() and p.resolve().is_relative_to(DB.resolve()),'regular contained advisory source')
        values[n]=c.sha(p.read_bytes())
    return dict(commit=DB_SHA,files=values)

def config():
    body=(ROOT/'deny.toml').read_bytes();c.need(c.sha(body)==c.PINS['deny.toml'],'same full Root deny policy')
    return body.replace(b'[advisories]',b'[advisories]\ndb-path="'+(BASE/'advisory-dbs').as_posix().encode()+b'"',1)

def command(mode):
    _,env,_=s.environment();env['CARGO_HOME']=str(BASE/'cargo-home')
    git=shutil.which('git',path=env['PATH']);c.need(git,'actual Git executable')
    if mode=='init':argv=[git,'init','--template=','--initial-branch=main',str(DB)]
    elif mode=='fetch':argv=[git,'-c','core.hooksPath=NUL','-C',str(DB),'fetch','--depth=1','https://github.com/RustSec/advisory-db.git',DB_SHA]
    elif mode=='checkout':argv=[git,'-c','core.hooksPath=NUL','-C',str(DB),'checkout','--detach',DB_SHA]
    else:
        c.need(mode in ('owned','upstream'),'fixed exact source identities')
        argv=[str(Path.home()/'.cargo/bin/cargo-deny.exe'),'--manifest-path',str(BASE/mode/'Cargo.toml'),'--all-features','--locked','--offline','--format','json','check','--config',str(BASE/'deny-location-only.toml'),'--disable-fetch','all']
    return argv,env

def prepare():
    BASE.mkdir(exist_ok=True);c.need(not (BASE/'prepare.json').exists(),'exclusive policy source context')
    _,u,lock=m.parent();owned=m.project(u,lock);owned['Cargo.lock']=(m.BASE/'state/resolved.lock').read_bytes();m.verify_source(owned,u)
    upstream=dict(u);upstream['LICENSE']=m.license_bytes();upstream['Cargo.lock']=lock
    upstream['Cargo.toml']=u['Cargo.toml'].replace(b'resolver = "2"\n',b'')+b'\n[dev-dependencies.serde_json]\nversion="1"\n[workspace]\nresolver="2"\n'
    c.need(m.registry_nodes(owned['Cargo.lock'])==m.registry_nodes(upstream['Cargo.lock']),'whole registry graph independent of fork root identity')
    for label,files in [('owned',owned),('upstream',upstream)]:
        folder=BASE/label
        for n,body in files.items():
            path=folder/n;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(body)
    ledger=json.loads((m.BASE/'input/registry.json').read_bytes());index=json.loads((m.PARENT/'index.json').read_bytes())
    c.need(c.sha((m.PARENT/'index.json').read_bytes())==m.PARENT_SHA,'whole accepted archive ledger')
    for name,digest in ledger.items():
        row=index['members']['capture/r4/input/archives/'+name+'.crate'];raw=(m.PARENT/'objects'/row['sha256']).read_bytes()
        c.need(c.sha(raw)==digest==row['sha256'],'whole immutable upstream archive')
        files=m.codec().unpack(raw,name);folder=BASE/'vendor'/name
        for n,body in files.items():
            path=folder/n;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(body)
        c.write(folder,'.cargo-checksum.json',dict(package=digest,files={n:c.sha(b) for n,b in files.items()}))
    (BASE/'cargo-home').mkdir();(BASE/'cargo-home/config.toml').write_text('[source.crates-io]\nreplace-with="reviewed-upstream"\n[source.reviewed-upstream]\ndirectory="'+(BASE/'vendor').as_posix()+'"\n',encoding='utf8')
    (BASE/'deny-location-only.toml').write_bytes(config())
    after=tomllib.loads(config().decode());after['advisories'].pop('db-path')
    c.need(after==tomllib.loads((ROOT/'deny.toml').read_text()),'only DB storage location differs')
    c.write(BASE,'prepare.json',dict(owned=c.tree(BASE/'owned'),upstream=c.tree(BASE/'upstream'),vendor=c.tree(BASE/'vendor'),
        config_sha256=c.sha(config()),cargo_config_sha256=c.sha((BASE/'cargo-home/config.toml').read_bytes()),
        root_adopted=False,registry_qualified=False))
    print(json.dumps(dict(complete=True,registry_packages=len(ledger),advisory_identity_mapping=['hydra-chitchat','chitchat'],root_adopted=False)))

def frame(mode):
    if mode in ('init','fetch','checkout'):return None
    spec=json.loads((BASE/'prepare.json').read_bytes())
    for name in ('owned','upstream','vendor'):c.need(c.tree(BASE/name)==spec[name],'whole actual policy source: '+name)
    c.need((BASE/'deny-location-only.toml').read_bytes()==config() and c.sha((BASE/'cargo-home/config.toml').read_bytes())==spec['cargo_config_sha256'],'same exact policy/source configuration')
    return database()

def main(mode):
    out=BASE/'commands'/mode;out.mkdir(parents=True,exist_ok=False);before=source();head=c.snapshot();db=frame(mode);argv,env=command(mode)
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,database=db,image_sha256=c.sha(Path(argv[0]).read_bytes()),bound=1200,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,registry_qualified=False,release_admitted=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),mode,'worker'],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=env,timeout=1210)
            own=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'first complete original policy owner')
            c.need(source()==before and c.snapshot()==head,'full source/ref policy postflight')
            if db is not None:
                c.need(frame(mode)==db,'whole exact source/DB postflight');errors=[]
                for line in (out/'stderr.log').read_text().splitlines():
                    try:row=json.loads(line).get('fields',{})
                    except json.JSONDecodeError:continue
                    if row.get('severity')=='error':errors.append(row)
                c.need(not errors,'no advisory/bans/license/source error');result['policy_passed']=True;result['database_commit']=DB_SHA
            result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result);print(json.dumps(dict(mode=mode,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    if sys.argv[1]=='prepare':prepare()
    elif len(sys.argv)==3:
        mode=sys.argv[1];c.need(sys.argv[2]=='worker','fixed owned policy worker');argv,env=command(mode);os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),argv,str(ROOT),s.capacity,bound=1200);c.write(BASE/'commands'/mode,'execution.json',own);raise SystemExit(own['exit'])
    else:main(sys.argv[1])
