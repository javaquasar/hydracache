"""Independent offline accounting of the first four native security peers."""
import json
import re

import common as c

PEERS=('windows-194','windows-188','gnu-194','gnu-188')
FORBIDDEN={'bindgen','cmake','with-rust-allocator','zstdmt'}


def offsets(raw):
    rows=[tuple(map(int,v)) for v in re.findall(r'HC074-ZSTD-OFFSET offset=(\d+) capacity=(\d+) written=(\d+)',raw)]
    # DATA is a fixed ASCII byte string; derive its declared length, never shrink a corpus.
    source=(c.TOOLS/'probe.rs').read_text(encoding='utf8')
    match=re.search(r'const DATA: &\[u8\] = b"([^"]+)";',source)
    c.need(match is not None,'exact owned payload declaration')
    end=len(match[1].encode())
    expected=[(offset,capacity,0 if offset>=end else min(capacity,end-offset))
              for offset in (0,1,end-1,end,end+1,end+4096,2**64-10,2**64-1)
              for capacity in (0,1,7,256)]*2
    c.need(rows==expected,'complete ordered two-reader/offset/capacity diagnostics')
    return rows


def compiled_features(raw,kind):
    result={}
    for line in raw.splitlines():
        row=json.loads(line)
        name=row.get('target',{}).get('name')
        if row.get('reason')=='compiler-artifact' and name in ('zstd','zstd_safe','zstd_sys'):
            features=set(row['features'])
            c.need(not features&FORBIDDEN,'unrequested dependency feature activated')
            c.need(name not in result or result[name]==features,'ambiguous selected feature graph')
            result[name]=features
    expected={'zstd_safe','zstd_sys'}|({'zstd'} if kind=='probe' else set())
    c.need(set(result)==expected,'complete compiled dependency profile')
    c.need({'legacy','std','seekable'}<=result['zstd_safe'] and {'legacy','seekable'}<=result['zstd_sys'],
           'vulnerable legacy/seekable branches must actually be enabled')
    return {name:sorted(value) for name,value in result.items()}


def main():
    out=c.BASE/'analysis';out.mkdir(exist_ok=False)
    (out/'producer.py').write_bytes(__import__('pathlib').Path(__file__).read_bytes())
    matrix={};reference=None;feature_reference=None;offset_reference=None
    for peer in PEERS:
        folder=c.BASE/peer
        result=json.loads((folder/'result.json').read_bytes())
        session=json.loads((folder/'session-result.json').read_bytes())
        execution=json.loads((folder/'execution.json').read_bytes())
        c.need(result['complete'] and session['complete'] and result['head_unchanged'] and not result['changed'], 'complete original peer: '+peer)
        c.need(execution['exit']==0 and execution['retired'] and not execution['errors'] and
               execution['bound_seconds']==1200 and execution['cleanup_seconds']==5,'original parent owner')
        cases=json.loads((folder/'cases.json').read_bytes());features={}
        for kind in ('probe','upstream'):
            listed=c.inventory((folder/(kind+'-list')/'stdout.log').read_text(encoding='utf8'),kind)
            raw=(folder/(kind+'-test')/'stdout.log').read_text(encoding='utf8')
            c.need(c.accounting(raw,listed)==cases[kind]['counts'] and listed==cases[kind]['names'],'actual raw full-case reconciliation')
            features[kind]=compiled_features((folder/(kind+'-build')/'stdout.log').read_text(encoding='utf8'),kind)
            for action in ('build','list','test','check','clippy'):
                receipt=json.loads((folder/(kind+'-'+action)/'execution.json').read_bytes())
                c.need(receipt['exit']==0 and receipt['retired'] and not receipt['errors'],'complete child owner')
        rows=offsets((folder/'probe-test/stdout.log').read_text(encoding='utf8'))
        if reference is None:reference,feature_reference,offset_reference=cases,features,rows
        c.need(cases==reference and features==feature_reference and rows==offset_reference,'identical cross-peer selected catalogs/features/offsets')
        matrix[peer]=dict(public_cases=cases['probe']['counts']['passed'],upstream_cases=cases['upstream']['counts']['passed'],
            ignored=0,filtered=0,offset_observations=len(rows),check=True,strict_clippy=True)
    c.source();c.frozen_inputs()
    c.write(out,'result.json',dict(complete=True,matrix=matrix,features=feature_reference,
        catalogs_identical=True,offsets_identical=True,original_refusals_retained=True,
        old_unsafe_apis_executed=False,root_adopted=False,MTU_or_allocation_gate_closed=False,
        performance_claim=False,workspace_pass=False,release_admitted=False))
    print(json.dumps(dict(complete=True,matrix=matrix,features=feature_reference)))


if __name__=='__main__':main()
