"""Separate whole-library unit build; no test or live host entrypoint execution."""
from pathlib import Path
import hashlib
ROOT=Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
path=ROOT/'tools/dbus-original-owner-074/native.py';raw=path.read_bytes()
if hashlib.sha256(raw).hexdigest()!='30aa12ddc33da8b94a0c8fd07d4161b2d4d42e27baa08c1c2d39070d5b045b9a':raise ValueError('same bounded original CI coordinator')
text=raw.decode().replace("BASE=ROOT/'target/dbus-original-owner-20261028'","BASE=ROOT/'target/dbus-original-delivery-20261028/unit-build'",1).replace('/home/artur/.cache/hydracache-074-dbus-original-owner/20261028-a1','/home/artur/.cache/hydracache-074-dbus-original-delivery/20261028-a2',1)
text=text.replace("('check','clippy')","('build',)")
text=text.replace("argv[argv.index('--all-targets')]='--lib'","\n  if '--all-targets' in argv:argv[argv.index('--all-targets')]='--lib'")
old="owner.write(out,'result.json',dict(exit=code,source_unchanged=True,library_only=True,tests_executed=False,release_admitted=False))"
new='''artifact=None
    if code==0:
     messages=[json.loads(line) for line in (out/'stdout.log').read_bytes().splitlines()]
     found=[m for m in messages if m.get('reason')=='compiler-artifact' and m['target']['name']=='hydracache_long_run_supervisor_074' and m['target']['kind']==['lib'] and m['profile']['test'] and m.get('executable')]
     owner.need(len(found)==1 and [m['success'] for m in messages if m.get('reason')=='build-finished']==[True],'actual full original library test artifact')
     image=Path(found[0]['executable']);target=NATIVE/'builds'/variant/compiler
     owner.need(image.resolve().is_relative_to(target) and Path(found[0]['target']['src_path'])==folder/'tools/long-run-supervisor-074/src/lib.rs','owned library test image/source')
     artifact=dict(path=str(image),sha256=owner.sha(image.read_bytes()),bytes=image.stat().st_size)
    owner.write(out,'result.json',dict(exit=code,source_unchanged=True,library_only=True,tests_executed=False,artifact=artifact,release_admitted=False))'''
if text.count(old)!=1:raise ValueError('exact original phase receipt site')
text=text.replace(old,new,1)
exec(compile(text,str(path),'exec'),globals())
