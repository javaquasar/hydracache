"""Read only sealed bytes/JSON/ZIP; never import or execute archived producers."""
from pathlib import Path
import io
import json
import re
import types
import zipfile
import common as c
import analyze as a

EXPECTED='853f8691d7e9e1f0d2c1143908fddaefdb36e0fd24a507cbf7ac4424a3c61a71'
PACKET=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-commit-20261029'
Q='capture/chitchat-commit-Q-20261029-r2/'
P='capture/chitchat-commit-P-20261029-r2/'


def main():
    raw=(PACKET/'index.json').read_bytes();c.need(c.sha(raw)==EXPECTED,'exact sealed index')
    index=json.loads(raw);members=index['members']
    c.need(index['schema']=='hc074-chitchat-commit-v1' and len(members)==1143,'complete schema/member accounting')
    seen=set()
    for name,row in members.items():
        c.need(not name.startswith('/') and '..' not in name.split('/') and re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'regular contained object reference')
        path=PACKET/'objects'/row['sha256'];c.need(path.is_file() and not path.is_symlink(),'regular object')
        body=path.read_bytes();c.need(len(body)==row['bytes'] and c.sha(body)==row['sha256'],'object bytes '+name);seen.add(row['sha256'])
    c.need({p.name for p in (PACKET/'objects').iterdir()}==seen and len(seen)==341,'complete unique objects')
    def get(name):return (PACKET/'objects'/members[name]['sha256']).read_bytes()
    cases=json.loads(get(Q+'input/cases.json'))
    project=lambda rows:[tuple(r[k] for k in ('mtu','value_len','accepted','wire')) for r in rows]
    def semantics(rows):c.need(len(rows)==239 and project(rows)==project(cases),'sealed original semantic body')
    for prefix in (P,Q):
        frame_raw=get(prefix+'frame.json');frame=json.loads(frame_raw);frozen=json.loads(get(prefix+'format/result.json'))
        capsule=get(prefix+'source.zip')
        c.need(c.sha(frame_raw)==frozen['frame_sha256'] and c.sha(capsule)==frozen['zip_sha256'],'exact source capsule/frame')
        rel=prefix.removeprefix('capture/').removesuffix('/')
        with zipfile.ZipFile(io.BytesIO(capsule)) as z:
            c.need(len(z.namelist())==len(set(z.namelist())) and set(z.namelist())==set(frame)|{'target/'+rel+'/frame.json'},'complete unique ZIP input inventory')
            for name,digest in frame.items():c.need(c.sha(z.read(name))==digest,'actual archived source bytes')
    baseline=json.loads(get(P+'regression-P/native/regressions.json'))
    c.need((baseline['passed'],baseline['failed'],len(baseline['expected_refusals']))==(4,3,3),'first baseline retained')
    values=[]
    for i in range(1,5):
        raw=get(Q+f'cohort-{i}/native/observation/stdout.log').decode()
        value=c.diagnostic(raw,'HC074_NATIVE_ALLOC:')
        c.need(value==json.loads(get(Q+f'cohort-{i}/native/observations.json')),'raw observation body')
        values.append(value)
    # The parser is current reviewed source; only its semantic input view changes.
    original=a.old.c
    try:
        a.old.c=types.SimpleNamespace(need=c.need,semantics=semantics)
        result=a.reconcile(values)
    finally:a.old.c=original
    c.need(result==json.loads(get(Q+'analysis/result.json')) and result['allocation_dominance_pass'],'sealed complete parser reconciliation')
    print(json.dumps(dict(complete=True,index_sha256=EXPECTED,members=len(members),objects=len(seen),archived_code_executed=False,root_adopted=False,release_admitted=False)))


if __name__=='__main__':main()
