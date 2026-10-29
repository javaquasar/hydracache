"""Actual affected Windows packages; fixed argv, complete cases, original bounds."""
from pathlib import Path
import json
import os
import subprocess
import sys
import time

import run as w

ROOT, BASE = w.ROOT, w.BASE
PACKAGES = ('hydracache-client-hc2','hydracache-client-plane-spike')
LOCKS = {
    'baseline':'76d4ab3b38c1b259c0784e66dd620a5f61d45cde1760a9533c4d6c7db061b207',
    'candidate':None,
}


def source():
    values=w.source()
    names=subprocess.check_output(['git','ls-files','-z','--','crates','third_party'],cwd=ROOT)
    for name in names.split(b'\0'):
        if name:
            name=name.decode();values[name]=w.sha((ROOT/name).read_bytes())
    return values


def command(action,compiler):
    version={'194':'1.94.0','188':'1.88.0'}[compiler]
    tool=Path(os.environ['USERPROFILE'])/'.rustup/toolchains'/(version+'-x86_64-pc-windows-msvc')/'bin'
    env=dict(os.environ,RUSTC=str(tool/'rustc.exe'),RUSTUP_TOOLCHAIN=version+'-x86_64-pc-windows-msvc',
        RUSTC_WRAPPER='',RUSTC_WORKSPACE_WRAPPER='',CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='2')
    env['PATH']=str(tool)+os.pathsep+env['PATH']
    if action=='version':return [str(tool/'rustc.exe'),'-vV'],env
    args=['--manifest-path',str(ROOT/'Cargo.toml'),'--target-dir',str(BASE/'consumer-targets'/compiler),'--offline','--locked']
    for package in PACKAGES:args+=['-p',package]
    if action=='build':argv=[str(tool/'cargo.exe'),'test',*args,'--no-run','--message-format=json']
    elif action=='metadata':argv=[str(tool/'cargo.exe'),'metadata','--offline','--locked','--format-version','1']
    else:
        argv=[str(tool/'cargo.exe'),action,*args,'--all-targets','--all-features']
        if action=='clippy':argv+=['--','-D','warnings']
    return argv,env


def generated(compiler):
    result={}
    build=BASE/'consumer-targets'/compiler/'debug/build'
    for package in PACKAGES:
        for folder in build.glob(package+'-*/out'):
            for path in sorted(folder.iterdir()):
                if path.is_file() and path.suffix in ('.rs','.bin'):
                    key=package+'/'+path.name
                    body=path.read_bytes()
                    w.p.require(key not in result or result[key]==body,'unambiguous generated consumer source')
                    result[key]=body
    w.p.require(any(n.endswith('.rs') for n in result) and
        any(n.endswith('hc2_contract_descriptor.bin') for n in result),'actual generated source/descriptor')
    return result


def invoke(out,label,argv,env,deadline):
    folder=out/label;folder.mkdir(exist_ok=False)
    w.p.write(folder,'intent.json',dict(argv=argv,deadline=deadline,bound=1200,retry=False))
    with (folder/'stdout.log').open('xb') as stdout,(folder/'stderr.log').open('xb') as stderr:
        before=source()
        child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'worker',str(folder)],cwd=ROOT,
            stdout=stdout,stderr=stderr,stdin=subprocess.DEVNULL,
            env=dict(env,HC074_CONSUMER_ARGV=json.dumps(argv),HC074_CONSUMER_DEADLINE=str(deadline)),
            timeout=max(0,deadline-time.monotonic())+10)
        receipt=json.loads((folder/'execution.json').read_bytes())
        w.p.require(child.returncode==receipt['exit']==0 and receipt['retired'] and not receipt['errors'],
                    'complete actual consumer command: '+label)
        w.p.require(before==source(),'actual consumer source drift')
    return folder


def main(mode,compiler):
    w.p.require(os.name=='nt' and mode in LOCKS and compiler in ('194','188'),'fixed Windows consumer gate')
    out=BASE/('consumer-'+mode+'-'+compiler);out.mkdir(exist_ok=False)
    before=source();head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT)
    w.p.write(out,'intent.json',dict(source=before,head=head.decode(),mode=mode,compiler=compiler,
        session_seconds=1100,command_bound_seconds=1200,retry=False,server=False,services=False))
    deadline=time.monotonic()+1100
    try:
        expected=LOCKS[mode]
        if expected is None:
            prior=(BASE/'root-lock-before').read_bytes()
            old=b'version = "0.13.4"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "e9f068eba8e7071c5f9511831b44f32c740d5adf574e990f946ddb53db2f314e"'
            new=old.replace(b'0.13.4',b'0.13.5').replace(b'e9f068eba8e7071c5f9511831b44f32c740d5adf574e990f946ddb53db2f314e',b'278e9268c93b9e498a2da46c34d90ba93a3255eb99009646398f34bc73ab36eb')
            w.p.require(w.sha(prior)==LOCKS['baseline'] and prior.count(old)==1,'exact consumer predecessor')
            expected=w.sha(prior.replace(old,new))
        w.p.require(w.sha((ROOT/'Cargo.lock').read_bytes())==expected,'exact actual consumer lock view')
        argv,env=command('version',compiler);folder=invoke(out,'version',argv,env,deadline)
        raw=(folder/'stdout.log').read_text();version={'194':'1.94.0','188':'1.88.0'}[compiler]
        w.p.require('release: '+version+'\n' in raw and 'host: x86_64-pc-windows-msvc\n' in raw,'actual consumer compiler')
        argv,env=command('build',compiler);folder=invoke(out,'build',argv,env,deadline)
        images=[]
        for line in (folder/'stdout.log').read_text().splitlines():
            row=json.loads(line)
            if row.get('reason')=='compiler-artifact' and row.get('profile',{}).get('test') and row.get('executable'):
                path=Path(row['executable'])
                w.p.require(path.resolve().is_relative_to(BASE/'consumer-targets'/compiler),'actual held consumer image')
                images.append(dict(path=str(path),sha256=w.sha(path.read_bytes()),target=row['target']))
        w.p.require(images,'nonempty complete consumer inventory');w.p.write(out,'artifacts.json',images)
        counts=[]
        import re
        for index,image in enumerate(images):
            path=Path(image['path']);w.p.require(w.sha(path.read_bytes())==image['sha256'],'held actual consumer test')
            listed=invoke(out,str(index)+'-list',[str(path),'--list'],env,deadline)
            names=[v[:-6] for v in (listed/'stdout.log').read_text().splitlines() if v.endswith(': test')]
            folder=invoke(out,str(index)+'-test',[str(path),'--test-threads=4','--color=never'],env,deadline)
            raw=(folder/'stdout.log').read_text()
            totals=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',raw)
            w.p.require(len(totals)==1,'complete consumer result');passed,failed,ignored,measured,filtered=map(int,totals[0])
            w.p.require(not failed and not measured and not filtered and passed+ignored==len(names),'consumer exact case accounting')
            w.p.require(w.sha(path.read_bytes())==image['sha256'],'held consumer postflight')
            counts.append(dict(target=image['target']['name'],cases=names,passed=passed,ignored=ignored))
        files=generated(compiler)
        for name,body in files.items():
            path=out/'generated'/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(body)
        w.p.write(out,'generated.json',{n:w.sha(v) for n,v in files.items()})
        for action in ('check','clippy'):
            argv,env=command(action,compiler);invoke(out,action,argv,env,deadline)
        w.p.require(before==source() and head==subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT),'complete consumer source/HEAD postflight')
        w.p.require(time.monotonic()<deadline,'original complete consumer deadline')
        w.p.write(out,'result.json',dict(complete=True,counts=counts,source_unchanged=True,head_unchanged=True,release_admitted=False))
        print(json.dumps(dict(mode=mode,compiler=compiler,passed=sum(v['passed'] for v in counts),ignored=sum(v['ignored'] for v in counts),targets=len(counts))))
    except BaseException as error:
        w.p.write(out,'refusal.json',dict(type=type(error).__name__,reason=str(error),release_admitted=False));raise


if __name__=='__main__':
    if len(sys.argv)==3 and sys.argv[1]=='worker':
        folder=Path(sys.argv[2]);deadline=float(os.environ.pop('HC074_CONSUMER_DEADLINE'))
        argv=json.loads(os.environ.pop('HC074_CONSUMER_ARGV'))
        def capacity():
            if __import__('shutil').disk_usage(ROOT).free<w.p.FLOOR:raise w.owned.CapacityError('original four-GiB floor')
        value=w.owned._owned(w.owned.load_job().WindowsJob(),argv,str(ROOT),capacity,bound=1200,deadline=deadline)
        w.p.write(folder,'execution.json',value);raise SystemExit(value['exit'])
    w.p.require(len(sys.argv)==3,'fixed consumer mode/compiler');main(*sys.argv[1:])
