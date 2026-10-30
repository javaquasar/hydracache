"""Unchanged CI ownership and fresh source custody for the direct tool edge."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib
import time

ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
BASE=ROOT/'target/supervisor-zstd-security-20261030'
NATIVE=Path('/home/artur/.cache/hydracache-074-root-package/20261028-a1')
PKG='tools/long-run-supervisor-074'
PINS={'deny.toml':'16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930',
 'docs/testing/performance/0.74/qualification-manifest.toml':'11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc',
 'tools/windows-workspace-preflight-074/owned.py':'e7c5a3c0c4fea90f39772512c4ad96c60693fc0b482092a608dcf039e151eade',
 'scripts/ci/watchdog_linux_group.py':'e5ff52835fc2486171dbdbbb1b6df47177f583b4539f561779992c2c5065a691',
 'scripts/ci/watchdog_windows_job.py':'8b392811e106eb7e692e21ee5b55cd04ea8ac421cad015edb66ce736b08c2ab1',
 'tools/dbus-source-package-074/package.py':'1b13c005abfbb96b1374e27fcc871ce0a14d87fe955e2097712a9cfb029fadf3'}
OLD_LOCK='34d7a21d9833a180460e5a094f9ee7f1effe07dfb43a65d90865798963e0070c'
ARCHIVES={'zstd-0.14.3':'db3ae9bf90af6e40197bef767eeefc46ad6397420fe873830bd85e5fbbfb3c70',
 'zstd-safe-8.2.0':'ded520f16b29e1a9b771c722221b2b0a0398c2316d9bb3c42b458a5414d7434b'}
sys.path.insert(0,str(ROOT/'tools/windows-workspace-preflight-074'));import owned
TESTS=sorted(('archive_bytes_and_headers_are_deterministic',
 'archive_replay_verification_rejects_digest_and_extra_file_drift',
 'create_new_limits_and_output_placement_fail_closed',
 'hardlinked_input_is_rejected_without_creating_an_archive'))

def need(ok,why):
    if not ok:raise ValueError(why)
def sha(raw):return hashlib.sha256(raw).hexdigest()
def write(folder,name,value):
    with (folder/name).open('x',encoding='utf8',newline='\n') as f:json.dump(value,f,indent=2,sort_keys=True);f.write('\n')
def load(path,name):
    s=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m
def capacity():
    if shutil.disk_usage(ROOT).free<4*1024**3 or os.name=='posix' and shutil.disk_usage(NATIVE).free<4*1024**3:raise owned.CapacityError('original physical/native four-GiB floor')

def git(*args):
    prefix=['git'] if os.name=='nt' else ['/mnt/c/Program Files/Git/cmd/git.exe','--git-dir=C:/Workspace/prj/jq/cashe/hydracache/.git/worktrees/hydracache-074','--work-tree=C:/Workspace/prj/jq/cashe/hydracache-074']
    return subprocess.check_output(prefix+list(args),cwd=ROOT)
def source():
    members=tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf8'))['workspace']['members']
    names=git('ls-files','-z','--',*members,'third_party/hc074-dbus').split(b'\0')
    names={x.decode() for x in names if x}|set(PINS)|{'Cargo.toml','Cargo.lock','.cargo/config.toml'}
    names|={p.relative_to(ROOT).as_posix() for p in HERE.iterdir() if p.is_file()}
    names|={p.relative_to(ROOT).as_posix() for p in (BASE/'probe').rglob('*') if p.is_file() and 'target' not in p.relative_to(BASE/'probe').parts}
    names|={p.relative_to(ROOT).as_posix() for p in (BASE/'input').rglob('*') if p.is_file()}
    if os.name=='posix':
        helper=load(ROOT/'tools/dbus-root-package-074/source_guard.py','direct_zstd_source_guard');result=helper.hashes(ROOT,sorted(names))
    else:
        result={}
        for n in sorted(names):
            p=ROOT/n;need(p.resolve().is_relative_to(ROOT.resolve()) and not p.is_symlink() and not p.is_junction(),'original regular source');result[n]=sha(p.read_bytes())
    for n,h in PINS.items():need(result[n]==h,'original unchanged pin: '+n)
    return result
def snapshot():
    return dict(head=git('rev-parse','HEAD').decode().strip(),tracked=git('status','--porcelain','--untracked-files=no').decode())

def phase(label):return BASE/('gnu' if os.name=='posix' else '.')/label

def registry(deadline):
    need(os.name=='posix','fresh native vendor custody')
    receipt=json.loads((NATIVE/'vendor-receipt.json').read_bytes())
    extra=json.loads((BASE/'input/added-vendor.json').read_bytes())
    need(not set(receipt)&set(extra),'additive vendor source only')
    for name,members in (receipt|extra).items():
        need(time.monotonic()<deadline,'original native session deadline during registry custody');capacity()
        directory=NATIVE/'vendor'/name
        need(directory.resolve()==directory and not directory.is_symlink(),'original vendor directory')
        files={}
        for p in directory.rglob('*'):
            need(not p.is_symlink(),'regular vendor members')
            if p.is_file():files[p.relative_to(directory).as_posix()]=sha(p.read_bytes())
        files.pop('.cargo-checksum.json')
        checks=json.loads((directory/'.cargo-checksum.json').read_bytes())
        need(files==members and checks['files']==members,'whole freshly read vendor inventory: '+name)
    return dict(registry_packages=len(receipt)+len(extra),original_receipt_sha256=sha((NATIVE/'vendor-receipt.json').read_bytes()),config_sha256=sha((NATIVE/'cargo-home/config.toml').read_bytes()))
def environment(compiler):
    exact={'RUSTFLAGS','RUSTDOCFLAGS','CARGO_ENCODED_RUSTFLAGS','CARGO_BUILD_RUSTFLAGS',
      'CC','CXX','AR','CFLAGS','CPPFLAGS','CMAKE_TOOLCHAIN_FILE','BINDGEN_EXTRA_CLANG_ARGS',
      'RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER'}
    prefixes=('CARGO_PROFILE_','CARGO_TARGET_','CC_','CFLAGS_','CXXFLAGS','PKG_CONFIG_','ZSTD_')
    need(not any(v and (k in exact or k.startswith(prefixes)) for k,v in os.environ.items()),'no ambient compiler/source overrides')
    version={'194':'1.94.0','188':'1.88.0'}[compiler]
    if os.name=='nt':
        tool=Path(os.environ['USERPROFILE'])/'.rustup/toolchains'/(version+'-x86_64-pc-windows-msvc')/'bin'
        env=dict(os.environ);env.update(PATH=str(tool)+os.pathsep+env['PATH'],RUSTUP_TOOLCHAIN=version+'-x86_64-pc-windows-msvc')
        suffix='.exe'
    else:
        tool=Path('/home/artur/.rustup/toolchains')/(version+'-x86_64-unknown-linux-gnu')/'bin';suffix=''
        env=dict(HOME='/home/artur',USER='artur',LOGNAME='artur',LANG='C.UTF-8',PATH=str(tool)+':/usr/local/bin:/usr/bin:/bin',CARGO_HOME=str(NATIVE/'cargo-home'),RUSTUP_TOOLCHAIN=version+'-x86_64-unknown-linux-gnu')
    env.update(RUSTC=str(tool/('rustc'+suffix)),RUSTC_WRAPPER='',RUSTC_WORKSPACE_WRAPPER='',CARGO_NET_OFFLINE='true',CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='2')
    return tool,env,suffix
