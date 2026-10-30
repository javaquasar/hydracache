"""G source from a pinned declined parent; never executes archived Python."""
import json
from pathlib import Path
import shutil
import sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'tools/chitchat-security-only-074'));import scope as c
BASE=ROOT/'target/chitchat-frame-20261030'
PARENT=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-fused-20261030'
PARENT_SHA='e554bd8ee5bd7b4f0cc2963393ccbea922e93663f294962446d1caa651034403'
EXECUTION_ALLOWED=False
SUFFIX='\n#[cfg(test)]\n#[path="frame_tests.rs"]\nmod frame_review;\n#[cfg(test)]\nconst FRAME_VARIANT:&str="{v}";\n'
BASELINE_HELPER='''
#[cfg(test)]
impl CompressedStreamWriter {
    fn commit_simulated_prefix(&mut self,encoded:&[u8]) { self.output.extend_from_slice(encoded); }
}
'''

def parent():
    raw=(PARENT/'index.json').read_bytes();c.need(c.sha(raw)==PARENT_SHA,'immutable negative F index')
    index=json.loads(raw);sys.path.insert(0,str(ROOT/'tools/chitchat-fused-result-074'))
    import archive
    archive.validate(index,lambda h:(PARENT/'objects'/h).read_bytes())
    def read(name):
        row=index['members'][name];body=(PARENT/'objects'/row['sha256']).read_bytes()
        c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'whole immutable parent member');return body
    return index,read

def baseline():
    BASE.mkdir(exist_ok=False);out=BASE/'preparation';out.mkdir()
    index,read=parent();negative=json.loads(read('capture/retention/analysis.json'))
    c.need(negative['complete'] and not negative['resource_gate_pass'] and negative['delta_bytes']==12,'actual negative before G, no F rerun')
    trees={}
    for v in ('B','F'):
        folder=BASE/'fixtures'/v;folder.mkdir(parents=True)
        prefix='capture/fixtures/'+v+'/'
        for name in index['members']:
            if not name.startswith(prefix):continue
            relative=Path(name[len(prefix):]);c.need(not relative.is_absolute() and '..' not in relative.parts,'contained source')
            path=folder/relative;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(read(name))
        path=folder/'src/serialize.rs';raw=path.read_bytes()
        if v=='B':
            # The common test padding is not a new production field.
            for old,new in [(b'    tail_cache:Option<usize>,',b'    #[cfg(test)]\n    tail_cache:Option<usize>,'),
              (b'            tail_cache:None,',b'            #[cfg(test)]\n            tail_cache:None,')]:
                c.need(raw.count(old)==1,'one diagnostic B padding');raw=raw.replace(old,new)
        raw+=BASELINE_HELPER.encode()+SUFFIX.format(v=v).encode()
        path.write_bytes(raw);(folder/'src/frame_tests.rs').write_bytes((HERE/'tests.rs').read_bytes())
        trees[v]=c.tree(folder)
    c.write(out,'baseline.json',dict(parent_sha256=PARENT_SHA,first_negative=negative,fixtures=trees,
      tests_before_G=True,unchanged_F_not_rerun=True,new_hypothesis='original frame commit operations',root_adopted=False))

def candidate():
    c.need(not (BASE/'STOP.json').exists(),'first refusal stops G')
    held=json.loads((BASE/'preparation/baseline.json').read_bytes())
    c.need(all(c.tree(BASE/'fixtures'/v)==files for v,files in held['fixtures'].items()),'immutable baseline/control sources')
    source=BASE/'fixtures/F';folder=BASE/'fixtures/G';shutil.copytree(source,folder)
    path=folder/'src/serialize.rs';before=path.read_bytes()
    old=b'        w.output.extend_from_slice(&encoded[..encoded_len]);'
    c.need(before.count(old)==before.count(BASELINE_HELPER.encode())==1,'exact single product commit site and test adapter')
    after=before.replace(old,b'        w.commit_simulated_prefix(&encoded[..encoded_len]);')
    after=after.replace(BASELINE_HELPER.encode(),b'\n'+(HERE/'commit.rs').read_bytes())
    after=after.replace(SUFFIX.format(v='F').encode(),SUFFIX.format(v='G').encode())
    path.write_bytes(after)
    c.write(BASE/'preparation','candidate.json',dict(before_sha256=c.sha(before),after_sha256=c.sha(after),
      fixtures=c.tree(folder),changed_product_site='accepted encoded-prefix commit only',new_vec=False,
      capacity_reservation_added=False,compression_calls_changed=False,public_traits_changed=False,root_adopted=False))

if __name__=='__main__':
    c.need(EXECUTION_ALLOWED,'G execution sealed')
    {'baseline':baseline,'candidate':candidate}[sys.argv[1]]()
