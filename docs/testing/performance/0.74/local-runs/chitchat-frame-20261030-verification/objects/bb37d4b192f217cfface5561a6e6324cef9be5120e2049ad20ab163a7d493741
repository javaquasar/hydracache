"""Close G from the first complete resource corpus; no executable invocation."""
import json,shutil
from pathlib import Path
import checks as a

def main():
    values=a.load_native();result=json.loads((a.BASE/'analysis/native.json').read_bytes())
    a.need(a.review(values)==result and not result['allocation_retention_gate_pass'],'first complete negative result')
    a.need(result['regressions']==[
      dict(after=9,before=6,field='allocs',kind='stateful',peer='B',row=6,scope='logical'),
      dict(after=9,before=6,field='allocs',kind='stateful',peer='B',row=6,scope='physical')],'exact observed refusal')
    out=a.BASE/'attribution/accepted-multiblock';out.mkdir()
    rows={}
    for i,v in ((0,'B'),(1,'P'),(2,'G')):
        row=values[i]['stateful']['rows'][6];r=row['records'][0]
        rows[v]=dict(name=row['name'],threshold=row['threshold'],steps=row['steps'],
          decisions=r['decisions'],compressions=r['compressions'],states=r['states'],
          finish_capacity=r['finish_capacity'],meter=r['meter'],
          wire_bytes=len(bytes.fromhex(r['wire'])),wire_sha256=a.sha(bytes.fromhex(r['wire'])),
          payload_sha256=[a.sha(bytes.fromhex(v)) for v in row['inputs']])
    registry=Path.home()/'.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/zstd-0.14.3'
    source={}
    for relative in ('src/lib.rs','src/bulk/compressor.rs'):
        raw=(registry/relative).read_bytes();path=out/'registry'/relative;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
        source[relative]=dict(sha256=a.sha(raw),bytes=len(raw))
    review=dict(complete=True,decision='declined-first-full-resource-gate',rows=rows,regressions=result['regressions'],
      confirmed='G has one extra compression call and three extra logical/physical Rust allocations; capacities, decisions and wire agree',
      source_inference='The 102-byte simulated tail is not cached as compressed; its additional compression error maps to an owned String/io::Error. This is consistent with the allocation increase, not allocator-stack attribution.',
      registry_source=source,new_measurements=False,CPU_qualified=False,RSS_qualified=False,root_adopted=False,release_admitted=False)
    (out/'analysis.json').write_text(json.dumps(review,indent=2,sort_keys=True)+'\n',encoding='utf8')
    snapshot=a.BASE/'contexts/executed-producers';snapshot.mkdir()
    for name in ('chitchat-frame-074','chitchat-frame-result-074','chitchat-frame-native-r2-074','chitchat-frame-native-r3-074','chitchat-frame-native-r4-074','chitchat-frame-evidence-074'):
        shutil.copytree(a.ROOT/'tools'/name,snapshot/name,ignore=shutil.ignore_patterns('__pycache__'))
    path=a.ROOT/'tools/chitchat-frame-074/recipe.py';raw=path.read_bytes()
    a.need(raw.count(b'EXECUTION_ALLOWED=True')==1,'one current G execution gate')
    path.write_bytes(raw.replace(b'EXECUTION_ALLOWED=True',b'EXECUTION_ALLOWED=False'))
    (a.BASE/'decision.json').write_text(json.dumps(dict(decision=review['decision'],further_measurements_allowed=False,
       CPU_qualified=False,RSS_qualified=False,root_adopted=False,release_admitted=False),indent=2)+'\n',encoding='utf8')
    print(json.dumps(dict(decision=review['decision'],regressions=2,root_adopted=False)))

if __name__=='__main__':main()
