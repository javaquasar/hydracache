"""Add only two checksum-reviewed versions to the existing non-product build cache."""
import json
import os
import time
from pathlib import Path
import common as c

def capture():
    folder=c.BASE/'input';folder.mkdir(exist_ok=False);(folder/'archives').mkdir()
    cache=Path.home()/'.cargo/registry/cache/index.crates.io-1949cf8c6b5b557f'
    codec=c.load(c.ROOT/'tools/dbus-source-package-074/package.py','direct_zstd_codec')
    files={}
    for name,digest in c.ARCHIVES.items():
        raw=(cache/(name+'.crate')).read_bytes();c.need(c.sha(raw)==digest,'same audited registry checksum')
        (folder/'archives'/(name+'.crate')).write_bytes(raw)
        files[name]={n:c.sha(b) for n,b in codec.unpack(raw,name).items()}
    c.write(folder,'added-vendor.json',files)

def seed():
    c.need(os.name=='posix' and os.getuid()==os.getgid()==1000,'nonroot local native build cache only')
    deadline=time.monotonic()+1100
    out=c.BASE/'native-supply';out.mkdir(exist_ok=False)
    original=(c.NATIVE/'vendor-receipt.json').read_bytes();config=(c.NATIVE/'cargo-home/config.toml').read_bytes()
    codec=c.load(c.ROOT/'tools/dbus-source-package-074/package.py','direct_zstd_codec')
    old=json.loads(original)
    # Existing 644 packages and old receipt/config remain byte-identical.
    for name,members in old.items():
        c.need(time.monotonic()<deadline,'original supply deadline');c.capacity()
        root=c.NATIVE/'vendor'/name
        actual={p.relative_to(root).as_posix():c.sha(p.read_bytes()) for p in root.rglob('*') if p.is_file()}
        actual.pop('.cargo-checksum.json');c.need(actual==members,'fresh original source preflight')
    for name,digest in c.ARCHIVES.items():
        raw=(c.BASE/'input/archives'/(name+'.crate')).read_bytes();c.need(c.sha(raw)==digest,'reviewed input archive')
        root=c.NATIVE/'vendor'/name;root.mkdir(exist_ok=False)
        members=codec.unpack(raw,name)
        for relative,body in members.items():
            p=root/relative;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(body)
        c.write(root,'.cargo-checksum.json',dict(package=digest,files={n:c.sha(b) for n,b in members.items()}))
    value=c.registry(deadline)
    c.need((c.NATIVE/'vendor-receipt.json').read_bytes()==original and (c.NATIVE/'cargo-home/config.toml').read_bytes()==config,'old config and receipt immutable')
    c.write(out,'result.json',dict(complete=True,**value))

if __name__=='__main__':
    import sys
    {'capture':capture,'seed':seed}[sys.argv[1]]()
