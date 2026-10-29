"""Fresh source/ref/config/environment observations, never cached admission."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

import prepare as p


def git(root,*args):
    return subprocess.check_output(['git','--no-replace-objects',*args],cwd=root,timeout=30)


def helpers(root):
    names=[r.relative_to(root).as_posix() for r in (root/'tools/windows-workspace-preflight-074').iterdir() if r.is_file()]
    names += ['scripts/ci/watchdog_windows_job.py','tools/xtask-package-attribution-074/review.py']
    return {name:p.sha((root/name).read_bytes()) for name in sorted(names)}


def frame(root=p.ROOT,*,clean=True):
    p.require(os.name=='nt' and root.resolve()==root, 'original Windows workspace only')
    p.require(not any(k in os.environ for k in ('PYTHONPATH','PYTHONHOME')),'inherited Python overrides')
    review=p.source_review(root)
    environment=p.environment_review(root)  # Original executable/lazy Git controls before status.
    status=git(root,'status','--porcelain','--untracked-files=all','--','Cargo.toml','Cargo.lock',
        'deny.toml','.cargo','crates','tools','scripts','console','.github','.gitattributes')
    p.require(not clean or not status,'clean source/configuration required before Cargo')
    names=[n.decode() for n in git(root,'ls-files','-z').split(b'\0') if n]
    selected=[n for n in names if (n.startswith(('crates/','tools/','scripts/','console/','.cargo/','.github/','docs/plans/'))
        or n in ('Cargo.toml','Cargo.lock','deny.toml','.gitattributes','docs/COMPAT.md','docs/RULES.md','docs/GATES.md',
                  'docs/testing/performance/0.74/qualification-manifest.toml')) and '/node_modules/' not in n]
    sources={}
    for name in selected:
        path=root/name
        p.require(path.is_file() and not path.is_symlink() and not path.is_junction()
            and path.resolve().is_relative_to(root),'source custody: '+name)
        sources[name]=p.sha(path.read_bytes())
    config=git(root,'config','--includes','--show-origin','--list')
    branch=git(root,'symbolic-ref','HEAD').decode().strip()
    p.require(branch=='refs/heads/feat/0.74-resp-native-throughput','original worktree branch required')
    versions={}
    for command in ('cargo','rustc'):
        argv=[command,'--version'] if command=='cargo' else [command,'-vV']
        raw=subprocess.check_output(argv,cwd=root,timeout=15,env=dict(os.environ,RUSTUP_AUTO_INSTALL='0'))
        p.require((raw.startswith(b'cargo 1.94.0 ') if command=='cargo' else
            b'release: 1.94.0\n' in raw and b'host: x86_64-pc-windows-msvc\n' in raw),'actual default toolchain drift')
        versions[command]=dict(path=shutil.which(command),sha256=p.sha(Path(shutil.which(command)).read_bytes()),version=raw.decode())
    versions['python']=dict(path=sys.executable,sha256=p.sha(Path(sys.executable).read_bytes()),version=sys.version)
    versions['git']=dict(path=shutil.which('git'),sha256=p.sha(Path(shutil.which('git')).read_bytes()),version=git(root,'--version').decode())
    fingerprint={k:v for k,v in os.environ.items() if k not in ('CARGO_INCREMENTAL','CARGO_BUILD_JOBS')}
    return dict(head=git(root,'rev-parse','HEAD').decode().strip(),branch=branch,refs_sha256=p.sha(git(root,'show-ref','--head')),
        config_sha256=p.sha(config),status_sha256=p.sha(status),sources=sources,helpers=helpers(root),versions=versions,
        inherited_environment_sha256=p.sha(json.dumps(fingerprint,sort_keys=True).encode()),
        environment=environment,targets=review['targets'],workspace_members=p.workspace_members(root))


def equivalent(before,after):
    p.require(before==after,'source/ref/configuration/toolchain/environment frame drift')
