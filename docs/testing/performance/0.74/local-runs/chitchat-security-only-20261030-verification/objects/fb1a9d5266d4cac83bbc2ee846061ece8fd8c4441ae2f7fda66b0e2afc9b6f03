"""Three exact inputs: shared annotation-only B, Zstd-only Z, LRU-only U."""
import difflib
import json
from pathlib import Path
import tomllib
import scope as c

def original(raw):
    c.need(c.sha(raw)==c.BASE_SHA,'exact official upstream archive')
    codec=c.load(c.ROOT/'tools/dbus-source-package-074/package.py','security_only_regular_codec')
    return codec.unpack(raw,'chitchat-0.11.1')

def replace(files,path,old,new):
    c.need(files[path].count(old)==1,'one exact preimage: '+path);files[path]=files[path].replace(old,new)

# Exact previously reviewed L/L2 test-only format captures; no assertion changes.
TEST_STYLE={'src/lib.rs':[(b'format!("k{}", i)',b'format!("k{i}")',1),
 (b'panic!("Expected SynAck, got {:?}", ack)',b'panic!("Expected SynAck, got {ack:?}")',1)],
 'tests/perf_test.rs':[(b'"Delay exceeded: {:?}",\n        delay',b'"Delay exceeded: {delay:?}"',5)],
 'tests/cluster_test.rs':[
 (b'write!(f, "Key \'{}\' == \'{}\'", key, expected_value)',b'write!(f, "Key \'{key}\' == \'{expected_value}\'")',1),
 (b'write!(f, "INS KV in node {:?}: ", chitchat_id)',b'write!(f, "INS KV in node {chitchat_id:?}: ")',1),
 (b'write!(f, "({}={})", key, value)',b'write!(f, "({key}={value})")',1),
 (b'write!(f, "DEL node {:?} key {}", chitchat_id, key)',b'write!(f, "DEL node {chitchat_id:?} key {key}")',1),
 (b'write!(f, "WAIT {:?}", duration)',b'write!(f, "WAIT {duration:?}")',1),
 (b'"Node {:?} assertion failed",\n                            expected_status,',b'"Node {expected_status:?} assertion failed",',1),
 (b'format!("node-{}", i)',b'format!("node-{i}")',1),
 (b'format!("key_{}", idx)',b'format!("key_{idx}")',1)]}

def test_style(files,reverse=False):
    for path,edits in TEST_STYLE.items():
        for before,after,count in edits:
            old,new=(after,before) if reverse else (before,after)
            c.need(files[path].count(old)==count,'exact test-only format preimage: '+path)
            files[path]=files[path].replace(old,new)

def variants(raw):
    upstream=original(raw);b=dict(upstream)
    replace(b,'src/lib.rs',b'-> Option<KeyChangeEvent> {',b"-> Option<KeyChangeEvent<'_>> {")
    replace(b,'src/delta.rs',b'fn as_ref(&self) -> DeltaOpRef {',b"fn as_ref(&self) -> DeltaOpRef<'_> {")
    test_style(b)
    z=dict(b)
    replace(z,'Cargo.toml',b'[dependencies.zstd]\nversion = "0.13"',b'[dependencies.zstd]\nversion = "=0.14.3"')
    replace(z,'Cargo.toml.orig',b'zstd = "0.13"',b'zstd = "=0.14.3"')
    u=dict(z)
    replace(u,'Cargo.toml',b'[dependencies.lru]\nversion = "0.17"',b'[dependencies.lru]\nversion = "=0.18.2"')
    replace(u,'Cargo.toml.orig',b'lru = "0.17"',b'lru = "=0.18.2"')
    for name,files in [('B',b),('Z',z),('U',u)]:
        c.need(set(files)==set(upstream),'same complete upstream member inventory')
        c.need(files['src/serialize.rs']==upstream['src/serialize.rs'],'unchanged bound/writer/compression algorithms')
        changed={n for n in files if files[n]!=upstream[n]}
        common={'src/lib.rs','src/delta.rs','tests/perf_test.rs','tests/cluster_test.rs'}
        c.need(changed==(common if name=='B' else common|{'Cargo.toml','Cargo.toml.orig'}),'only annotations/test formatting/manifests change')
    return upstream,dict(B=b,Z=z,U=u)

def prepare():
    out=c.BASE/'preparation';out.mkdir(exist_ok=False);inputs=c.BASE/'input';inputs.mkdir();(inputs/'archives').mkdir()
    raw=(Path.home()/'.cargo/registry/cache/index.crates.io-1949cf8c6b5b557f/chitchat-0.11.1.crate').read_bytes()
    (inputs/'archives/chitchat-0.11.1.crate').write_bytes(raw)
    upstream,rows=variants(raw);codec=c.load(c.ROOT/'tools/dbus-source-package-074/package.py','security_only_source_codec')
    c.write(out,'upstream.json',{n:c.sha(b) for n,b in upstream.items()})
    expected={};stacks=[]
    for name,files in rows.items():
        folder=c.BASE/'canonical'/name;folder.mkdir(parents=True)
        for n,body in files.items():
            p=folder/n;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(body)
        tar=codec.pack(files,'chitchat-0.11.1');(out/(name+'-source-only.tar')).write_bytes(tar)
        expected[name]={n:c.sha(b) for n,b in files.items()}
        parent=upstream if name=='B' else rows['B'] if name=='Z' else rows['Z']
        changes=[]
        for n in sorted(files):
            if files[n]==parent[n]:continue
            changes.append(dict(path=n,before=c.sha(parent[n]),after=c.sha(files[n])))
            (out/(name+'-'+n.replace('/','_')+'.diff')).write_text(''.join(difflib.unified_diff(parent[n].decode().splitlines(True),files[n].decode().splitlines(True),fromfile='a/'+n,tofile='b/'+n)),encoding='utf8',newline='\n')
        stacks.append(dict(variant=name,changes=changes,archive_sha256=c.sha(tar)))
        fixture=c.BASE/'fixtures'/name;fixture.mkdir(parents=True)
        for n,body in files.items():
            p=fixture/n;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(body)
        manifest=files['Cargo.toml'].replace(b'resolver = "2"\n',b'')
        manifest+=b'\n[dev-dependencies.serde_json]\nversion="1"\n[workspace]\nresolver="2"\n'
        (fixture/'Cargo.toml').write_bytes(manifest)
        (fixture/'Cargo.lock').write_bytes((c.ROOT/'Cargo.lock').read_bytes())
        for target,old in [('src/stream_review.rs','tools/chitchat-mtu-review-074/stream_tests.rs'),('src/delta_review.rs','tools/chitchat-mtu-review-074/delta_tests.rs')]:
            (fixture/target).write_bytes((c.ROOT/old).read_bytes())
        (fixture/'src/security_review.rs').write_bytes((c.HERE/'security_tests.rs').read_bytes())
        with (fixture/'src/serialize.rs').open('ab') as f:f.write(b'\n#[cfg(test)]\n#[path="stream_review.rs"]\nmod security_stream_review;\n')
        with (fixture/'src/delta.rs').open('ab') as f:f.write(b'\n#[cfg(test)]\n#[path="delta_review.rs"]\nmod security_delta_review;\n')
        with (fixture/'src/lib.rs').open('ab') as f:f.write(b'\n#[cfg(test)]\nmod security_review;\n')
    c.write(out,'source-recipe.json',dict(schema='hc074-chitchat-security-only-v1',base_sha256=c.BASE_SHA,upstream={n:c.sha(b) for n,b in upstream.items()},final=expected,steps=stacks,
      mtu_algorithms_unchanged=True,q_adopted=False,root_adopted=False,wide_gate_passed=False,registry_qualified=False))

if __name__=='__main__':prepare()
