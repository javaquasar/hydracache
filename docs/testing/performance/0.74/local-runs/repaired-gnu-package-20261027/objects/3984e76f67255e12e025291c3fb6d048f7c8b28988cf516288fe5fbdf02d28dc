"""Fixed repaired GNU source/environment review; no test dispatcher."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess

CONTROL = Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
SOURCE = 'fe4b3a0f9060615185895337181780a6f3f55439'
ROOT = Path('/home/artur/.cache/hydracache-074-frontier-native/636feb166b273b5c3d6fc0b93c4fccf57560cf03/repo')
TARGET = Path('/home/artur/.cache/hydracache-074-194-check')
DATA_SHA = '59f8d00f06e3468626b686201957615fa2b1fbb7bfec4020b63bb863aa48ec8a'
CONTEXT_SHA = 'c4bbc2d6b69e416ac3fcb30a1a64fb3bb54473b70977a1875ff90ca264b0cd44'
REQUEST_SHA = '8b7cb4fea8fa4e6f5c61d1905cf852ba7219913b168e1c32ba229c01d3125383'
PINS = {
    'tools/native-frontier-074/contract.py': '13a92353314ab7b6b3f443f4d8244b823e38f390a35885edd050bbb73a5c2353',
    'tools/native-frontier-074/native.py': '8badd8357fd491b01e65c3c118421cdd8f41eb952909678c97f78ac2bffe3769',
    'tools/package-closure-offline-074/analysis.py': 'dd53d98e0b299e892a0ccd3597335409a3c3e476db08dd2317706918e1092474',
    'tools/package-closure-offline-074/gnu_review.py': '816c467195b4d58ee1e25cd530258b15241023cf61bbea56ac284bf80b21c1c4',
    'tools/historical-blob-batch-074/module_context_v5.py': CONTEXT_SHA,
}

def require(value, reason):
    if not value:
        raise ValueError(reason)

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def canonical(value):
    return sha(json.dumps(value,sort_keys=True,separators=(',',':')).encode())

def load(path, name, expected):
    require(sha(path.read_bytes()) == expected, 'helper body drift: '+str(path))
    spec = importlib.util.spec_from_file_location(name,path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

def inputs():
    raw = Path(__file__).with_name('inputs.json').read_bytes()
    require(sha(raw) == DATA_SHA, 'review input bytes drift')
    value = json.loads(raw)
    require(value['source'] == SOURCE and value['request_sha256'] == REQUEST_SHA, 'review identity drift')
    require((len(value['sources']),len(value['auxiliary']),len(value['targets'])) == (391,58,155), 'review census drift')
    return value

def check_bytes(root, expected):
    for name,digest in expected.items():
        path = root/name
        require(path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root.resolve()),'source path custody')
        require(sha(path.read_bytes().replace(b'\r\n',b'\n')) == digest,'reviewed complete body drift: '+name)

def source_review(root, data, document, context):
    require(canonical(document) == data['syntax_sha256'], 'syntax inventory drift')
    require(not document['executed_input_code'] and not document['compiler_expansion'], 'source-only scope')
    require({r['source'] for r in document['sources']} == set(data['sources']), 'syntax membership drift')
    check_bytes(root, {**data['sources'],**data['auxiliary']})
    additions,unresolved = context.module_edges(root,document)
    require(not additions and not unresolved,'module closure unresolved')
    discovered = {p.relative_to(root).as_posix() for folder in ('crates/xtask/src','crates/xtask/tests') for p in (root/folder).rglob('*.rs')}
    require(discovered <= data['sources'].keys(),'new uncensused Rust source')
    targets = ['unittests src/lib.rs','unittests src/main.rs']+sorted('tests/'+p.name for p in (root/'crates/xtask/tests').glob('*.rs'))
    require(targets == data['targets'],'target catalog drift')
    forbidden = {'literal:ssh','literal:systemctl','literal:systemd-run','literal:wsl','literal:timedatectl'}
    for row in document['sources']:
        for site in row['sites']:
            require(site['kind'] != 'include','unexpanded include edge')
            if site['kind'] in ('command-constructor','macro-command-constructor'):
                require(site['detail'] not in forbidden,'host/service constructor')
    return dict(sources=391,auxiliary=58,targets=targets,sites=sum(len(r['sites']) for r in document['sources']),
        changed=data['changed'],added=data['added'],delta_review=data['delta_review'],
        source_effect_review_complete=True,module_closure_complete=True,compiler_expansion=False,
        tests_dispatched=False,package_pass=False,release_admitted=False)

def space():
    require(shutil.disk_usage('/mnt/c').free >= 4*1024**3,'physical 4 GiB floor')
    require(shutil.disk_usage(ROOT).free >= 4*1024**3,'native 4 GiB floor')

def review():
    import sys
    require(sys.platform.startswith('linux') and os.geteuid()==1000,'nonroot local GNU only')
    require(ROOT.resolve()==ROOT and (ROOT/'.git').is_dir(),'native root custody')
    blocked=[k for k in os.environ if k.startswith(('CARGO_','RUST_','RUSTFLAGS','HC074_','HYDRACACHE_','GIT_','LD_')) and not(k=='GIT_PAGER' and os.environ[k]=='cat')]
    require(not blocked and not any(k in os.environ for k in ('PYTHONPATH','PYTHONHOME')),'inherited environment override')
    for name,digest in PINS.items():
        require(sha((CONTROL/name).read_bytes())==digest,'review helper drift')
    sys.path.insert(0,str(CONTROL/'tools/native-frontier-074'))
    import native
    import contract
    request_raw=(CONTROL/'target/native-frontier-20261027'/SOURCE/'request.json').read_bytes()
    require(sha(request_raw)==REQUEST_SHA,'request drift')
    request=contract.request(contract.decode(request_raw))
    require(native.git(ROOT,'rev-parse','HEAD').decode().strip()==SOURCE,'native HEAD drift')
    require(contract.refs(native.git(ROOT,'for-each-ref','--format=%(objectname) %(refname)'))==request['refs'],'native refs drift')
    require(not (ROOT/'.git/objects/info/alternates').exists() and not (ROOT/'.git/commondir').exists(),'borrowed database')
    native.validate_files(ROOT,request['tree'],space)
    configuration=subprocess.check_output(['/usr/bin/git','config','--includes','--show-origin','--list'],cwd=ROOT,timeout=10)
    require(sha(configuration)=='252bf249a0ada7296a65c459df81ec7be0f82ed4d3b84bb6b0cb2d4f59e2ce02','actual Git config drift')
    configs=[p/'.cargo'/n for p in (ROOT,*ROOT.parents) for n in ('config','config.toml')]
    configs += [Path('/home/artur/.cargo')/n for n in ('config','config.toml')]
    require(not any(p.exists() for p in configs if p != ROOT/'.cargo/config.toml'),'shadow Cargo configuration')
    require(TARGET.is_dir() and TARGET.resolve()==TARGET and TARGET.stat().st_uid==os.geteuid(),'target custody')
    # These are the four ignored scratch outputs left by the complete prior package.
    scratch=ROOT/'target'
    require(scratch.is_dir() and not scratch.is_symlink() and scratch.stat().st_uid==os.geteuid(),'scratch custody')
    require({p.name for p in scratch.iterdir()}=={'test-evidence','hc2-python-generation','hc2-ci-test','.rustc_info.json'},'scratch catalog drift')
    require(not any(p.is_symlink() for p in scratch.rglob('*')),'scratch symlink')
    for name in ('python3','git','kill'):
        require(shutil.which(name)=='/usr/bin/'+name,'fixture executable resolution drift')
    toolchain=subprocess.check_output(['/home/artur/.cargo/bin/rustc','+1.94.0','--version','--verbose'],timeout=15,env=dict(os.environ,RUSTUP_AUTO_INSTALL='0')).decode()
    require('host: x86_64-unknown-linux-gnu\n' in toolchain and 'release: 1.94.0\n' in toolchain,'toolchain drift')
    context=load(CONTROL/'tools/historical-blob-batch-074/module_context_v5.py','checked_repaired_context',CONTEXT_SHA)
    data=inputs()
    document=json.loads((CONTROL/'target/gnu-repaired-local-20261027/syntax-current/result.json').read_bytes())
    result=source_review(ROOT,data,document,context)
    sys.path.insert(0,str(CONTROL/'tools/package-closure-offline-074'))
    inherited=load(CONTROL/'tools/package-closure-offline-074/gnu_review.py','checked_inherited_gnu_groups',PINS['tools/package-closure-offline-074/gnu_review.py'])
    result.update(source=SOURCE,request_sha256=REQUEST_SHA,git_config_sha256=sha(configuration),
        manual_inherited_gnu_groups=inherited.GROUPS,environment_preflight_complete=True,toolchain=toolchain,
        raw_path=os.environ['PATH'],raw_cargo_resolution=shutil.which('cargo'),python3=shutil.which('python3'),
        physical_free=shutil.disk_usage('/mnt/c').free,native_free=shutil.disk_usage(ROOT).free,
        package_bound_seconds=780,test_threads=4,disk_floor_bytes=4*1024**3,
        capacity_is_not_build_reservation=True,existing_ignored_scratch_reviewed=True,
        services=False,clocks=False,server=False,qualification=False)
    return result

if __name__=='__main__':
    print(json.dumps(review(),indent=2,sort_keys=True))
