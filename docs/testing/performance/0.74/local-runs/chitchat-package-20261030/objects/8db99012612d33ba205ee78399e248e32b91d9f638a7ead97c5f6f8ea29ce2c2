"""Pure, pinned security-source projection and package/registry reconciliation."""
import hashlib,importlib.util,json,re,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
BASE=ROOT/'target/chitchat-package-20261030-r3'
NAME='hydra-chitchat';VERSION='0.11.1-hydra.1';PREFIX=NAME+'-'+VERSION
CRATE_INDEX='https://github.com/rust-lang/crates.io-index'
PARENT=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-security-only-20261030'
PARENT_SHA='e6dd71ebb6818b13587b4750360ade76db5484e5083021ca6505699c039373b3'
UPSTREAM_SHA='9e1cfd5d16c6fc43487006ab5b5c910f295cc53cf07d90491fb36c8dc479c754'
CODEC_SHA='1b13c005abfbb96b1374e27fcc871ce0a14d87fe955e2097712a9cfb029fadf3'
sha=lambda raw:hashlib.sha256(raw).hexdigest()
encoded=lambda row:(json.dumps(row,sort_keys=True,indent=2)+'\n').encode()
def need(ok,why):
    if not ok:raise ValueError(why)

def codec():
    path=ROOT/'tools/dbus-source-package-074/package.py';need(sha(path.read_bytes())==CODEC_SHA,'original regular archive codec')
    spec=importlib.util.spec_from_file_location('hc074_package_codec',path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module

def parent():
    raw=(PARENT/'index.json').read_bytes();need(sha(raw)==PARENT_SHA,'exact accepted security-only source packet')
    index=json.loads(raw);result=index['result']
    need(result['complete_functional_matrix'] and result['completed_peers']==12 and result['policy']['U']==[] and result['actual_known_bound_failures']==6,'complete original security compatibility and retained negatives')
    need(not result['root_adopted'] and not result['resources_qualified'],'no original scope promotion')
    def read(name):
        row=index['members'][name];raw=(PARENT/'objects'/row['sha256']).read_bytes()
        need(sha(raw)==row['sha256'] and len(raw)==row['bytes'],'whole accepted source member');return raw
    archive=read('capture/r4/input/archives/chitchat-0.11.1.crate');need(sha(archive)==UPSTREAM_SHA,'exact official base archive')
    upstream=codec().unpack(archive,'chitchat-0.11.1')
    prefix='capture/r4/canonical/U/';u={n[len(prefix):]:read(n) for n in index['members'] if n.startswith(prefix)}
    recipe=json.loads(read('capture/r4/preparation/source-recipe.json'))
    need(len(u)==21 and set(u)==set(upstream) and {n:sha(b) for n,b in u.items()}==recipe['final']['U'],'whole accepted canonical U')
    need(u['src/serialize.rs']==upstream['src/serialize.rs'],'no MTU/writer mutation')
    return upstream,u,read('capture/r4/fixtures/U/Cargo.lock')

def license_bytes():
    raw=(HERE/'inputs/LICENSE').read_bytes()
    blob=hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
    need(blob=='bb24b90b572e6cd6e116591f94c1ccdb383e1257','exact upstream commit license blob');return raw

def manifest(u):
    raw=u['Cargo.toml'];raw=raw[raw.index(b'[package]'):]
    for old,new in [(b'name = "chitchat"\nversion = "0.11.1"',b'name = "'+NAME.encode()+b'"\nversion = "'+VERSION.encode()+b'"'),
      (b'rust-version = "1.86"',b'rust-version = "1.88"'),(b'readme = false',b'readme = "README.md"'),
      (b'repository = "https://github.com/quickwit-oss/chitchat"',b'repository = "https://github.com/javaquasar/hydracache"')]:
        need(raw.count(old)==1,'one metadata preimage');raw=raw.replace(old,new)
    raw=raw.replace(b'[features]',b'publish = ["crates-io"]\ninclude = ["src/**", "tests/**", "README.md", "LICENSE", "BACKPORT074.json", "upstream/**"]\n\n[features]',1)
    need(raw.count(b'resolver = "2"\n')==1,'one inherited package resolver')
    raw=raw.replace(b'resolver = "2"\n',b'')
    raw+=b'\n[workspace]\nresolver = "2"\n'
    return raw

def seed_lock(raw):
    pattern=rb'(?ms)^\[\[package\]\]\nname = "chitchat"\nversion = "0\.11\.1"\n.*?(?=^\[\[package\]\]|\Z)'
    matches=list(re.finditer(pattern,raw));need(len(matches)==1,'one original local lock node')
    old=matches[0].group();need(b'source = ' not in old and old.count(b' "serde_json",\n')==1,'only old diagnostic root edge removed')
    new=old.replace(b'name = "chitchat"',b'name = "'+NAME.encode()+b'"').replace(b'version = "0.11.1"',b'version = "'+VERSION.encode()+b'"').replace(b' "serde_json",\n',b'')
    return raw[:matches[0].start()]+new+raw[matches[0].end():]

def project(u,lock):
    files=dict(u)
    for name in ('Cargo.lock','Cargo.toml.orig','.cargo_vcs_info.json'):files['upstream/'+name]=files.pop(name)
    files['Cargo.toml']=manifest(u);files['Cargo.lock']=seed_lock(lock);files['LICENSE']=license_bytes()
    files['README.md']=b'''# HydraCache security dependency backport (unpublished preparation)

Draft hydra-chitchat 0.11.1-hydra.1 keeps library name chitchat. Based on exact
Quickwit chitchat0.11.1, MIT licensed. Runtime algorithms and wire format remain
original; exact Zstd0.14.3 and LRU0.18.2 replace the old dependency requirements.
Two lifetime annotations and reviewed test-only formatting are unchanged from U.

Upstream MTU bounds still have the recorded pre-existing defect. No MTU fix,
allocation/CPU/RSS claim, Root/release admission or registry publication is made.
See BACKPORT074.json for exact upstream/source/license/maintenance provenance.
The upstream/ directory contains historical inputs, including a stale old lock;
only the top-level Cargo.lock is the active preparation lock.
'''
    files['BACKPORT074.json']=encoded(dict(schema='hc074-chitchat-owned-security-source-v1',name=NAME,version=VERSION,library='chitchat',
      upstream=dict(name='chitchat',version='0.11.1',archive_sha256=UPSTREAM_SHA,commit='c2b98cbea3162a4353c1fc1ffa7fd1e5985aee3a',repository='https://github.com/quickwit-oss/chitchat'),
      parent_sha256=PARENT_SHA,canonical_U={n:sha(b) for n,b in sorted(u.items())},
      license_sha256=sha(files['LICENSE']),license_blob_sha1='bb24b90b572e6cd6e116591f94c1ccdb383e1257',
      maintenance=dict(proposed_owner='HydraCache project maintainer (javaquasar)',review='single-maintainer; not independent review',
        upstream_review='before each release candidate and at least monthly while selected',advisory_mapping='also evaluate upstream chitchat0.11.1 advisories; never use the fork name as an exclusion',
        update='new source/version requires new exact receipt and full gates; never move an exact version',rollback='no Root integration yet; remove only the prepared draft identity, not prior evidence'),
      source_algorithms_unchanged=True,known_MTU_contracts='six original failures retained; unresolved',resources_qualified=False,
      root_adopted=False,registry_qualified=False,release_admitted=False))
    return files

def registry_nodes(raw):return [r for r in tomllib.loads(raw.decode())['package'] if 'source' in r]

def verify_source(files,u):
    expected=(set(u)-{'Cargo.toml.orig','.cargo_vcs_info.json'})|{'upstream/Cargo.lock','upstream/Cargo.toml.orig','upstream/.cargo_vcs_info.json','README.md','LICENSE','BACKPORT074.json'}
    need(set(files)==expected,'exact complete package source inventory')
    need(all(files[n]==body for n,body in u.items() if n.endswith('.rs')),'all runtime/test Rust bodies exact U')
    need(files['Cargo.toml']==manifest(u) and files['LICENSE']==license_bytes(),'exact reviewed metadata/license')
    for n in ('Cargo.lock','Cargo.toml.orig','.cargo_vcs_info.json'):need(files['upstream/'+n]==u[n],'exact historical input')
    row=json.loads(files['BACKPORT074.json'])
    need(row['canonical_U']=={n:sha(b) for n,b in sorted(u.items())} and row['parent_sha256']==PARENT_SHA,'exact canonical source provenance')
    need(all(row[k] is False for k in ('root_adopted','registry_qualified','release_admitted','resources_qualified')),'no scope promotion')
    need(row['upstream']['name']=='chitchat' and row['upstream']['version']=='0.11.1' and row['upstream']['archive_sha256']==UPSTREAM_SHA,'upstream identity not concealed')
    lock=tomllib.loads(files['Cargo.lock'].decode());local=[r for r in lock['package'] if 'source' not in r]
    need(len(local)==1 and local[0]['name']==NAME and local[0]['version']==VERSION,'one owned active lock node')
    return dict(complete=True,source_members=len(files),runtime_changes=0,registry_qualified=False,root_adopted=False)

def registry_row(files,digest):
    doc=tomllib.loads(files['Cargo.toml'].decode());deps=[]
    for section,kind in [('dependencies','normal'),('dev-dependencies','dev')]:
        for name,value in doc.get(section,{}).items():
            need(isinstance(value,dict) and 'path' not in value and 'git' not in value,'only reviewed registry requirements')
            deps.append(dict(name=name,req=value['version'],features=value.get('features',[]),optional=value.get('optional',False),
                default_features=value.get('default-features',True),target=None,kind=kind,registry=CRATE_INDEX,package=value.get('package')))
    return dict(name=NAME,vers=VERSION,deps=deps,features=doc['features'],cksum=digest,yanked=False,links=None,rust_version='1.88',v=2)

def verify_crate(raw,u,source):
    files=codec().unpack(raw,PREFIX)
    need(set(files)==set(source)|{'Cargo.toml.orig'},'whole Cargo package inventory, no stray hooks/VCS/data')
    for name,body in source.items():
        if name not in ('Cargo.toml','Cargo.lock'):need(files[name]==body,'exact Cargo package source member: '+name)
    need(files['Cargo.toml.orig']==source['Cargo.toml'],'exact authored manifest retained')
    doc=tomllib.loads(files['Cargo.toml'].decode());auth=tomllib.loads(source['Cargo.toml'].decode())
    need(doc['package']['name']==NAME and doc['package']['version']==VERSION and 'workspace' not in doc and 'patch' not in doc,'normalized owned registry identity')
    for field in ('dependencies','dev-dependencies','features','lib','test'):need(doc[field]==auth[field],'no normalized feature/API/dependency mutation')
    need(registry_nodes(files['Cargo.lock'])==registry_nodes(source['Cargo.lock']),'same complete packaged registry lock graph')
    need({n:b for n,b in files.items() if n.endswith('.rs')}=={n:b for n,b in u.items() if n.endswith('.rs')},'whole Cargo Rust bodies exact U')
    return files
