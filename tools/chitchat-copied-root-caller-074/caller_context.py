"""Actual typed callers of admitted copied Root, no signature substitutes."""
import json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'tools/chitchat-copied-root-074'));import root_context as parent
c=parent.c;m=parent.m
BASE=ROOT/'target/chitchat-copied-root-caller-20261030'
N=Path('/home/artur/.cache/hydracache-074-actual-caller/20261030-a1')
VARIANTS=('B','O','mixed')

def manifest(variant):
    c.need(variant in VARIANTS,'three fixed actual caller roots');v='B' if variant=='B' else 'O'
    root=parent.N/'copies'/v
    alias='"=0.11.1"' if variant in ('B','mixed') else '{ package = "hydra-chitchat", version = "=0.11.1-hydra.1", registry = "hc074-draft" }'
    return ('[package]\nname="hc074-actual-caller-'+variant.lower()+'"\nversion="0.0.0"\nedition="2021"\nrust-version="1.88"\npublish=false\n[workspace]\n[dependencies]\n'
        'hydracache={ path="'+(root/'crates/hydracache').as_posix()+'" }\n'
        'hydracache-cluster-chitchat={ path="'+(root/'crates/hydracache-cluster-chitchat').as_posix()+'" }\n'
        'chitchat='+alias+'\n[patch.crates-io]\n'
        'zbus_macros={ path="'+(root/'third_party/hc074-dbus/zbus_macros').as_posix()+'" }\n'
        'zbus_utils={ path="'+(root/'third_party/hc074-dbus/zbus_utils').as_posix()+'" }\n').encode()

def source():
    values=parent.source()
    for folder in (HERE,BASE/'input'):
        for p in folder.rglob('*'):
            if '__pycache__' in p.parts:continue
            c.need(not p.is_symlink() and not getattr(p,'is_junction',lambda:False)(),'regular current caller source')
            if p.is_file():values[p.relative_to(ROOT).as_posix()]=c.sha(p.read_bytes())
    for name in ('stage','metadata'):
        p=parent.BASE/'native'/name/'result.json';c.need(json.loads(p.read_bytes())['complete'],'admitted actual copied Root parent');values[p.relative_to(ROOT).as_posix()]=c.sha(p.read_bytes())
    c.need(not (parent.BASE/'STOP.json').exists(),'no bypass of a failed parent')
    return values

def admission(compiler):
    for v in ('B','O'):
        for action in ('check','lint'):
            p=parent.BASE/'native'/f'{action}-{v}-{compiler}-adapter'/'result.json'
            c.need(json.loads(p.read_bytes())['complete'],'same compiler actual adapter check/lint prerequisite')

def environment(compiler):
    tool,env,suffix=parent.environment(compiler)
    if sys.platform!='win32':env['CARGO_HOME']=str(N/'cargo-home')
    return tool,env,suffix

def actual():
    parent.actual()
    for v in VARIANTS:
        frame=BASE/'state'/('resolved-'+v+'.json')
        if not frame.exists():frame=BASE/'state'/('staged-'+v+'.json')
        if frame.exists():c.need(c.tree(N/'source'/v)==json.loads(frame.read_bytes()),'whole actual typed caller source/lock')
    frame=BASE/'state/config.json'
    if frame.exists():c.need(c.sha((N/'cargo-home/config.toml').read_bytes())==json.loads(frame.read_bytes())['sha256'],'fixed caller registry configuration')
    frame=BASE/'state/owned-download.json'
    if frame.exists():
        row=json.loads(frame.read_bytes());values=c.tree(Path(row['path']));values.pop('.cargo-ok',None);values.pop('.cargo-checksum.json',None)
        c.need(values==row['files'],'whole freshly read actual caller-owned dependency source')

def download(raw,values):
    c.need(c.sha(raw)==parent.abi.CRATE_SHA,'whole exact qualified owned archive')
    expected={n:c.sha(b) for n,b in m.codec().unpack(raw,'hydra-chitchat-0.11.1-hydra.1').items()}
    c.need(values==expected,'whole actually downloaded caller source equals qualified archive')

def graph(data,variant):
    c.need(data['workspace_root']==str(N/'source'/variant),'real separate caller workspace')
    ledger=json.loads((parent.INPUT/'registry.json').read_bytes());reference=json.loads((parent.INPUT/'registry-reference.json').read_bytes())
    selected='B' if variant=='B' else 'O';root=parent.N/'copies'/selected
    expected=json.loads((parent.BASE/'state'/('staged-B.json' if selected=='B' else 'resolved-O.json')).read_bytes())
    for p in data['packages']:
        if p.get('source'):
            if p['name']=='hydra-chitchat':c.need((p['version'],p['source'])==('0.11.1-hydra.1',reference['registry']),'exact normal owned caller identity')
            else:c.need(p['source']=='registry+https://github.com/rust-lang/crates.io-index' and p['name']+'-'+p['version'] in ledger,'only lock-bound ordinary caller dependencies')
        elif p['manifest_path']!=str(N/'source'/variant/'Cargo.toml'):
            path=Path(p['manifest_path']);c.need(path.is_relative_to(root) and path.relative_to(root).as_posix() in expected,'actual admitted Root path dependency, no stub')
    chosen={p['name']:p for p in data['packages'] if p['name'] in ('chitchat','hydra-chitchat')}
    c.need(set(chosen)==({'chitchat'} if variant=='B' else {'hydra-chitchat'} if variant=='O' else {'chitchat','hydra-chitchat'}),'exact positive or deliberately mixed graph')
    for name in ('hydracache','hydracache-cluster-chitchat'):
        rows=[p for p in data['packages'] if p['name']==name];c.need(len(rows)==1 and rows[0]['manifest_path']==str(root/'crates'/name/'Cargo.toml'),'actual single admitted core/adapter')
    return dict(complete=True,selected_root=selected,chitchat_packages=sorted(chosen),normal_registry_packages=sum(bool(p.get('source')) for p in data['packages']),root_adopted=False)
