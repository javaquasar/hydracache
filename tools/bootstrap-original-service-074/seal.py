"""Retain exact non-admitting outcomes and deduplicate their checked sources."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT=Path(__file__).resolve().parents[2]
INPUT=ROOT/'target/bootstrap-original-service-20261023'
OUTPUT=ROOT/'docs/testing/performance/0.74/local-runs/bootstrap-original-service-20261023'
LANES=('baseline','host','host-reviewed','host-final','194','194-final','188-final',
    'metadata-final','metadata-reviewed','account-compatible-controls',
    '194-lookup-candidate','188-lookup-candidate','metadata','seal-controls')
FAILED={'baseline','host','host-reviewed','194'}
NATIVE=('native-first','native-account-compatible-once')
def digest(raw): return hashlib.sha256(raw).hexdigest()
def read(path): return json.loads(path.read_bytes())
def publish(path,value):
    raw=(json.dumps(value,indent=2,sort_keys=True)+'\n').encode()
    with path.open('xb') as stream: stream.write(raw)
def relative(value):
    path=Path(value)
    if path.anchor or value.startswith('/') or '..' in path.parts or '\\' in value or ':' in value:
        raise ValueError('Unsafe evidence path')
    return path
def validate_lane(folder,failed):
    result=read(folder/'result.json')
    if bool(result['failure']) != failed: raise ValueError('Unexpected check outcome')
    before=read(folder/'source-before.json')
    for name,sha in before.items():
        if digest((folder/'source'/relative(name)).read_bytes()) != sha:
            raise ValueError('Checked source snapshot drift')
    if not failed and read(folder/'source-after.json') != before:
        raise ValueError('Green check source drift')
    for step in result['steps']:
        receipt=read(folder/(step+'.json'))
        for stream in ('stdout','stderr'):
            if digest((folder/(step+'.'+stream+'.log')).read_bytes()) != receipt[stream+'_sha256']:
                raise ValueError('Check log drift')
        if not failed and receipt['exit'] != 0: raise ValueError('Green check contains refusal')
    return result
def verify(folder):
    index=read(folder/'index.json')
    for name,entry in index['files'].items():
        relative(name)
        sha=entry['sha256']
        if len(sha)!=64 or any(c not in '0123456789abcdef' for c in sha):
            raise ValueError('Unsafe object digest')
        raw=(folder/'objects'/sha).read_bytes()
        if digest(raw)!=sha or len(raw)!=entry['bytes']: raise ValueError('Evidence object drift')
    if index['admission'] or index['server'] or index['product']: raise ValueError('Not an admission packet')
    return index
def run():
    results={lane:validate_lane(INPUT/lane,lane in FAILED) for lane in LANES}
    for lane in NATIVE:
        outcome=read(INPUT/lane/'result.json')
        if outcome['returncode'] not in (74,101) or outcome['original_send_observed'] or outcome['readiness_observed']:
            raise ValueError('Unexpected native outcome; never relabel admission')
    OUTPUT.mkdir(parents=True,exist_ok=False)
    (OUTPUT/'objects').mkdir()
    # Raw logs/source bytes remain unchanged in Git on every checkout.
    with (OUTPUT/'.gitattributes').open('xb') as stream:
        stream.write(b'objects/* -text whitespace=cr-at-eol,-blank-at-eof\nindex.json text eol=lf\n')
    files={}
    for lane in LANES+NATIVE:
        for source in sorted((INPUT/lane).rglob('*')):
            if source.is_symlink(): raise ValueError('Evidence symlink')
            if not source.is_file() or (lane in NATIVE and source.name in ('bootstrap','libtest')): continue
            raw=source.read_bytes();sha=digest(raw)
            dest=OUTPUT/'objects'/sha
            if not dest.exists():
                with dest.open('xb') as stream: stream.write(raw)
            files[source.relative_to(INPUT).as_posix()]={'sha256':sha,'bytes':len(raw)}
    index={'schema':'hc074-original-service-evidence-v1','files':files,
        'checks':results,'admission':False,'server':False,'product':False,
        'qualification_sha256':digest((ROOT/'docs/testing/performance/0.74/qualification-manifest.toml').read_bytes()),
        'seal_source':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        'original_attempt_source':read(INPUT/'native-account-compatible-once/intent.json')['git']}
    publish(OUTPUT/'index.json',index)
    verify(OUTPUT)
    print(json.dumps({'files':len(files),'objects':len(list((OUTPUT/'objects').iterdir())),
        'index_sha256':digest((OUTPUT/'index.json').read_bytes()),'admission':False}))
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode',choices=['retain','verify'])
    args=parser.parse_args()
    run() if args.mode=='retain' else print(json.dumps({'verified':len(verify(OUTPUT)['files'])}))
