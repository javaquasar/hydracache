"""Byte-only sealed replay of raw receipts and source inventories, no execution."""
import hashlib
import io
import json
import re
import zipfile
import analyze as a

EXPECTED='134607dd8e6f8dced2bb1f0f1eb1b594f1ccc70c802a8c7d601bd64b2cf7e670'
PACKET=a.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-host-cost-20261029'


def sha(raw):return hashlib.sha256(raw).hexdigest()


def main():
    raw=(PACKET/'index.json').read_bytes();a.need(sha(raw)==EXPECTED,'exact sealed native index')
    index=json.loads(raw);members=index['members'];seen=set()
    a.need(index['schema']=='hc074-chitchat-host-control-v1' and len(members)==534,'complete logical inventory')
    for name,row in members.items():
        a.need(not name.startswith('/') and '..' not in name.split('/') and re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'contained object reference')
        path=PACKET/'objects'/row['sha256'];a.need(path.is_file() and not path.is_symlink(),'regular stored object')
        raw=path.read_bytes();a.need(sha(raw)==row['sha256'] and len(raw)==row['bytes'],'exact sealed body');seen.add(row['sha256'])
        a.need('/prepared/input/images/' not in name and '/prepared/input/vendor/' not in name and not name.endswith(('/payload.tar.gz','/patch.tar.gz')),'no compiled/mutable caches')
    a.need(len(seen)==367 and seen=={p.name for p in (PACKET/'objects').iterdir()},'complete unique objects')
    def get(name):return (PACKET/'objects'/members[name]['sha256']).read_bytes()
    def capture(name):return get('capture/'+name)
    result=a.reconcile(capture)
    a.need(result==json.loads(capture(a.R1+'/analysis/result.json')) and result['host_control_gate_pass'],'independent complete raw accounting')
    for context in (a.R0,a.R1):
        frozen=json.loads(capture(context+'/frozen.json'))
        for name,digest in frozen['producer'].items():a.need(sha(get('original-producer/'+context+'/'+name))==digest,'exact original committed producer')
    prefix=a.R0+'/prepared/input/';frame_raw=capture(prefix+'frame.json');frame=json.loads(frame_raw);capsule=capture(prefix+'source.zip')
    a.need(sha(frame_raw)=='818dcf2cca31ee308e41e962de491bc24666b0fb5582bd918cebf65ccc30d1b4','original303-file runtime frame')
    with zipfile.ZipFile(io.BytesIO(capsule)) as z:
        a.need(len(z.namelist())==len(set(z.namelist())) and set(z.namelist())==set(frame)|{'target/chitchat-cost-20261029-r1/frame.json'},'complete unique source capsule')
        for name,digest in frame.items():
            body=z.read(name);a.need(sha(body)==digest and body==capture(prefix+'source/'+name),'original source/registry-archive byte joins')
    parent=a.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-cost-20261029'
    raw=(parent/'index.json').read_bytes();a.need(sha(raw)==index['predecessor_index_sha256']=='9589ebd9014f7b2be15aa8cfedec635e06d042a4b81fe92da3894fdea1008462','unchanged WSL parent')
    row=json.loads(raw)['members']['capture/chitchat-cost-20261029-r1/diagnosis/result.json'];raw=(parent/'objects'/row['sha256']).read_bytes()
    a.need(sha(raw)==row['sha256'] and json.loads(raw)['clock_control_pass'] is False,'first WSL clock refusal never promoted')
    for key in ('CPU_qualified','RSS_qualified','cohort_dispatched','exclusive_host','root_adopted','release_admitted','archived_code_executed'):
        a.need(index[key] is False,'no unearned numerical/product qualification')
    print(json.dumps(dict(complete=True,members=len(members),objects=len(seen),index_sha256=EXPECTED,
        host_control_gate_pass=True,controls=3,CPU_qualified=False,RSS_qualified=False,archived_code_executed=False)))


if __name__=='__main__':main()
