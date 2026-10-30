"""Actual all-feature resolve reachability, not a publishing or consumer verdict."""
import json
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'supervisor-zstd-security-074'))
import common as c

def main():
    folder=c.BASE/'metadata-r2-194'
    c.need(json.loads((folder/'result.json').read_bytes())['complete'],'complete locked metadata')
    raw=(folder/'stdout.log').read_bytes();metadata=json.loads(raw)
    intent=json.loads((folder/'intent.json').read_bytes())
    c.need(intent['source']['Cargo.lock']==c.sha((c.ROOT/'Cargo.lock').read_bytes()),'same exact Root lock')
    packages={p['id']:p for p in metadata['packages']};nodes={r['id']:r for r in metadata['resolve']['nodes']}
    roots=[p for p in metadata['workspace_members'] if packages[p]['publish']!=[]]
    reached=set();pending=list(roots)
    while pending:
        node=pending.pop()
        if node in reached:continue
        reached.add(node);pending.extend(d['pkg'] for d in nodes[node]['deps'])
    forbidden=[p for p in reached if packages[p]['name']=='hydracache-long-run-supervisor-074' or (packages[p]['name'],packages[p]['version']) in (('zstd','0.14.3'),('zstd-safe','8.2.0'))]
    c.need(len(packages)==685 and len(roots)==23 and not forbidden,'the changed private edge is unreachable from all23 publishable roots')
    out=c.BASE/'publication';out.mkdir(exist_ok=False)
    c.write(out,'result.json',dict(complete=True,metadata_sha256=c.sha(raw),root_lock_sha256=c.sha((c.ROOT/'Cargo.lock').read_bytes()),
      workspace_members=len(metadata['workspace_members']),publishable_roots=sorted(packages[p]['name'] for p in roots),
      reachable_nodes=len(reached),new_private_nodes_reached=False,registry_consumer_test=False,publication_admitted=False))
    print(json.dumps(dict(publishable_roots=len(roots),reachable_nodes=len(reached),new_private_nodes_reached=False)))

if __name__=='__main__':main()
