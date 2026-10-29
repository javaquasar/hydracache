"""Restore immutable data/build proof and exact executable export prerequisites."""
import ast
import hashlib
import io
import json
from pathlib import Path
import tomllib
import zipfile

ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
OUT=ROOT/'target/chitchat-host-cost-20261029';LOCAL='capture/chitchat-cost-20261029-r1/'
INDEX='9589ebd9014f7b2be15aa8cfedec635e06d042a4b81fe92da3894fdea1008462'


def need(ok,reason):
    if not ok:raise ValueError(reason)


def sha(raw):return hashlib.sha256(raw).hexdigest()


def put(path,value):
    with path.open('x',encoding='utf8',newline='\n') as out:json.dump(value,out,indent=2,sort_keys=True);out.write('\n')


def function(raw,name):
    text=raw.decode();nodes=[n for n in ast.parse(text).body if isinstance(n,ast.FunctionDef) and n.name==name]
    need(len(nodes)==1,'one original function');return (ast.get_source_segment(text,nodes[0])+'\n').encode()


def main():
    packet=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-cost-20261029'
    raw=(packet/'index.json').read_bytes();need(sha(raw)==INDEX,'sealed exact predecessor');members=json.loads(raw)['members']
    def get(name):
        row=members[name];raw=(packet/'objects'/row['sha256']).read_bytes();need(sha(raw)==row['sha256'] and len(raw)==row['bytes'],'exact archived data');return raw
    prepared=OUT/'prepared';prepared.mkdir(exist_ok=False);dest=prepared/'input';dest.mkdir()
    capsule=get(LOCAL+'source.zip');frame_raw=get(LOCAL+'frame.json');frame=json.loads(frame_raw);frozen=json.loads(get(LOCAL+'format/result.json'))
    need(sha(capsule)==frozen['zip_sha256'] and sha(frame_raw)==frozen['frame_sha256'],'original exact source capsule')
    with zipfile.ZipFile(io.BytesIO(capsule)) as archive:
        need(len(archive.namelist())==len(set(archive.namelist())) and set(archive.namelist())==set(frame)|{'target/chitchat-cost-20261029-r1/frame.json'},'full unique capsule')
        for name in archive.namelist():
            need(not name.startswith('/') and '..' not in name.split('/'),'contained original source')
            body=archive.read(name);need(sha(body)==frame.get(name,sha(frame_raw)),'original source bytes')
            path=dest/'source'/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(body)
    (dest/'frame.json').write_bytes(frame_raw);(dest/'source.zip').write_bytes(capsule)
    for name in ('host.py','remote.py'):(dest/name).write_bytes((HERE/name).read_bytes())
    group=(ROOT/'scripts/ci/watchdog_linux_group.py').read_bytes()
    need(sha(group)=='e5ff52835fc2486171dbdbbb1b6df47177f583b4539f561779992c2c5065a691','original LinuxGroup')
    (dest/'group.py').write_bytes(group)
    owned=(ROOT/'tools/windows-workspace-preflight-074/owned.py').read_bytes()
    need(sha(owned)=='e7c5a3c0c4fea90f39772512c4ad96c60693fc0b482092a608dcf039e151eade','original owned helper')
    (dest/'owned_function.py').write_bytes(function(owned,'_owned'))
    validation=(ROOT/'tools/chitchat-cost-074/analyze.py').read_bytes()
    need(validation==get('source/tools/chitchat-cost-074/analyze.py'),'unchanged predicates/order/limits')
    (dest/'validate_function.py').write_bytes(function(validation,'validate'))
    protocol=(ROOT/'tools/chitchat-cost-074/protocol.json').read_bytes()
    need(protocol==get('source/tools/chitchat-cost-074/protocol.json'),'unchanged preregistration')
    (dest/'protocol.json').write_bytes(protocol)
    path=dest/'semantics/input/expected-caller.json';path.parent.mkdir(parents=True);path.write_bytes(get(LOCAL+'input/expected-caller.json'))
    vendor=json.loads(get(LOCAL+'input/vendor.json'));lock=tomllib.loads(get(LOCAL+'fixtures/Q/Cargo.lock').decode())['package']
    checksums={p['name']+'-'+p['version']:p['checksum'] for p in lock if p.get('source','').startswith('registry+')}
    need(set(vendor)==set(checksums),'whole original locked registry set')
    images={v:json.loads(get(LOCAL+'build-'+v+'/native/artifact.json')) for v in ('B','P','Q')}
    put(prepared/'native-export.json',dict(images=images,vendor=vendor,checksums=checksums))
    for v in ('B','P','Q'):
        path=dest/'build-proofs'/v;path.mkdir(parents=True)
        for name in ('artifact.json','compiler/stdout.log','cargo/stdout.log','imports/stdout.log'):
            target=path/name;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(get(LOCAL+'build-'+v+'/native/'+name))
    put(prepared/'source-review.json',dict(predecessor=INDEX,frame_sha256=sha(frame_raw),zip_sha256=sha(capsule),
        source_files=len(frame),images=images,registry_crates=len(vendor),original_functions_unmodified=True,
        controls_only=True,exclusive_host=False,clock_changes=False,service_changes=False,product=False))
    print(json.dumps(dict(prepared=True,images=3,source_files=len(frame),registry_crates=len(vendor),diagnostics_executed=False)))


if __name__=='__main__':main()
