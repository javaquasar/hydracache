"""Standalone data-only continuation; archived producers are never imported."""
import importlib.util,json,sys
from pathlib import Path
import review as r
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('hc074_active_copied_root_packet',ROOT/'tools/chitchat-copied-root-evidence-074/packet.py')
historical=importlib.util.module_from_spec(spec);spec.loader.exec_module(historical)
# Only the current active helper is imported, never an archived producer.
OUT=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-copied-root-native-20261030'
PARENT='160ea3e77442cc8319ac43b5a2061a78d62a1ee96180d985b34e7f6d564834b9'
VERIFY='4a78c606450165d278fdf5272f4a7b58606fd27c7cf4056d006f5e2a3ce2678d'
HERE=Path(__file__).resolve().parent

def validate(index,read):
    r.need(index['schema']=='hc074-copied-root-native-v1' and index['parent_sha256']==PARENT and index['parent_verification_sha256']==VERIFY,'exact immutable continuation parents')
    data=historical.validate(dict(index,schema='hc074-chitchat-copied-root-v1'),read)
    r.need(r.reconcile(data)==index['native_result'],'independent actual metadata/command outcomes')
    return data

def retain():
    data=historical.collect()
    for path in historical.walk(HERE):data['source/current/'+path.relative_to(ROOT).as_posix()]=path.read_bytes()
    for folder,digest,label in (('chitchat-copied-root-20261030',PARENT,'primary'),('chitchat-copied-root-verification-20261030',VERIFY,'verification')):
        raw=(OUT.parent/folder/'index.json').read_bytes();r.need(historical.sha(raw)==digest,'whole original parent index');data['parents/'+label+'.json']=raw
    result=historical.reconcile(data);native=r.reconcile(data)
    index=dict(schema='hc074-copied-root-native-v1',parent_sha256=PARENT,parent_verification_sha256=VERIFY,result=result,native_result=native,
        ordinary_archives_excluded=True,compiled_images_excluded=True,members={n:dict(sha256=historical.sha(b),bytes=len(b)) for n,b in sorted(data.items())})
    objects={historical.sha(b):b for b in data.values()};validate(index,objects.__getitem__)
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir()
    for h,b in objects.items():(OUT/'objects'/h).write_bytes(b)
    raw=historical.encoded(index);(OUT/'index.json').write_bytes(raw)
    (OUT/'.gitattributes').write_bytes(b'objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n')
    print(json.dumps(dict(sha256=historical.sha(raw),members=len(data),objects=len(objects),result=native)))

def verify(digest):
    raw=(OUT/'index.json').read_bytes();r.need(historical.sha(raw)==digest,'whole external native result digest');index=json.loads(raw)
    data=validate(index,lambda h:(OUT/'objects'/h).read_bytes());print(json.dumps(dict(complete=True,result=index['native_result'])))
    return data

if __name__=='__main__':
    if sys.argv[1]=='retain':retain()
    else:r.need(sys.argv[1]=='verify' and len(sys.argv)==3,'fixed native data-only action');verify(sys.argv[2])
