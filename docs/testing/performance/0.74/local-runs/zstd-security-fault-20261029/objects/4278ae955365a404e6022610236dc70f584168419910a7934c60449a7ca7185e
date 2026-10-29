"""Nonroot local GNU/MSRV peer; original LinuxGroup inside the original Job."""
from pathlib import Path
import importlib.util
import json
import os
import subprocess
import sys
import time

import run as w

ROOT, BASE = w.ROOT, w.BASE
NATIVE = Path('/home/artur/.cache/hydracache-074-markdown-review/20261029')
GROUP_SHA = 'e5ff52835fc2486171dbdbbb1b6df47177f583b4539f561779992c2c5065a691'


def require(value, reason):
    if not value:
        raise ValueError(reason)


def write(folder, name, value):
    with (folder/name).open('x',encoding='utf8',newline='\n') as out:
        json.dump(value,out,indent=2,sort_keys=True);out.write('\n')


def source():
    value=w.source()
    for path in (BASE/'native-inputs').rglob('*'):
        if path.is_file():value[path.relative_to(ROOT).as_posix()]=w.sha(path.read_bytes())
    return value


def capacity():
    import shutil
    if shutil.disk_usage(ROOT).free<4*1024**3 or NATIVE.exists() and shutil.disk_usage(NATIVE).free<4*1024**3:
        raise w.owned.CapacityError('original physical/native four-GiB floors')


def codec():
    path=ROOT/'tools/dbus-source-package-074/package.py'
    require(w.sha(path.read_bytes())=='1b13c005abfbb96b1374e27fcc871ce0a14d87fe955e2097712a9cfb029fadf3','same safe archive codec')
    spec=importlib.util.spec_from_file_location('markdown_native_registry_codec',path)
    value=importlib.util.module_from_spec(spec);spec.loader.exec_module(value);return value


def seed():
    NATIVE.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
    require(not NATIVE.exists() and NATIVE.parent.resolve()==NATIVE.parent
            and NATIVE.parent.stat().st_uid==1000,'exclusive nonroot private source root')
    NATIVE.mkdir(mode=0o700)
    inputs=json.loads((BASE/'native-inputs/registry.json').read_bytes())
    unpack=codec().unpack
    inventory={}
    for name,digest in inputs.items():
        capacity();raw=(BASE/'native-inputs/archives'/(name+'.crate')).read_bytes()
        require(w.sha(raw)==digest,'official archive checksum')
        members=unpack(raw,name);folder=NATIVE/'vendor'/name;folder.mkdir(parents=True)
        for relative,body in members.items():
            path=folder/relative;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(body)
        hashes={n:w.sha(body) for n,body in members.items()};inventory[name]=hashes
        write(folder,'.cargo-checksum.json',dict(package=digest,files=hashes))
    (NATIVE/'cargo-home').mkdir(mode=0o700)
    configuration='[source.crates-io]\nreplace-with="reviewed-md"\n[source.reviewed-md]\ndirectory="'+str(NATIVE/'vendor')+'"\n'
    (NATIVE/'cargo-home/config.toml').write_text(configuration,encoding='utf8')
    write(NATIVE,'registry.json',inventory)


def registry():
    values=json.loads((NATIVE/'registry.json').read_bytes())
    for name,members in values.items():
        for relative,expected in members.items():
            require(w.sha((NATIVE/'vendor'/name/relative).read_bytes())==expected,'fresh native registry source')


def invoke(out,label,argv,env,deadline,expected=0):
    require(time.monotonic()<deadline,'original GNU session deadline')
    before=source();registry();folder=out/label;folder.mkdir(exist_ok=False)
    write(folder,'intent.json',dict(argv=argv,deadline=deadline,expected_exit=expected,retry=False))
    with (folder/'stdout.log').open('xb') as stdout,(folder/'stderr.log').open('xb') as stderr:
        process=subprocess.run([sys.executable,'-B',str(Path(__file__)),'command-worker',str(folder)],
            stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,
            env=dict(env,HC074_MD_COMMAND=json.dumps(argv),HC074_MD_DEADLINE=str(deadline)),
            timeout=max(0,deadline-time.monotonic())+10)
    value=json.loads((folder/'execution.json').read_bytes())
    require(process.returncode==value['exit']==expected and value['retired'] and not value['errors'],'original Linux command/retirement')
    registry();require(before==source(),'GNU probe/source drift')
    print(json.dumps(dict(phase=label,actual_exit=process.returncode)),flush=True)
    return folder


def session(compiler):
    require(os.getuid()==os.getgid()==1000,'fixed nonroot GNU operator')
    out=BASE/('gnu-'+compiler);deadline=time.monotonic()+1100;before=source()
    boot=Path('/proc/sys/kernel/random/boot_id').read_text()
    result=dict(review_complete=False,release_admitted=False,server=False,clocks=False)
    try:
        if compiler=='seed':seed()
        else:
            version={'194':'1.94.0','188':'1.88.0'}[compiler]
            tool=Path('/home/artur/.rustup/toolchains')/(version+'-x86_64-unknown-linux-gnu')/'bin'
            env=dict(HOME='/home/artur',USER='artur',LOGNAME='artur',LANG='C.UTF-8',
                PATH=str(tool)+':/usr/bin:/bin',CARGO_HOME=str(NATIVE/'cargo-home'),CARGO_NET_OFFLINE='true',
                RUSTC=str(tool/'rustc'),RUSTUP_TOOLCHAIN=version+'-x86_64-unknown-linux-gnu',
                RUSTC_WRAPPER='',RUSTC_WORKSPACE_WRAPPER='',CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='2')
            f=invoke(out,'version',[str(tool/'rustc'),'-vV'],env,deadline)
            actual=(f/'stdout.log').read_text()
            require('release: '+version+'\n' in actual and 'host: x86_64-unknown-linux-gnu\n' in actual,'actual GNU compiler')
            for mode in ('baseline','candidate'):
                target=NATIVE/'targets'/(mode+'-'+compiler)
                args=['--manifest-path',str(BASE/'fixtures'/mode/'Cargo.toml'),'--target-dir',str(target),'--offline','--locked']
                built=invoke(out,mode+'-build',[str(tool/'cargo'),'test',*args,'--no-run','--message-format=json'],env,deadline)
                images=[]
                for line in (built/'stdout.log').read_text().splitlines():
                    row=json.loads(line)
                    if row.get('reason')=='compiler-artifact' and row.get('profile',{}).get('test') and row.get('executable'):
                        path=Path(row['executable'])
                        require(Path(row['target']['src_path']).resolve()==(w.TOOLS/'probe.rs').resolve()
                                and path.resolve().is_relative_to(target),'held actual GNU artifact')
                        images.append(dict(path=str(path),sha256=w.sha(path.read_bytes())))
                require(len(images)==1,'one actual GNU test image');image=images[0];write(out,mode+'-artifact.json',image)
                listed=invoke(out,mode+'-list',[image['path'],'--list'],env,deadline)
                require(sorted(s[:-6] for s in (listed/'stdout.log').read_text().splitlines() if s.endswith(': test'))==w.TESTS,'full actual GNU inventory')
                require(w.sha(Path(image['path']).read_bytes())==image['sha256'],'fresh GNU image')
                tested=invoke(out,mode+'-test',[image['path'],'--test-threads=1','--nocapture','--color=never'],env,deadline,101 if mode=='baseline' else 0)
                raw=(tested/'stdout.log').read_text();expected='2 passed; 1 failed;' if mode=='baseline' else '3 passed; 0 failed;'
                require(raw.count('test result:')==1 and expected in raw and '0 ignored; 0 measured; 0 filtered out;' in raw,'complete GNU test result')
                require(w.sha(Path(image['path']).read_bytes())==image['sha256'],'GNU image postflight')
                invoke(out,mode+'-check',[str(tool/'cargo'),'check',*args,'--all-targets','--all-features'],env,deadline)
                invoke(out,mode+'-clippy',[str(tool/'cargo'),'clippy',*args,'--all-targets','--all-features','--','-D','warnings'],env,deadline)
        require(before==source() and boot==Path('/proc/sys/kernel/random/boot_id').read_text(),'complete GNU source/boot postflight')
        require(time.monotonic()<deadline,'original GNU session deadline')
        result.update(review_complete=True,source_unchanged=True,boot_unchanged=True)
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error))
    write(out,'native-result.json',result)
    raise SystemExit(0 if result['review_complete'] else 1)


def windows(phase):
    out=BASE/('gnu-'+phase);out.mkdir(exist_ok=False);before=source()
    head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT)
    argv=['wsl.exe','-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B',
          '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/markdown-security-review-074/native.py','session',phase]
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        write(out,'intent.json',dict(argv=argv,source=before,head=head.decode(),bound=1200,session=1100,retry=False))
        code=subprocess.call([sys.executable,'-B',str(Path(__file__)),phase,'windows-worker'],stdout=stdout,stderr=stderr,cwd=ROOT)
    execution=json.loads((out/'execution.json').read_bytes());native=json.loads((out/'native-result.json').read_bytes())
    require(code==0 and execution['retired'] and not execution['errors'] and native['review_complete'],'complete GNU-owned peer')
    require(before==source() and head==subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT),'complete host postflight')
    write(out,'result.json',dict(review_complete=True,source_unchanged=True,head_unchanged=True,release_admitted=False))
    print(json.dumps(dict(gnu_phase=phase,review_complete=True)))


if __name__=='__main__':
    if len(sys.argv)==3 and sys.argv[1]=='command-worker':
        folder=Path(sys.argv[2]);path=ROOT/'scripts/ci/watchdog_linux_group.py'
        require(w.sha(path.read_bytes())==GROUP_SHA,'original Linux CI owner')
        spec=importlib.util.spec_from_file_location('md_original_linux_group',path)
        group=importlib.util.module_from_spec(spec);spec.loader.exec_module(group)
        result=w.owned._owned(group.LinuxGroup(),json.loads(os.environ.pop('HC074_MD_COMMAND')),str(ROOT),capacity,
            bound=1200,deadline=float(os.environ.pop('HC074_MD_DEADLINE')))
        write(folder,'execution.json',result);raise SystemExit(result['exit'])
    if len(sys.argv)==3 and sys.argv[1]=='session':session(sys.argv[2])
    if len(sys.argv)==3 and sys.argv[2]=='windows-worker':
        phase=sys.argv[1]
        command=['wsl.exe','-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B',
            '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/markdown-security-review-074/native.py','session',phase]
        result=w.owned._owned(w.owned.load_job().WindowsJob(),command,str(ROOT),capacity,bound=1200)
        write(BASE/('gnu-'+phase),'execution.json',result);raise SystemExit(result['exit'])
    require(len(sys.argv)==2 and sys.argv[1] in ('seed','194','188'),'one fixed GNU phase')
    windows(sys.argv[1])
