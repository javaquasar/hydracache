"""GNU functional gate only, under the original Job and Linux subreaper limits."""
from pathlib import Path
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import time
import traceback

import common as c
import run

NATIVE=Path('/home/artur/.cache/hydracache-074-chitchat-inplace/20261029')
VARIANT='L3'


def load(path, name):
    spec=importlib.util.spec_from_file_location(name,path)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return module


def capacity():
    c.capacity()
    if NATIVE.exists() and shutil.disk_usage(NATIVE).free<4*1024**3:
        raise c.owned.CapacityError('original native four-GiB floor')


def configuration():
    return '[source.crates-io]\nreplace-with="inplace-native"\n[source.inplace-native]\ndirectory="'+(NATIVE/'vendor').as_posix()+'"\n'


def native_guard():
    c.guard(VARIANT);capacity()
    c.need(os.getuid()==os.getgid()==1000 and NATIVE.resolve()==NATIVE and NATIVE.stat().st_uid==1000,'fixed native owner/path')
    c.need((NATIVE/'cargo-home/config.toml').read_text(encoding='utf8')==configuration(),'exact native directory source')
    inventory=json.loads((c.BASE/'vendor-receipt.json').read_bytes())
    for name,members in inventory.items():
        folder=NATIVE/'vendor'/name
        actual=c.files(folder);checksum=actual.pop('.cargo-checksum.json')
        c.need(actual==members,'complete fresh native registry inventory')
        c.need(checksum==c.sha((c.BASE/'vendor'/name/'.cargo-checksum.json').read_bytes()),'exact native checksum metadata')


def seed():
    c.guard(VARIANT);capacity()
    c.need(os.getuid()==os.getgid()==1000,'fixed nonroot native operator')
    NATIVE.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
    c.need(NATIVE.parent.resolve()==NATIVE.parent and NATIVE.parent.stat().st_uid==1000,'private native parent')
    NATIVE.mkdir(mode=0o700,exist_ok=False)
    codec=load(c.ROOT/'tools/dbus-source-package-074/package.py','inplace_native_source_codec')
    lock=json.loads((c.BASE/'vendor-receipt.json').read_bytes())
    import tomllib
    rows=tomllib.loads((c.BASE/'fixtures'/VARIANT/'Cargo.lock').read_text(encoding='utf8'))['package']
    registry={r['name']+'-'+r['version']:r['checksum'] for r in rows if r.get('source','').startswith('registry+')}
    c.need(set(registry)==set(lock),'complete native locked registry set')
    for name,digest in registry.items():
        capacity();raw=(c.BASE/'input/archives'/(name+'.crate')).read_bytes()
        c.need(c.sha(raw)==digest,'native locked archive identity')
        members=codec.unpack(raw,name);folder=NATIVE/'vendor'/name;folder.mkdir(parents=True)
        for relative,body in members.items():
            path=folder/relative;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(body)
        c.write(folder,'.cargo-checksum.json',dict(package=digest,files={n:c.sha(b) for n,b in members.items()}))
    (NATIVE/'cargo-home').mkdir(mode=0o700)
    (NATIVE/'cargo-home/config.toml').write_text(configuration(),encoding='utf8')
    native_guard()


def invoke(out,label,argv,env,deadline):
    native_guard();before=c.source();folder=out/label;folder.mkdir(exist_ok=False)
    c.write(folder,'intent.json',dict(argv=argv,deadline=deadline,retry=False))
    with (folder/'stdout.log').open('xb') as stdout,(folder/'stderr.log').open('xb') as stderr:
        worker=subprocess.run([sys.executable,'-B',str(Path(__file__)),'command',str(folder)],cwd=c.ROOT,
            stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,
            env=dict(env,HC074_GNU_ARGV=json.dumps(argv),HC074_GNU_DEADLINE=str(deadline)),timeout=max(0,deadline-time.monotonic())+10)
    result=json.loads((folder/'execution.json').read_bytes())
    c.need(worker.returncode==result['exit']==0 and result['retired'] and not result['errors'],'first native owned outcome: '+label)
    native_guard();c.need(before==c.source(),'native source drift')
    return folder


def session(label):
    out=c.BASE/label;deadline=time.monotonic()+1100;result=dict(complete=False,release_admitted=False)
    try:
        c.environment_review()
        if label=='seed-gnu':seed()
        else:
            compiler=label.removeprefix('gnu-');c.need(compiler in ('194','188'),'fixed native compiler')
            version={'194':'1.94.0','188':'1.88.0'}[compiler]
            tool=Path.home()/'.rustup/toolchains'/(version+'-x86_64-unknown-linux-gnu')/'bin'
            env=dict(HOME='/home/artur',USER='artur',LOGNAME='artur',LANG='C.UTF-8',
                PATH=str(tool)+':/usr/local/bin:/usr/bin:/bin',RUSTC=str(tool/'rustc'),
                RUSTUP_TOOLCHAIN=version+'-x86_64-unknown-linux-gnu',RUSTC_WRAPPER='',RUSTC_WORKSPACE_WRAPPER='',
                CARGO_HOME=str(NATIVE/'cargo-home'),CARGO_NET_OFFLINE='true',CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='2',PROPTEST_RNG_SEED='7606312')
            folder=invoke(out,'compiler',[str(tool/'rustc'),'-vV'],env,deadline)
            raw=(folder/'stdout.log').read_text(encoding='utf8')
            c.need('release: '+version+'\n' in raw and 'host: x86_64-unknown-linux-gnu\n' in raw,'actual native compiler/host')
            target=NATIVE/'targets'/compiler
            args=['--offline','--locked','--manifest-path',str(c.BASE/'fixtures'/VARIANT/'Cargo.toml'),'--target-dir',str(target)]
            folder=invoke(out,'build',[str(tool/'cargo'),'test',*args,'--lib','--features','testsuite','--no-run','--message-format=json'],env,deadline)
            images=[]
            for line in (folder/'stdout.log').read_text(encoding='utf8').splitlines():
                row=json.loads(line)
                if row.get('reason')=='compiler-artifact' and row.get('profile',{}).get('test') and row.get('executable'):
                    path=Path(row['executable']);c.need(path.resolve().is_relative_to(target) and Path(row['target']['src_path']).resolve()==(c.BASE/'fixtures'/VARIANT/'src/lib.rs').resolve(),'actual native image/source')
                    images.append(dict(path=str(path),sha256=c.sha(path.read_bytes())))
            c.need(len(images)==1,'one native library image');image=images[0];c.write(out,'artifact.json',image)
            folder=invoke(out,'list',[image['path'],'--list'],env,deadline)
            names=[v[:-6] for v in (folder/'stdout.log').read_text(encoding='utf8').splitlines() if v.endswith(': test')]
            expected=[n for n in run.expected_catalog(VARIANT) if n!='delta::cost_review::cpu_observations']
            c.need(sorted(names)==expected and len(set(names))==len(names)==118,'exact complete GNU catalog')
            c.write(out,'catalog.json',names);cases=[]
            for index,prefix in enumerate(run.GROUPS+['serialize::state_review::']):
                chosen=[n for n in names if n.startswith(prefix)];c.need(chosen,'nonempty native partition')
                folder=invoke(out,'test-'+str(index),[image['path'],prefix,'--test-threads=1','--show-output','--color=never'],env,deadline)
                raw=(folder/'stdout.log').read_text(encoding='utf8')
                cases.append(dict(prefix=prefix,names=chosen,counts=run.accounting(raw,len(chosen),len(names))))
                if prefix=='delta::mtu_review::':run.semantics(run.diagnostic(raw,'HC074_SUPPORTED_DELTA:')['cases'])
            c.write(out,'cases.json',cases)
            c.need(c.sha(Path(image['path']).read_bytes())==image['sha256'],'native image postflight')
            for action in ('check','clippy'):
                argv=[str(tool/'cargo'),action,*args,'--all-targets','--all-features']
                if action=='clippy':argv+=['--','-D','warnings']
                invoke(out,action,argv,env,deadline)
        c.need(time.monotonic()<deadline,'original complete GNU session deadline')
        result.update(complete=True,root_adopted=False,performance_claim=False)
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'session-result.json',result);raise SystemExit(0 if result['complete'] else 1)


def main(label):
    c.need(os.name=='nt' and label in ('seed-gnu','gnu-194','gnu-188'),'fixed Windows-to-GNU phase')
    c.environment_review();c.guard(VARIANT);out=c.BASE/label;out.mkdir(parents=True,exist_ok=False)
    before=c.source();head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=c.ROOT)
    c.write(out,'intent.json',dict(source=before,head=head.decode(),bound=1200,session=1100,retry=False,server=False,services=False,clocks=False))
    for path in c.TOOLS.iterdir():
        if path.is_file():
            dest=out/'producer'/path.name;dest.parent.mkdir(exist_ok=True);dest.write_bytes(path.read_bytes())
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'windows-worker',label],cwd=c.ROOT,stdout=stdout,stderr=stderr,stdin=subprocess.DEVNULL,timeout=1210)
    execution=json.loads((out/'execution.json').read_bytes());result=json.loads((out/'session-result.json').read_bytes())
    c.need(child.returncode==execution['exit']==0 and execution['retired'] and not execution['errors'] and result['complete'],'complete first GNU phase')
    c.guard(VARIANT);c.need(before==c.source() and head==subprocess.check_output(['git','rev-parse','HEAD'],cwd=c.ROOT),'root source/HEAD drift')
    c.write(out,'result.json',dict(complete=True,source_after=c.source(),head_unchanged=True,root_adopted=False))
    print(json.dumps(dict(phase=label,complete=True)))


if __name__=='__main__':
    if len(sys.argv)==3 and sys.argv[1]=='session':session(sys.argv[2])
    if len(sys.argv)==3 and sys.argv[1]=='command':
        folder=Path(sys.argv[2]);native_guard()
        argv=json.loads(os.environ.pop('HC074_GNU_ARGV'));deadline=float(os.environ.pop('HC074_GNU_DEADLINE'))
        value=c.owned._owned(load(c.ROOT/'scripts/ci/watchdog_linux_group.py','inplace_original_linux_group').LinuxGroup(),argv,str(c.ROOT),capacity,bound=1200,deadline=deadline)
        c.write(folder,'execution.json',value);raise SystemExit(value['exit'])
    if len(sys.argv)==3 and sys.argv[1]=='windows-worker':
        label=sys.argv[2];c.source()
        argv=['wsl.exe','-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B',
              '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/chitchat-inplace-074/gnu.py','session',label]
        value=c.owned._owned(c.owned.load_job().WindowsJob(),argv,str(c.ROOT),c.capacity,bound=1200)
        c.write(c.BASE/label,'execution.json',value);raise SystemExit(value['exit'])
    c.need(len(sys.argv)==2,'one fixed native phase');main(sys.argv[1])
