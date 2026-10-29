"""Read-only existing WSL image/vendor export, never execute a diagnostic image."""
from pathlib import Path
import hashlib
import json
import os

ROOT=Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
DEST=ROOT/'target/chitchat-host-cost-20261029/prepared/input'
NATIVE=Path('/home/artur/.cache/hydracache-074-chitchat-c-native-target/20261029')


def sha(raw):return hashlib.sha256(raw).hexdigest()


def copy(path,dest,digest):
    assert path.is_file() and not path.is_symlink();body=path.read_bytes();assert sha(body)==digest
    dest.parent.mkdir(parents=True,exist_ok=True)
    with dest.open('xb') as out:out.write(body)
    assert sha(path.read_bytes())==digest==sha(dest.read_bytes())


def main():
    assert os.getuid()==os.getgid()==1000 and NATIVE.resolve()==NATIVE
    spec=json.loads((DEST.parent/'native-export.json').read_bytes())
    for variant,row in spec['images'].items():
        path=NATIVE/'clean-cost-targets'/variant/'x86_64-unknown-linux-gnu/release/hc074-clean-cost'
        assert row['path']==str(path);copy(path,DEST/'images'/variant,row['sha256'])
    for name,files in spec['vendor'].items():
        folder=NATIVE/'vendor'/name
        expected=dict(files);expected['.cargo-checksum.json']=sha((json.dumps(dict(package=spec['checksums'][name],files=files),indent=2,sort_keys=True)+'\n').encode())
        actual={p.relative_to(folder).as_posix():sha(p.read_bytes()) for p in folder.rglob('*') if p.is_file()}
        assert actual==expected
        for leaf,digest in expected.items():copy(folder/leaf,DEST/'vendor'/name/leaf,digest)
    print(json.dumps(dict(images=3,vendor_crates=len(spec['vendor']),read_only_source=True,diagnostics_executed=False)))


if __name__=='__main__':main()
