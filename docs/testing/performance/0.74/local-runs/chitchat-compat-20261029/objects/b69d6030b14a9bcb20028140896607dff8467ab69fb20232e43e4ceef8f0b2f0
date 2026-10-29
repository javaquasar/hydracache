"""Copy existing inner bytes after host standby, never synthesize a full pass."""
from pathlib import Path
import hashlib
import json
import os

root=Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
source=Path('/home/artur/.cache/hydracache-074-chitchat-c-native-target/20261029/source-root-compat/target/chitchat-compat-20261029/gnu-188')
out=root/'target/chitchat-compat-20261029/diagnosis/recovered-gnu-188'
assert os.getuid()==1000 and source.resolve()==source and source.stat().st_uid==1000
assert out.resolve().is_relative_to(root)
out.mkdir(exist_ok=False)
(out/'producer.py').write_bytes(Path(__file__).read_bytes());members={}
for path in sorted(source.rglob('*')):
    assert not path.is_symlink()
    if not path.is_file():continue
    name=path.relative_to(source).as_posix();body=path.read_bytes();dest=out/'raw'/name
    dest.parent.mkdir(parents=True,exist_ok=True)
    with dest.open('xb') as stream:stream.write(body)
    members[name]=dict(sha256=hashlib.sha256(body).hexdigest(),bytes=len(body))
assert json.loads((out/'raw/session-result.json').read_bytes())['complete']
(out/'recovery.json').write_text(json.dumps(dict(complete=True,members=members,inner_result_recovered=True,
    overall_original_gate_pass=False,outer_timeout_unchanged=True,rerun_dispatched=False),indent=2)+'\n',encoding='utf8')
print(json.dumps(dict(copied_members=len(members),overall_original_gate_pass=False,rerun_dispatched=False)))
