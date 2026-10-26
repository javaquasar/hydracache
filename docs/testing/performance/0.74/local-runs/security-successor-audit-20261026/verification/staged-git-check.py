"""Verify exact archived bytes in the Git index, without a working-tree rewrite."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
PREFIX = 'docs/testing/performance/0.74/local-runs/security-successor-audit-20261026/'
OUT = ROOT/PREFIX/'verification/staged-git-first.json'
names = [n.decode() for n in subprocess.check_output(['git','diff','--cached','--name-only','-z'],cwd=ROOT).split(b'\0')
         if n and n.decode().startswith(PREFIX)]
if not names:
    raise ValueError('no staged packet')
queries = b''.join((':'+name+'\n').encode() for name in names)
raw = subprocess.check_output(['git','cat-file','--batch'],input=queries,cwd=ROOT)
offset = 0
for name in names:
    end = raw.index(b'\n',offset)
    fields = raw[offset:end].split()
    if len(fields) != 3 or fields[1] != b'blob':
        raise ValueError('missing staged blob')
    size = int(fields[2])
    start = end+1
    content = raw[start:start+size]
    if content != (ROOT/name).read_bytes():
        raise ValueError('staged raw bytes differ: '+name)
    offset = start+size+1
if offset != len(raw):
    raise ValueError('extra batch result')
primary = ROOT/PREFIX/'index.json'
secondary = ROOT/PREFIX/'verification/index.json'
for path,digest in ((primary,'af9931a32cb6e60a0cd4390845056b8df5eea027fb6fa1ef9d8834573389835c'),
        (secondary,'898b7e76175199294c744a18ef33ff77f838e029584c200e8e4aceff8aff86b7')):
    if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
        raise ValueError('sealed index differs')
    index = json.loads(path.read_bytes())
    for row in index['members'].values():
        data = (path.parent/'objects'/row['sha256']).read_bytes()
        if len(data) != row['bytes'] or hashlib.sha256(data).hexdigest() != row['sha256']:
            raise ValueError('sealed object differs')
result = dict(staged_packet_blobs=len(names),raw_byte_mismatches=0,
              primary_members=219,verification_members=53,package_pass=False,release_admitted=False)
with OUT.open('x') as stream:
    json.dump(result,stream,sort_keys=True,indent=2)
    stream.write('\n')
print(result)
