"""Freeze actual applied/formatted root and offline registry inputs for package gates."""
from pathlib import Path
import json
import os
import re
import sys
import tomllib

import common as c


def main():
    out=c.BASE/'prepare'
    out.mkdir(exist_ok=False)
    (out/'producer.py').write_bytes(Path(__file__).read_bytes())
    applied=json.loads((c.BASE/'apply/result.json').read_bytes())
    formatted=json.loads((c.BASE/'format/final/result.json').read_bytes())
    native_format=json.loads((c.BASE/'format/final/native-result.json').read_bytes())
    c.need(applied['applied'] and formatted['exit']==0 and native_format['passed'], 'applied root and complete GNU format gate')
    source=c.hashes(c.source_names())
    c.need(source==formatted['after'], 'same final formatted root bytes')
    for name,digest in applied['protected'].items():c.need(source[name]==digest,'original deny/qualification')
    metadata=json.loads(c.app.member('metadata-all-features/cargo.stdout.json'))
    lock=tomllib.loads((c.ROOT/'Cargo.lock').read_text())['package']
    catalog={(p['name'],p['version'],p.get('source')):p for p in lock}
    c.need(len(catalog)==len(lock)==683,'reviewed complete root lock')
    package=next(p for p in metadata['packages'] if p['name']=='hydracache-long-run-supervisor-074')
    workspace=Path(metadata['workspace_root'])
    targets=[]
    for row in package['targets']:
        targets.append(dict(name=row['name'],kind=row['kind'],test=row['test'],doctest=row['doctest'],
            src=Path(row['src_path']).relative_to(workspace).as_posix()))
    c.need(len(targets)==90,'all original package targets, including examples')
    inputs=c.BASE/'input'
    (inputs/'archives').mkdir(parents=True)
    registry=[]
    home=Path(os.environ['USERPROFILE'])/'.cargo'
    for pkg in metadata['packages']:
        if pkg['source'] is None:continue
        c.need(pkg['source']=='registry+https://github.com/rust-lang/crates.io-index','same registry policy')
        row=catalog[(pkg['name'],pkg['version'],pkg['source'])]
        manifest=Path(pkg['manifest_path'])
        slot=manifest.parent.parent.name
        c.need(manifest.parent.parent.parent.resolve()==(home/'registry/src').resolve()
               and re.fullmatch('index.crates.io-[a-f0-9]{16}',slot),'exact metadata-selected registry slot')
        name=pkg['name']+'-'+pkg['version']
        path=home/'registry/cache'/slot/(name+'.crate')
        raw=path.read_bytes()
        c.need(c.sha(raw)==row['checksum'],'exact lock-bound archive: '+name)
        (inputs/'archives'/(name+'.crate')).write_bytes(raw)
        registry.append(dict(name=name,sha256=row['checksum'],bytes=len(raw),source_slot=slot))
    c.need(len(registry)==644,'all reviewed registry identities available offline')
    helper_names=[p.relative_to(c.ROOT).as_posix() for p in Path(__file__).parent.iterdir() if p.is_file()]
    helper_names+=['scripts/ci/watchdog_linux_group.py','tools/windows-workspace-preflight-074/owned.py',
                   'tools/windows-workspace-preflight-074/prepare.py','tools/dbus-source-package-074/package.py']
    receipt=dict(source=source,helpers=c.hashes(helper_names),native_root=c.NATIVE,registry=registry,
        targets=targets,source_snapshot=c.git_snapshot(),compilers=['1.94.0','1.88.0'],
        contract=dict(outer_seconds=1200,session_seconds=1100,floor_bytes=4*1024**3,poll_seconds=5,retirement_seconds=5,
            check='--all-targets --all-features',clippy='--all-targets --all-features -- -D warnings',
            tests='all actual test-profile artifacts of --all-targets; full unfiltered default catalogs',
            ignored='reported, never executed except the exact private-bus matrix',retry=False),
        phases=['seed','checks','tests-194','tests-188'],
        source_worktree_uncommitted=True,server=False,release_admitted=False,original_windows_seconds=[300,60,10])
    write=c.write
    (c.BASE/'freeze').mkdir()
    write(c.BASE/'freeze','result.json',receipt)
    write(out,'result.json',dict(prepared=True,source_files=len(source),registry=len(registry),targets=len(targets),release_admitted=False))
    print(json.dumps(dict(prepared=True,source_files=len(source),registry=len(registry),targets=len(targets))))


if __name__=='__main__':main()
