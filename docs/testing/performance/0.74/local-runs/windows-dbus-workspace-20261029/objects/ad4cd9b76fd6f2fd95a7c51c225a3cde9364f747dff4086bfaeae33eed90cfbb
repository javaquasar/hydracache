"""Offline checks and a fresh GNU build for the separately authorized one-shot lane."""
import check as old
from pathlib import Path

OUTPUT=old.ROOT/'target/bootstrap-original-getunit-20261023'
FILES=list(dict.fromkeys(old.FILES+[
    'tools/bootstrap-original-service-074/getunit_once.py',
    'tools/bootstrap-original-service-074/getunit_checks.py',
    'tools/bootstrap-original-service-074/test_getunit_once.py',
    'docs/testing/performance/0.74/bootstrap-original-getunit-plan-20261023.md',
]))

def run():
    gnu=[step for step in old.commands('194') if step[0] in ('original-units','check','clippy','selected-build','image-sha','image-size','native-libtest')]
    commands=[('controls',['python','-B','-m','unittest','discover','-s','tools/bootstrap-original-service-074','-v']),
        ('canaries',['cargo','test','--offline','--locked','-p','xtask','--test','bootstrap_installed_origin_074']),
        ('shell',['wsl.exe','-d','HydraCache074Native','-u','root','--cd','/','--exec','/bin/sh','-n','/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/bootstrap-original-service-074/native_fixture.sh'])]
    commands+=gnu+[
        ('format',['cargo','fmt','-p','hydracache-long-run-supervisor-074','-p','xtask','--','--check']),
        ('diff',['git','diff','--check'])]
    old.snapshot.OUTPUT,old.snapshot.FILES,old.snapshot.commands=OUTPUT,FILES,lambda _:commands
    old.snapshot.run('checks')

if __name__=='__main__': run()
