"""Separate approved full-library lane; R5 and its short-case STOP stay immutable."""
import json,os,shutil,sys,types
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'tools/chitchat-package-r5-074'));import common as prior
m=prior.m;c=prior.c;PARENT=m.BASE
BASE=ROOT/'target/chitchat-package-complete-20261030'
N=Path('/home/artur/.cache/hydracache-074-package/20261030-a6')
PACKET=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-package-20261030'
SHA='6200fd3fe30ae00532e30a5622573851a3dee15cb243f08051f0cd2ee6b22b47'
PHASES=('stage','library194','library188','registry','consumer194','consumer188')

def prerequisites(phase):
    c.need(phase in PHASES,'fixed full-lane phase')
    return {'stage':(), 'library194':('stage',),'library188':('stage',),
        'registry':('library194','library188'),'consumer194':('registry',),'consumer188':('registry',)}[phase]

def source():
    value=prior.source()
    for root in (HERE,BASE/'input',BASE/'artifact',BASE/'source'):
        for path in root.rglob('*'):
            if '__pycache__' in path.parts:continue
            c.need(not path.is_symlink() and not path.is_junction(),'regular full-lane input')
            if path.is_file():value[path.relative_to(ROOT).as_posix()]=c.sha(path.read_bytes())
    raw=(PACKET/'index.json').read_bytes();c.need(c.sha(raw)==SHA,'immutable first R5 packet')
    value[(PACKET/'index.json').relative_to(ROOT).as_posix()]=SHA
    for name,digest in json.loads((BASE/'input/parent-frame.json').read_bytes()).items():
        c.need(c.sha((ROOT/name).read_bytes())==digest,'unchanged accepted parent receipt/STOP')
        value[name]=digest
    return value

def capacity():
    prior.capacity()
    if os.name=='posix' and N.exists() and shutil.disk_usage(N).free<4*1024**3:raise c.owned.CapacityError('original full-lane native floor')

def environment(compiler):
    tool,env,suffix=prior.environment(compiler)
    if os.name=='posix':env['CARGO_HOME']=str(N/'cargo-home')
    return tool,env,suffix

def actual():
    prior.actual()
    for label in ('source',):
        frame=BASE/'state'/(label+'.json')
        if frame.exists():c.need(c.tree(N/label)==json.loads(frame.read_bytes()),'whole full-lane immutable source')
    frame=BASE/'state/cargo-config.toml.json'
    if frame.exists():c.need(c.sha((N/'cargo-home/config.toml').read_bytes())==json.loads(frame.read_bytes())['sha256'],'whole exact full-lane Cargo config')
    frame=BASE/'state/consumer.json'
    if frame.exists():
        row=json.loads(frame.read_bytes());c.need(c.tree(N/'consumer')==row['source'],'whole original independent caller')
        downloaded=c.tree(Path(row['downloaded_path']));downloaded.pop('.cargo-ok',None);downloaded.pop('.cargo-checksum.json',None)
        c.need(downloaded==row['downloaded'],'whole actual registry download')
        for name,digest in row['registry_files'].items():c.need(c.sha((N/name).read_bytes())==digest,'whole local-registry input/endpoint')

def full_outcome(raw,names):
    import re
    text=raw.decode();actual=re.findall(r'^test (.+?) (?:- should panic )?\.\.\. ',text,re.M)
    c.need(len(names)==len(set(names))==100 and len(actual)==100 and set(actual)==set(names),'exact complete100-case catalog, no filter/omission/duplicate')
    c.need('test result: ok. 100 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' in text,'all100 original outcomes')
    return dict(complete=True,passed=100,failed=0,ignored=0,filtered_out=0,full_library_bound=780)

def prepare():
    capacity();c.need(c.sha((PACKET/'index.json').read_bytes())==SHA,'exact first evidence ledger')
    for mode in ('controls','syntax'):
        folder=ROOT/'target/chitchat-package-complete-controls-20261030'/mode
        c.need(json.loads((folder/'result.json').read_bytes())['complete'],'positive separate full-lane producer control')
    index=json.loads((PACKET/'index.json').read_bytes());c.need(index['result']['terminal_phase']=='rest194' and not index['result']['full_library_gate_pass'],'R5 remains terminal negative')
    prefix='capture/'+PARENT.name+'/'
    for name,row in index['members'].items():
        body=(PACKET/'objects'/row['sha256']).read_bytes();c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'whole parent evidence bytes')
    BASE.mkdir(exist_ok=False);(BASE/'input').mkdir();(BASE/'artifact').mkdir();(BASE/'state').mkdir()
    source_files={p.relative_to(PARENT/'source').as_posix():p.read_bytes() for p in (PARENT/'source').rglob('*') if p.is_file()}
    source_files['Cargo.lock']=(PARENT/'state/resolved.lock').read_bytes();_,u,_=m.parent();m.verify_source(source_files,u)
    for n,b in source_files.items():
        path=BASE/'source'/n;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b)
    for name in ('U-fixture.lock','consumer.rs'):(BASE/'input'/name).write_bytes((PARENT/'input'/name).read_bytes())
    for name in (m.PREFIX+'.crate','receipt.json'):(BASE/'artifact'/name).write_bytes((PARENT/'artifact'/name).read_bytes())
    frame={}
    for name in ('STOP.json','analysis/result.json','state/extracted.json','state/image-194.json','state/image-188.json','state/catalog-194.json','state/catalog-188.json',
                 *[f'native/{phase}/{receipt}' for phase in ('stage','resolve','package','build194','build188','pure194','rest194') for receipt in ('result.json','execution.json','session-result.json')]):
        path=PARENT/name;row=index['members'][prefix+name];c.need(c.sha(path.read_bytes())==row['sha256'],'exact held parent receipt')
        frame[path.relative_to(ROOT).as_posix()]=row['sha256']
    c.write(BASE/'input','parent-frame.json',frame)
    c.write(BASE/'input','scope.json',dict(parent_sha256=SHA,operator_agreed_full_lane=True,library_bound=780,short_case_bound=5,local_bound=1200,native_bound=1100,
        floor=4*1024**3,poll=5,retirement=5,retry=False,root_adopted=False,public_registry_qualified=False,release_admitted=False))
    print(json.dumps(dict(complete=True,full_lane_prepared=True,parent_first_refusal_preserved=True,root_adopted=False)))

def registry_functions(out,invoke):
    # Reuse reviewed CURRENT source helpers, not archived Python. A private
    # namespace redirects only output/native paths; prior module globals stay.
    module=c.load(ROOT/'tools/chitchat-package-r5-074/run.py','hc074_approved_package_calls')
    module.m=types.SimpleNamespace(**dict(vars(m),BASE=BASE));module.N=N;module.OUT=out;module.invoke=invoke
    c.need(m.BASE==PARENT and prior.NATIVE!=N,'no mutation of the terminal parent context')
    return module
