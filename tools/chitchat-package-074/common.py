"""Unchanged Root pins and original owned local/native envelopes."""
import json,os,shutil,sys,time
from pathlib import Path
import model as m
sys.path.insert(0,str(m.ROOT/'tools/chitchat-security-only-074'));import scope as c
NATIVE=Path('/home/artur/.cache/hydracache-074-package/20261030-a1')
VENDOR=Path('/home/artur/.cache/hydracache-074-security-only/20261030-v3/vendor')

def source():
    value=c.source()
    for root in (m.HERE,m.BASE/'source',m.BASE/'canonical-U',m.BASE/'input'):
        for path in root.rglob('*'):
            if '__pycache__' in path.parts:continue
            c.need(not path.is_symlink() and not getattr(path,'is_junction',lambda:False)(),'regular actual source/input')
            if path.is_file():value[path.relative_to(m.ROOT).as_posix()]=c.sha(path.read_bytes())
    return value

def capacity():
    c.capacity()
    if os.name=='posix' and NATIVE.exists() and shutil.disk_usage(NATIVE).free<4*1024**3:raise c.owned.CapacityError('original native four-GiB floor')

def environment(compiler='194'):
    tool,env,suffix=c.environment(compiler)
    if os.name=='posix':env['CARGO_HOME']=str(NATIVE/'cargo-home')
    return tool,env,suffix

def registry(deadline):
    ledger=json.loads((m.BASE/'input/registry.json').read_bytes());total=0
    for name,digest in ledger.items():
        c.need(time.monotonic()<deadline,'original registry/session deadline');capacity()
        folder=VENDOR/name;values=c.tree(folder);check=json.loads((folder/'.cargo-checksum.json').read_bytes());values.pop('.cargo-checksum.json')
        c.need(check['package']==digest and values==check['files'],'fresh whole exact registry source: '+name);total+=len(values)
    return dict(packages=len(ledger),members=total)

def actual():
    for label in ('source','extracted'):
        frame=m.BASE/'state'/(label+'.json')
        if frame.exists():c.need(c.tree(NATIVE/label)==json.loads(frame.read_bytes()),'whole actual immutable '+label)
    for name in ('config.toml',):
        frame=m.BASE/'state'/('cargo-'+name+'.json')
        if frame.exists():c.need(c.sha((NATIVE/'cargo-home'/name).read_bytes())==json.loads(frame.read_bytes())['sha256'],'original isolated Cargo configuration')
    frame=m.BASE/'state/consumer.json'
    if frame.exists():
        row=json.loads(frame.read_bytes());c.need(c.tree(NATIVE/'consumer')==row['source'],'whole original external consumer')
        downloaded=c.tree(Path(row['downloaded_path']));downloaded.pop('.cargo-ok',None);downloaded.pop('.cargo-checksum.json',None)
        c.need(downloaded==row['downloaded'],'whole actual independent registry package')
        for name,digest in row['registry_files'].items():c.need(c.sha((NATIVE/name).read_bytes())==digest,'held local registry source/endpoint')

def guard():source();actual();capacity()

def write(folder,name,row):folder.mkdir(parents=True,exist_ok=True);c.write(folder,name,row)
