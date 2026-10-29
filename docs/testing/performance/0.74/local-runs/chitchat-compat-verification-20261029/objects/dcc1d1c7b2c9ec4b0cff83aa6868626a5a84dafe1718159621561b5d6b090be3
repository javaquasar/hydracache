"""Byte-only replay of sealed receipts; never run archived producers/images."""
import json
import re
import analyze as a
import common as c

EXPECTED='865f98e6b82240f06b71cf7b316b441766a6653947fa6d8899fa8540f2fb9bfc'
PACKET=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-compat-20261029'


def main():
    raw=(PACKET/'index.json').read_bytes();c.need(c.sha(raw)==EXPECTED,'exact sealed index')
    index=json.loads(raw);members=index['members'];seen=set()
    c.need(index['schema']=='hc074-chitchat-compat-v1' and len(members)==759,'complete packet inventory')
    for name,row in members.items():
        c.need(not name.startswith('/') and '..' not in name.split('/') and re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'contained object reference')
        path=PACKET/'objects'/row['sha256'];c.need(path.is_file() and not path.is_symlink(),'regular sealed object')
        body=path.read_bytes();c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'sealed member '+name);seen.add(row['sha256'])
        c.need('/windows-targets/' not in name and ('/cargo-home/' not in name or name.endswith('/cargo-home/config.toml')),'compiled/mutable cache exclusion')
    c.need(len(seen)==377 and seen=={p.name for p in (PACKET/'objects').iterdir()},'complete unique object inventory')
    def get(name):return (PACKET/'objects'/members[name]['sha256']).read_bytes()
    def capture(name):return get('capture/'+name)
    result=a.reconcile(capture)
    c.need(result==json.loads(capture(a.R1+'/analysis/result.json')) and result['compatibility_gate_pass'],'independent raw complete-peer replay')
    for flag in ('CPU_qualified','RSS_qualified','native_C_qualified','root_adopted','release_admitted','archived_code_executed','old_inner_results_promoted'):
        c.need(index[flag] is False,'no qualification promotion')
    prefix=a.R0+'/diagnosis/recovered-gnu-188/';recovered=json.loads(capture(prefix+'recovery.json'))
    c.need(recovered['complete'] and recovered['inner_result_recovered'] and recovered['outer_timeout_unchanged'] and recovered['overall_original_gate_pass'] is False and recovered['rerun_dispatched'] is False,'separate unpromoted first inner recovery')
    c.need(len(recovered['members'])==30,'complete recovered raw inventory')
    for name,row in recovered['members'].items():
        body=capture(prefix+'raw/'+name);c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'exact recovered first raw member')
    c.need(not any(n.startswith('capture/'+a.R1+'/'+p+'/') for n in members for p in ('gnu-194','windows-194')),'only failed peers recovered')
    portable=get('source/tools/chitchat-inplace-074/allocation_tests_portable.rs')
    c.need(c.sha(portable)=='ab5cf5e12f5dfbb956ebcee59f8af554cf3030544eba07bd2e928ad2d0b109df','original portable observer prefix')
    c.need(capture(a.R1+'/observer.rs').startswith(portable),'no old observer body changes')
    c.need(capture(a.R1+'/caller.rs')==get('source/tools/chitchat-caller-074/caller.rs')+b'\n'+get('source/tools/chitchat-compat-074/semantic_export.rs'),'exact append-only semantic export')
    previous=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-caller-20261029'
    raw=(previous/'index.json').read_bytes();c.need(c.sha(raw)==index['predecessor_index_sha256']=='874ba6355b98229cbeab11c5b0742e439cbced847e6954478234e10e92d7ef56','original actual-caller predecessor')
    prefix='capture/chitchat-caller-20261029-r2/fixtures/Q/'
    before={n[len(prefix):]:r['sha256'] for n,r in json.loads(raw)['members'].items() if n.startswith(prefix)}
    prefix='capture/'+a.R1+'/fixtures/Q/'
    after={n[len(prefix):]:r['sha256'] for n,r in members.items() if n.startswith(prefix)}
    c.need(before==after and bool(after),'same complete runtime/test fixture as previous candidate')
    for mode in ('controls-r1','format','performance-contract'):
        prefix=a.R1+'/final/'+mode+'/'
        c.need(json.loads(capture(prefix+'result.json'))['complete'],'complete source/fault/format/contract checks')
        a.owner(json.loads(capture(prefix+'execution.json')))
    first=capture(a.R1+'/final/controls/stderr.log').decode()
    c.need("AttributeError: module 'prepare' has no attribute 'packet'" in first,'first offline harness import failure retained')
    c.need(not json.loads(capture(a.R1+'/final/controls/result.json'))['complete'],'old offline refusal not overwritten')
    print(json.dumps(dict(complete=True,members=len(members),objects=len(seen),index_sha256=EXPECTED,
        compatibility_gate_pass=True,test_executions=340,archived_code_executed=False,root_adopted=False,release_admitted=False)))


if __name__=='__main__':main()
