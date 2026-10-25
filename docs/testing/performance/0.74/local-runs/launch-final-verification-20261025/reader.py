"""Additional data-only reader; avoids shell quoting and preserves exact pins."""
import hashlib,json,subprocess
from pathlib import Path
root=Path(__file__).resolve().parent.parent
prefix='docs/testing/performance/0.74/'
def read(name):return subprocess.check_output(['git','show',':'+prefix+name],cwd=root)
counts=[]
for folder,digest in [
    ('local-runs/launch-build-20261025/','572c66863cbc5702ef24ec24c039b96e27c2d15a1ec40b26381f8813c4ec08d9'),
    ('local-runs/launch-coordinator-gates-20261025/','5adf30a4ab86e3d768c39e6a7267a9c5af82ffb0da519bdbc5c3e17c6ba72d19')]:
    raw=read(folder+'index.json');assert hashlib.sha256(raw).hexdigest()==digest
    entries=json.loads(raw)['members']
    for value in entries.values():
        key=value if isinstance(value,str) else value['sha256']
        content=read(folder+'objects/'+key)
        assert hashlib.sha256(content).hexdigest()==key
        assert isinstance(value,str) or len(content)==value['bytes']
    counts.append(len(entries))
assert hashlib.sha256(read('contract-observer-launch-receipt-20261025.json')).hexdigest()=='0f364991c37248f7f7d73d271932b3aaa77250d308df4a6314b0eb09905d65b5'
print(json.dumps(dict(staged_build_and_gates_verified=True,members=counts,receipt_bytes_verified=True,admission=False)))
