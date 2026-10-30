"""Single-edge direct-consumer candidate; no changes to the refused Chitchat Q."""
import tomllib
import common as c

def fixture():
    c.need(c.sha((c.ROOT/'Cargo.lock').read_bytes())==c.OLD_LOCK,'original baseline')
    p=c.BASE/'probe';p.mkdir(exist_ok=False)
    root='../../..'
    (p/'Cargo.toml').write_text('''[package]
name="hc074-supervisor-zstd-wire"
version="0.0.0"
edition="2021"
rust-version="1.88"
publish=false
[lib]
path="'''+root+'''/tools/supervisor-zstd-security-074/probe.rs"
[workspace]
resolver="2"
[dependencies]
hydracache-long-run-supervisor-074={path="'''+root+'''/tools/long-run-supervisor-074"}
sha2="0.11.0"
serde_json="1"
tempfile="3"
zstd_old={package="zstd",version="=0.13.3"}
zstd_new={package="zstd",version="=0.14.3"}
[patch.crates-io]
zbus_macros={path="'''+root+'''/third_party/hc074-dbus/zbus_macros"}
zbus_utils={path="'''+root+'''/third_party/hc074-dbus/zbus_utils"}
''',encoding='utf8',newline='\n')

def candidate(old):
    c.need(c.sha(old)==c.OLD_LOCK,'exact historical preimage')
    blocks=old.split(b'[[package]]\n');head=blocks.pop(0)
    records={}
    for block in blocks:
        row=tomllib.loads('[[package]]\n'+block.decode())['package'][0]
        records[row['name'],row['version']]=block
    expected={'chitchat':b'"zstd",','hydracache-long-run-supervisor-074':b'"zstd",','zstd':b'"zstd-safe",'}
    for (name,version),raw in list(records.items()):
        if name in expected:
            token=expected[name];c.need(raw.count(token)==1,'one old edge: '+name)
            replacement=b'"zstd 0.14.3",' if name=='hydracache-long-run-supervisor-074' else b'"zstd 0.13.3",' if name=='chitchat' else b'"zstd-safe 7.2.4",'
            records[name,version]=raw.replace(token,replacement)
    tested=(c.ROOT/'tools/chitchat-mtu-review-074/patches/tested-fixture.lock').read_bytes()
    for block in tested.split(b'[[package]]\n')[1:]:
        row=tomllib.loads('[[package]]\n'+block.decode())['package'][0]
        if row['name'] in ('zstd','zstd-safe'):
            c.need(c.ARCHIVES[row['name']+'-'+row['version']]==row['checksum'],'already audited exact registry archive')
            if row['name']=='zstd':block=block.replace(b'"zstd-safe",',b'"zstd-safe 8.2.0",')
            key=row['name'],row['version'];c.need(key not in records,'only two new versions');records[key]=block
    return head+b''.join(b'[[package]]\n'+raw for _,raw in sorted(records.items()))

def review(old,new):
    c.need(candidate(old)==new,'whole byte exact candidate, including all historical edges')
    a=tomllib.loads(old.decode())['package'];b=tomllib.loads(new.decode())['package']
    c.need(len(b)==len(a)+2,'exact two registry nodes')
    return dict(baseline_sha256=c.sha(old),candidate_sha256=c.sha(new),baseline_nodes=len(a),candidate_nodes=len(b),
      changed_consumer='unpublished hydracache-long-run-supervisor-074',chitchat_unchanged=True,
      new_registry=c.ARCHIVES,performance_claim=False,denial_policy_unchanged=True)

def apply():
    old=(c.BASE/'baseline/Cargo.lock').read_bytes();new=candidate(old)
    path=c.ROOT/'Cargo.lock';manifest=c.ROOT/(c.PKG+'/Cargo.toml')
    original=(c.BASE/'baseline/supervisor.Cargo.toml').read_bytes()
    c.need(path.read_bytes()==old and manifest.read_bytes()==original,'unchanged baseline before one-edge mutation')
    c.need(original.count(b'zstd = "0.13.3"')==1,'exact one private dependency declaration')
    out=c.BASE/'candidate-input';out.mkdir(exist_ok=False)
    c.write(out,'review.json',review(old,new))
    manifest.write_bytes(original.replace(b'zstd = "0.13.3"',b'zstd = "=0.14.3"'))
    path.write_bytes(new)

if __name__=='__main__':
    import sys
    {'fixture':fixture,'apply':apply}[sys.argv[1]]()
