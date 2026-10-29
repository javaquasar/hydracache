"""Restore the accepted source/table and change only test observer plumbing."""
import json
import os
import re
import common as c

INDEX='874ba6355b98229cbeab11c5b0742e439cbced847e6954478234e10e92d7ef56'
PREFIX='capture/chitchat-caller-20261029-r2/'


def packet():
    folder=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-caller-20261029'
    raw=(folder/'index.json').read_bytes();c.need(c.sha(raw)==INDEX,'immutable actual-caller predecessor')
    members=json.loads(raw)['members']
    def get(name):
        row=members[name];body=(folder/'objects'/row['sha256']).read_bytes()
        c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'exact source member '+name);return body
    return members,get


def main():
    c.environment_review()
    original=c.ROOT/'target/chitchat-compat-20261029'
    events=json.loads((original/'diagnosis/standby-events.json').read_bytes())
    c.need(events['diagnosis_complete'] and all(events[k] is False for k in ('settings_changed','clocks_changed','services_changed','old_outcomes_promoted')),'read-only recovery evidence')
    c.need(any(e['id']==506 and 'Lid' in e['message'] for e in events['events']) and any(e['id']==507 and 'Lid' in e['message'] for e in events['events']),'confirmed host standby/recovery')
    old_outcomes={}
    for label in ('gnu-188','windows-188'):
        raw=(original/label/'execution.json').read_bytes();owner=json.loads(raw)
        c.need(owner['classification']=='timeout' and owner['bound_seconds']==1200 and owner['retired'] and not owner['errors'],'first outer refusals remain exact')
        old_outcomes[label]=c.sha(raw)
    c.write(c.BASE,'recovery-context.json',dict(environment_change='confirmed Modern Standby exit',
        original_outcome_sha256=old_outcomes,old_inner_results_promoted=False,bounds_extended=False,only_failed_peers_dispatched=True))
    members,get=packet();(c.BASE/'input').mkdir()
    folder=c.BASE/'fixtures/Q';prefix=PREFIX+'fixtures/Q/'
    for name in members:
        if name.startswith(prefix):
            dest=folder/name[len(prefix):];dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(get(name))
    portable=(c.ROOT/'tools/chitchat-inplace-074/allocation_tests_portable.rs').read_bytes()
    c.need(c.sha(portable)=='ab5cf5e12f5dfbb956ebcee59f8af554cf3030544eba07bd2e928ad2d0b109df','qualified original portable observer')
    # No old observer control body changes; C absence is explicit, not zero data.
    observer=portable.decode()+'''
pub(crate) fn stateful_capture<T>(operation: impl FnOnce() -> T) -> (T, serde_json::Value) {
    let (result, rust) = measured(operation);
    assert_eq!(rust.live, 0);
    (result, json!({"rust":rust,"native":null,"native_C_allocations_included":false}))
}
#[path="caller.rs"]
pub(crate) mod actual_caller;
'''
    (c.BASE/'observer.rs').write_text(observer,encoding='utf8',newline='\n')
    original=get('source/tools/chitchat-caller-074/caller.rs')
    c.need(original==(c.ROOT/'tools/chitchat-caller-074/caller.rs').read_bytes(),'unchanged original caller module')
    (c.BASE/'caller.rs').write_bytes(original+b'\n'+(c.TOOLS/'semantic_export.rs').read_bytes())
    table=get('source/tools/chitchat-caller-074/cases-r2.json');c.need(c.sha(table)==c.TABLE_SHA,'exact accepted request table')
    (c.BASE/'cases-r2.json').write_bytes(table)
    path=folder/'src/delta.rs';text=path.read_text(encoding='utf8')
    relative=os.path.relpath(c.BASE/'observer.rs',path.parent).replace('\\','/')
    text,n=re.subn(r'(?<=#\[path = ")[^"\n]+(?="\]\npub\(crate\) mod cost_review;)',relative,text)
    c.need(n==1,'only the test observer path changes');path.write_text(text,encoding='utf8',newline='\n')
    original_catalog=json.loads(get(PREFIX+'build-Q/native/catalog.json'))
    removed=['delta::cost_review::'+v for v in ('native_c_allocator_control','native_rust_tag_control','native_operation_observations')]
    c.need(set(removed)<=set(original_catalog),'explicit diagnostic C extension catalog')
    catalog=sorted([n for n in original_catalog if n not in removed]+['delta::cost_review::actual_caller::compatibility_semantic_export'])
    c.write(c.BASE/'input','catalog-gnu.json',catalog)
    c.write(c.BASE/'input','catalog-windows.json',sorted(catalog+['delta::cost_review::cpu_observations']))
    observations=json.loads(get(PREFIX+'cohort-3/native/observations.json'))
    expected=[dict(input=r['input'],decisions=r['records'][0]['decisions'],wire=r['records'][0]['wire']) for r in observations['rows']]
    c.write(c.BASE/'input','expected-caller.json',expected)
    for name in members:
        if name.startswith(PREFIX+'input/') and '/catalog-' not in name:
            dest=c.BASE/name[len(PREFIX):];dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(get(name))
    (c.BASE/'cargo-home').mkdir();(c.BASE/'cargo-home/config.toml').write_text(c.windows_configuration(),encoding='utf8',newline='\n')
    c.windows_registry_guard()
    inherited=json.loads(get(PREFIX+'frame.json'));names=set(c.source())
    for name,digest in inherited.items():
        if name.startswith('tools/') and name.endswith('.rs'):
            c.need(c.sha((c.ROOT/name).read_bytes())==digest,'unchanged original Rust overlay '+name);names.add(name)
    for tree in (c.BASE/'fixtures',c.BASE/'input',c.BASE/'cargo-home'):
        names|={p.relative_to(c.ROOT).as_posix() for p in tree.rglob('*') if p.is_file()}
    names|={(c.BASE/name).relative_to(c.ROOT).as_posix() for name in ('observer.rs','caller.rs','cases-r2.json')}
    c.write(c.BASE,'input-names.json',sorted(names))
    c.write(c.BASE,'derivation.json',dict(predecessor=INDEX,table_sha256=c.TABLE_SHA,runtime_methods_changed=False,
        old_control_bodies_changed=False,native_C_available=False,resource_cohort_dispatched=False,root_adopted=False))
