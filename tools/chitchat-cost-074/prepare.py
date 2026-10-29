"""Restore identical runtime bodies and add only normal diagnostic entrypoints."""
import json
import common as c

INDEX='874ba6355b98229cbeab11c5b0742e439cbced847e6954478234e10e92d7ef56'
PREFIX='capture/chitchat-caller-20261029-r2/'


def packet():
    folder=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-caller-20261029'
    raw=(folder/'index.json').read_bytes();c.need(c.sha(raw)==INDEX,'original actual-caller packet');members=json.loads(raw)['members']
    def get(name):
        row=members[name];raw=(folder/'objects'/row['sha256']).read_bytes()
        c.need(c.sha(raw)==row['sha256'] and len(raw)==row['bytes'],'exact source member '+name);return raw
    return members,get


def main():
    c.environment_review();members,get=packet();(c.BASE/'input').mkdir()
    portable=(c.ROOT/'tools/chitchat-inplace-074/allocation_tests_portable.rs').read_bytes()
    c.need(c.sha(portable)=='ab5cf5e12f5dfbb956ebcee59f8af554cf3030544eba07bd2e928ad2d0b109df','original portable controls')
    (c.BASE/'observer.rs').write_bytes(portable+b'''\npub(crate) fn stateful_capture<T>(f:impl FnOnce()->T)->(T,serde_json::Value) {
        let (r,m)=measured(f);assert_eq!(m.live,0);(r,json!({"rust":m,"native":null,"native_C_allocations_included":false}))
    }
    #[path="caller.rs"] pub(crate) mod actual_caller;
''')
    (c.BASE/'caller.rs').write_bytes(get('source/tools/chitchat-caller-074/caller.rs'))
    (c.BASE/'cases-r2.json').write_bytes(get('source/tools/chitchat-caller-074/cases-r2.json'))
    c.need(c.sha((c.BASE/'cases-r2.json').read_bytes())==c.TABLE_SHA,'same full workload')
    derived={}
    for variant in ('B','P','Q'):
        prefix=PREFIX+'fixtures/'+variant+'/';folder=c.BASE/'fixtures'/variant
        for name in members:
            if name.startswith(prefix):
                dest=folder/name[len(prefix):];dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(get(name))
        patches={
            'src/delta.rs':'\n#[cfg(not(test))]\n#[path="../../../probe.rs"]\npub mod cost_probe;\n',
            'src/lib.rs':'\n#[cfg(not(test))]\n#[doc(hidden)]\npub use delta::cost_probe::main as cost_probe_main;\n',
            'Cargo.toml':'\n[[bin]]\nname="hc074-clean-cost"\npath="cost-main.rs"\n',
        }
        for name,suffix in patches.items():
            path=folder/name;text=path.read_text(encoding='utf8')
            if name=='Cargo.toml':
                c.need(text.count('[dev-dependencies.serde_json]')==1,'one already locked diagnostic JSON dependency')
                text=text.replace('[dev-dependencies.serde_json]','[dependencies.serde_json]')
            path.write_text(text+suffix,encoding='utf8',newline='\n')
        (folder/'cost-main.rs').write_text('fn main(){chitchat::cost_probe_main();}\n',encoding='utf8',newline='\n')
        derived[variant]=dict(append=patches,serde_json_dependency_kind_only=True)
        (c.BASE/('probe-'+variant+'.rs')).write_bytes((c.TOOLS/'probe.rs').read_bytes()+b'\nconst VARIANT:&str="'+variant.encode()+b'";\n')
        # Unique private normal module, independent of all cfg(test) overlays.
        path=folder/'src/delta.rs';path.write_bytes(path.read_bytes().replace(b'../../../probe.rs',('../../../probe-'+variant+'.rs').encode()))
    c.write(c.BASE,'derivation.json',dict(predecessor=INDEX,variants=derived,root_adopted=False,protocol=c.protocol()))
    for name in members:
        if name.startswith(PREFIX+'input/') and '/catalog-' not in name:
            dest=c.BASE/name[len(PREFIX):];dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(get(name))
    observations=json.loads(get(PREFIX+'cohort-3/native/observations.json'))
    c.write(c.BASE/'input','expected-caller.json',[dict(input=r['input'],decisions=r['records'][0]['decisions'],wire=r['records'][0]['wire']) for r in observations['rows']])
    (c.BASE/'cargo-home').mkdir();(c.BASE/'cargo-home/config.toml').write_text(c.windows_configuration(),encoding='utf8',newline='\n')
    c.windows_registry_guard();names=set(c.source());frame=json.loads(get(PREFIX+'frame.json'))
    for name,digest in frame.items():
        if name.startswith('tools/') and name.endswith('.rs'):
            c.need(c.sha((c.ROOT/name).read_bytes())==digest,'same old Rust overlay');names.add(name)
    for tree in ('fixtures','input','cargo-home'):
        names|={p.relative_to(c.ROOT).as_posix() for p in (c.BASE/tree).rglob('*') if p.is_file()}
    for name in ('observer.rs','caller.rs','cases-r2.json','probe-B.rs','probe-P.rs','probe-Q.rs'):
        names.add((c.BASE/name).relative_to(c.ROOT).as_posix())
    c.write(c.BASE,'input-names.json',sorted(names))
