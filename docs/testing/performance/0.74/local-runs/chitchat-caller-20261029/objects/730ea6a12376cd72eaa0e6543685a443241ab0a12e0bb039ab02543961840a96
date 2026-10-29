"""Byte-restored fixtures and source custody for the isolated native observer."""
from pathlib import Path
import hashlib
import importlib.util
import json
import os
import re
import shutil
import sys
import tomllib
import zipfile

ROOT=Path(__file__).resolve().parents[2]
TOOLS=ROOT/'tools/chitchat-native-cost-074'
RELBASE='target/chitchat-native-cost-opaque-20261029'
BASE=ROOT/RELBASE
REAL=ROOT if os.name=='nt' else Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
NATIVE=Path('/home/artur/.cache/hydracache-074-chitchat-c-native-target/20261029')
COPY=NATIVE/'source-root-opaque'
sha=lambda raw:hashlib.sha256(raw).hexdigest()
sys.path.insert(0,str(ROOT/'tools/windows-workspace-preflight-074'))
import owned

PINS={'Cargo.lock':'34d7a21d9833a180460e5a094f9ee7f1effe07dfb43a65d90865798963e0070c',
      'deny.toml':'16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930',
      'docs/testing/performance/0.74/qualification-manifest.toml':'11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc',
      'scripts/ci/watchdog_windows_job.py':'8b392811e106eb7e692e21ee5b55cd04ea8ac421cad015edb66ce736b08c2ab1',
      'scripts/ci/watchdog_linux_group.py':'e5ff52835fc2486171dbdbbb1b6df47177f583b4539f561779992c2c5065a691',
      'tools/windows-workspace-preflight-074/owned.py':'e7c5a3c0c4fea90f39772512c4ad96c60693fc0b482092a608dcf039e151eade',
      'tools/dbus-source-package-074/package.py':'1b13c005abfbb96b1374e27fcc871ce0a14d87fe955e2097712a9cfb029fadf3'}


def need(value,reason):
    if not value:raise ValueError(reason)


def write(folder,name,value):
    with (folder/name).open('x',encoding='utf8',newline='\n') as out:json.dump(value,out,indent=2,sort_keys=True);out.write('\n')


def files(folder):
    result={}
    for p in sorted(folder.rglob('*')):
        need(not p.is_symlink() and not getattr(p,'is_junction',lambda:False)(),'regular owned input')
        if p.is_file():result[p.relative_to(folder).as_posix()]=sha(p.read_bytes())
    return result


def load(path,name):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module


def source():
    names=set(PINS)|{'Cargo.toml','.cargo/config.toml'}
    names|={p.relative_to(ROOT).as_posix() for p in TOOLS.iterdir() if p.is_file()}
    names|={p.relative_to(ROOT).as_posix() for p in (ROOT/'tools/windows-workspace-preflight-074').iterdir() if p.is_file()}
    result={n:sha((ROOT/n).read_bytes()) for n in sorted(names)}
    for n,digest in PINS.items():need(result[n]==digest,'exact root/helper pin: '+n)
    return result


def capacity():
    if shutil.disk_usage(REAL).free<4*1024**3 or os.name!='nt' and NATIVE.exists() and shutil.disk_usage(NATIVE).free<4*1024**3:
        raise owned.CapacityError('original physical/native four-GiB floors')


def packet():
    folder=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-inplace-20261029'
    raw=(folder/'index.json').read_bytes()
    need(sha(raw)=='9381608b814a7fcdc16835ece436534fc1d48f816039023f9c0adbc16a234fd1','sealed previous source/observations')
    members=json.loads(raw)['members']
    def get(name):
        row=members[name];body=(folder/'objects'/row['sha256']).read_bytes()
        need(sha(body)==row['sha256'] and len(body)==row['bytes'],'sealed source member: '+name)
        return body
    return get


def diagnostic(raw,marker):
    rows=[v[len(marker):] for v in raw.splitlines() if v.startswith(marker)]
    need(len(rows)==1,'unique complete diagnostic: '+marker)
    return json.loads(rows[0])


def semantics(rows):
    original=json.loads((BASE/'input/cases.json').read_bytes())
    project=lambda data:[tuple(v[k] for k in ('mtu','value_len','accepted','wire')) for v in data]
    need(len(rows)==239 and len({(r['mtu'],r['value_len']) for r in rows})==239 and project(rows)==project(original),'original workload/order/admissions/wire')


def prepare():
    get=packet();input_dir=BASE/'input';input_dir.mkdir()
    old=json.loads(get('capture/preparation/B/result.json'))
    final=json.loads(get('capture/preparation/L3/result.json'))
    need(set(old['files'])==set(final['files']),'same original complete source file set')
    for variant,metadata in (('B',old),('P',final)):
        folder=BASE/'fixtures'/variant;folder.mkdir(parents=True)
        for name in metadata['files']:
            owner='B' if variant=='B' and name in ('src/serialize.rs','src/delta.rs') else 'L3'
            body=get('capture/fixtures/'+owner+'/'+name)
            path=folder/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(body)
    for name in final['overlays']:
        body=get('source/'+name);need((ROOT/name).read_bytes()==body,'unchanged old overlay')
    old_body=get('source/tools/chitchat-inplace-074/allocation_tests_portable.rs')
    body=old_body.decode()
    substitutions=[('unsafe { System.alloc(layout) }','tag_system(|| unsafe { System.alloc(layout) })'),
                   ('unsafe { System.alloc_zeroed(layout) }','tag_system(|| unsafe { System.alloc_zeroed(layout) })'),
                   ('unsafe { System.dealloc(ptr, layout) };','tag_system(|| unsafe { System.dealloc(ptr, layout) });'),
                   ('unsafe { System.realloc(ptr, layout, size) }','tag_system(|| unsafe { System.realloc(ptr, layout, size) })')]
    for before,after in substitutions:
        need(body.count(before)==1,'one delegating System call');body=body.replace(before,after)
    body+='\n'+(TOOLS/'native_tests.rs').read_text(encoding='utf8')
    (BASE/'observer.rs').write_text(body,encoding='utf8',newline='\n')
    for variant in ('B','P'):
        path=BASE/'fixtures'/variant/'src/delta.rs';text=path.read_text(encoding='utf8')
        marker=re.search(r'#\[path = "([^"]+allocation_tests(?:_portable)?\.rs)"\]\nmod cost_review;',text)
        need(marker is not None,'one old observer module')
        relative=os.path.relpath(BASE/'observer.rs',path.parent).replace('\\','/')
        text=text[:marker.start(1)]+relative+text[marker.end(1):]
        path.write_text(text,encoding='utf8',newline='\n')
    baseline=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-mtu-backport-20261028'
    raw=(baseline/'index.json').read_bytes();need(sha(raw)=='d6fc98b961530fdc3b879018521500c80d65094e7f160263b7318fa8870d5dfe','original semantic packet')
    row=json.loads(raw)['members']['capture/delta-B-194/stdout.log'];data=(baseline/'objects'/row['sha256']).read_bytes()
    need(sha(data)==row['sha256'],'original semantic bytes');write(input_dir,'cases.json',diagnostic(data.decode(),'HC074_SUPPORTED_DELTA:')['cases'])
    extra=['delta::cost_review::'+n for n in ('native_c_allocator_control','native_rust_tag_control','native_operation_observations')]
    for variant,label in (('B','build-release-B-194'),('P','build-L3-194')):
        catalog=json.loads(get('capture/'+label+'/catalog.json'))
        write(input_dir,'catalog-'+variant+'.json',sorted([n for n in catalog if n!='delta::cost_review::cpu_observations']+extra))
    vendor=json.loads(get('capture/vendor-receipt.json'));write(input_dir,'vendor.json',vendor)
    (input_dir/'archives').mkdir()
    for name in vendor:(input_dir/'archives'/(name+'.crate')).write_bytes(get('capture/input/archives/'+name+'.crate'))
    previous=ROOT/'target/chitchat-native-cost-target-20261029'
    previous_frame=json.loads((previous/'frame.json').read_bytes())
    for name in ('wrap.c','wrap.h','control.c'):
        relative=(TOOLS/name).relative_to(ROOT).as_posix()
        need(sha((TOOLS/name).read_bytes())==previous_frame[relative],'unchanged actual C wrapper/control source before reuse')
    write(input_dir,'reuse-object.json',json.loads((previous/'c-control/native/object.json').read_bytes()))
    names=set(source())|set(final['overlays'])
    names|={p.relative_to(ROOT).as_posix() for p in (BASE/'fixtures').rglob('*') if p.is_file()}
    names|={p.relative_to(ROOT).as_posix() for p in input_dir.rglob('*') if p.is_file()}
    names.add((BASE/'observer.rs').relative_to(ROOT).as_posix())
    write(BASE,'input-names.json',sorted(names))


def freeze():
    names=json.loads((BASE/'input-names.json').read_bytes())
    frame={n:sha((ROOT/n).read_bytes()) for n in names};write(BASE,'frame.json',frame)
    with zipfile.ZipFile(BASE/'source.zip','x',compression=zipfile.ZIP_DEFLATED) as archive:
        for name,digest in frame.items():
            raw=(ROOT/name).read_bytes();need(sha(raw)==digest,'frozen source while packaging');archive.writestr(name,raw)
        archive.writestr(RELBASE+'/frame.json',(BASE/'frame.json').read_bytes())
    return dict(frame_sha256=sha((BASE/'frame.json').read_bytes()),zip_sha256=sha((BASE/'source.zip').read_bytes()),files=len(frame))


def native_guard():
    capacity();raw=(BASE/'frame.json').read_bytes()
    need(sha(raw)==os.environ['HC074_NATIVE_FRAME'],'original externally held source frame')
    for name,digest in json.loads(raw).items():need(sha((ROOT/name).read_bytes())==digest,'fresh actual source: '+name)
    need((NATIVE/'cargo-home/config.toml').read_text()=='[source.crates-io]\nreplace-with="native-cost"\n[source.native-cost]\ndirectory="'+(NATIVE/'vendor').as_posix()+'"\n','exact native offline configuration')
    lock=tomllib.loads((BASE/'fixtures/P/Cargo.lock').read_text())['package']
    checks={v['name']+'-'+v['version']:v['checksum'] for v in lock if v.get('source','').startswith('registry+')}
    for name,hashes in json.loads((BASE/'input/vendor.json').read_bytes()).items():
        actual=files(NATIVE/'vendor'/name)
        expected=dict(hashes);checksum=(json.dumps(dict(package=checks[name],files=hashes),indent=2,sort_keys=True)+'\n').encode()
        expected['.cargo-checksum.json']=sha(checksum)
        need(actual==expected,'fresh complete actual registry source: '+name)


def stage(frame_sha):
    need(os.getuid()==os.getgid()==1000,'fixed native nonroot owner');capacity()
    need(NATIVE.exists(),'explicit existing qualified native object context')
    need(NATIVE.resolve()==NATIVE and NATIVE.stat().st_uid==1000,'private native path')
    frozen=json.loads((BASE/'format/result.json').read_bytes())
    raw=(BASE/'source.zip').read_bytes();need(sha(raw)==frozen['zip_sha256'],'exact source capsule')
    frame_raw=(BASE/'frame.json').read_bytes();need(sha(frame_raw)==frame_sha,'held frame');frame=json.loads(frame_raw)
    COPY.mkdir(mode=0o700)
    with zipfile.ZipFile(BASE/'source.zip') as archive:
        expected=set(frame)|{RELBASE+'/frame.json'}
        need(set(archive.namelist())==expected and len(archive.namelist())==len(expected),'exact unique source capsule')
        for name in archive.namelist():
            path=COPY/name;need(path.resolve().is_relative_to(COPY),'contained source copy');body=archive.read(name)
            need(sha(body)==frame.get(name,frame_sha),'copied source bytes')
            path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(body)
    held=json.loads((COPY/RELBASE/'input/reuse-object.json').read_bytes())
    need(held['path']==str(NATIVE/'wrap.o') and sha((NATIVE/'wrap.o').read_bytes())==held['sha256'],'unchanged held C object; never overwrite earlier context')
