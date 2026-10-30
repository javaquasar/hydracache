"""Local preparation of the approved dependency backport, never Root adoption."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import tomllib

ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
BASE=ROOT/'target/chitchat-security-only-20261030'
NATIVE=Path('/home/artur/.cache/hydracache-074-security-only/20261030-v1')
PINS={'Cargo.lock':'8ade63f6a4aae93f27f5161798bc87804542a48093b4203f35e25e29b680a4f5',
 'deny.toml':'16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930',
 'docs/testing/performance/0.74/qualification-manifest.toml':'11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc',
 'tools/windows-workspace-preflight-074/owned.py':'e7c5a3c0c4fea90f39772512c4ad96c60693fc0b482092a608dcf039e151eade',
 'scripts/ci/watchdog_windows_job.py':'8b392811e106eb7e692e21ee5b55cd04ea8ac421cad015edb66ce736b08c2ab1',
 'scripts/ci/watchdog_linux_group.py':'e5ff52835fc2486171dbdbbb1b6df47177f583b4539f561779992c2c5065a691',
 'tools/dbus-source-package-074/package.py':'1b13c005abfbb96b1374e27fcc871ce0a14d87fe955e2097712a9cfb029fadf3'}
BASE_SHA='9e1cfd5d16c6fc43487006ab5b5c910f295cc53cf07d90491fb36c8dc479c754'
sys.path.insert(0,str(ROOT/'tools/windows-workspace-preflight-074'));import owned

def need(ok,why):
    if not ok:raise ValueError(why)

def sha(body):return hashlib.sha256(body).hexdigest()

def write(folder,name,value):
    with (folder/name).open('x',encoding='utf8',newline='\n') as f:json.dump(value,f,indent=2,sort_keys=True);f.write('\n')

def load(path,name):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module

def git(*argv):
    prefix=['git'] if os.name=='nt' else ['/mnt/c/Program Files/Git/cmd/git.exe','--git-dir=C:/Workspace/prj/jq/cashe/hydracache/.git/worktrees/hydracache-074','--work-tree=C:/Workspace/prj/jq/cashe/hydracache-074']
    return subprocess.check_output(prefix+list(argv),cwd=ROOT)

def source():
    members=tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf8'))['workspace']['members']
    names={p.decode() for p in git('ls-files','-z','--',*members,'third_party/hc074-dbus').split(b'\0') if p}
    names|=set(PINS)|{'Cargo.toml','.cargo/config.toml'}
    names|={p.relative_to(ROOT).as_posix() for p in HERE.iterdir() if p.is_file()}
    values={}
    for n in sorted(names):
        p=ROOT/n;need(p.is_file() and p.resolve().is_relative_to(ROOT.resolve()) and not p.is_symlink() and not getattr(p,'is_junction',lambda:False)(),'regular actual Root/source member')
        values[n]=sha(p.read_bytes())
    for n,digest in PINS.items():need(values[n]==digest,'unchanged Root/helper: '+n)
    return values

def snapshot():return dict(head=git('rev-parse','HEAD').decode().strip(),status=git('status','--porcelain','--untracked-files=no').decode())

def capacity():
    if shutil.disk_usage(ROOT).free<4*1024**3 or os.name=='posix' and NATIVE.exists() and shutil.disk_usage(NATIVE).free<4*1024**3:raise owned.CapacityError('original physical/native four-GiB floor')

def tree(folder):
    result={}
    for p in folder.rglob('*'):
        need(not p.is_symlink() and not getattr(p,'is_junction',lambda:False)(),'regular source tree')
        if p.is_file():result[p.relative_to(folder).as_posix()]=sha(p.read_bytes())
    return result

def fixtures():return BASE/'fixtures' if os.name=='nt' else NATIVE/'fixtures'

def environment(compiler):
    helper=load(ROOT/'tools/supervisor-zstd-security-074/common.py','security_only_qualified_environment')
    tool,env,suffix=helper.environment(compiler)
    if os.name=='posix':env['CARGO_HOME']=str(NATIVE/'cargo-home')
    env['PROPTEST_RNG_SEED']='7606312'
    return tool,env,suffix
