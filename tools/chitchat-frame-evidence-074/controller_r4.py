"""Controller namespace only after R3 WSL denied access before Linux entry."""
import json
from pathlib import Path
import shutil

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'target/chitchat-frame-20261030'
old=ROOT/'tools/chitchat-frame-native-r3-074'
new=ROOT/'tools/chitchat-frame-native-r4-074'

def main():
    failed=BASE/'native-observe-r3'
    result=json.loads((failed/'result.json').read_bytes())
    own=json.loads((failed/'execution.json').read_bytes())
    assert not result['complete'] and own['retired'] and not own['errors']
    assert 'Wsl/E_ACCESSDENIED' in (failed/'stdout.log').read_text(encoding='utf-16-le')
    assert not any(failed.glob('observe-*')) and not (failed/'session-result.json').exists()
    shutil.copytree(old,new)
    path=new/'native.py';raw=path.read_text()
    raw=raw.replace("INPUT=p.BASE/'native-input-r3'","INPUT=p.BASE/'native-input-r4'")
    raw=raw.replace("FRAME=p.BASE/'native-frame-r3.json'","FRAME=p.BASE/'native-frame-r4.json'")
    raw=raw.replace("old=p.BASE/'native-input-r2';frame=p.BASE/'native-frame-r2.json'","old=p.BASE/'native-input-r3';frame=p.BASE/'native-frame-r3.json'")
    raw=raw.replace("controller test-name parser only; original native images/sources/object reused","WSL sandbox access refusal before Linux entry; controller namespace only, identical images/fixtures/object")
    raw=raw.replace('chitchat-frame-native-r3-074/native.py','chitchat-frame-native-r4-074/native.py')
    raw=raw.replace("OUT=p.BASE/('native-'+PHASE+'-r3')","OUT=p.BASE/('native-'+PHASE+'-r4')")
    raw=raw.replace("in ('build','semantic194','semantic188','observe')","in ('observe',)")
    raw=raw.replace("in ('native-semantic194-r3','native-semantic188-r3','native-observe-r3')","in ('native-observe-r4',)")
    path.write_text(raw,encoding='utf8',newline='\n')
    (new/'README.md').write_text((old/'README.md').read_text()+
        '\nR4 is a controller-only namespace after R3 received Wsl/E_ACCESSDENIED before Linux entry. '
        'It reuses the exact R2 images and R3 semantic gates; no source, sample, predicate or bound changes. '
        'R3 remains refused. Run R4 once with WSL sandbox access.\n',encoding='utf8',newline='\n')
    folder=BASE/'attribution/access-refusal-r3';folder.mkdir()
    (folder/'analysis.json').write_text(json.dumps(dict(first_result=result,owner=own,
        linux_session_started=False,resource_observations=0,repair='sandbox access and separate controller/output namespace',
        images_rebuilt=False,checks_changed=False,bounds_changed=False,root_adopted=False),indent=2)+'\n',encoding='utf8')

if __name__=='__main__':main()
