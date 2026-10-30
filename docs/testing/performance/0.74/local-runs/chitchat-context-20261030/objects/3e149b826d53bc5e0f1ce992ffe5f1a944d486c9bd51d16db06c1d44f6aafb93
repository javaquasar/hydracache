"""Remove only two terminal, owned native caches; keep all source/evidence."""
import json
import os
from pathlib import Path
import shutil

ROOT=Path('/home/artur/.cache/hydracache-074-borrowed')
BASE=Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/chitchat-borrowed-20261030')


def main():
    if os.getuid()!=0:raise ValueError('readable complete proc census required')
    targets=[ROOT/n/'builds' for n in ('20261030-c1-r2','20261030-c1-r3')]
    if ROOT.resolve()!=ROOT or any(p.resolve()!=p or not p.is_relative_to(ROOT) or p.stat().st_uid!=1000 for p in targets):
        raise ValueError('exact contained user-owned build paths')
    for name in ('native-r2','native-r3'):
        if not json.loads((BASE/name/'execution.json').read_bytes())['retired']:raise ValueError('terminal original owners')
    if json.loads((BASE/'analysis-r3/result.json').read_bytes())['complete']:raise ValueError('preserved artifact refusal')
    for process in Path('/proc').iterdir():
        if not process.name.isdigit():continue
        try:image=os.readlink(process/'exe')
        except FileNotFoundError:continue
        if any(image.startswith(str(t)+'/') for t in targets):raise ValueError('active cache image')
    rows=[]
    for target in targets:
        entries=list(target.rglob('*'))
        if any(p.is_symlink() for p in entries):raise ValueError('no cache symlinks')
        rows.append(dict(path=str(target),bytes=sum(p.stat().st_size for p in entries if p.is_file())))
    before=shutil.disk_usage(ROOT).free
    for target in targets:shutil.rmtree(target)
    value=dict(targets=rows,native_free_before=before,native_free_after=shutil.disk_usage(ROOT).free,
        source_or_evidence_removed=False,active_users=0,first_nonroot_census='refused PermissionError before deletion')
    with (BASE/'native-build-cleanup.json').open('x') as out:json.dump(value,out,indent=2)
    print(json.dumps(value))


if __name__=='__main__':main()
