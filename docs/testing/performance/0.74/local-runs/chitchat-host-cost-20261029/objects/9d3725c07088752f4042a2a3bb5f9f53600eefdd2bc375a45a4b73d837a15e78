"""Read only the recorded root; no launch, replacement, resume or retry command."""
import io
import json
import re
import tarfile
from pathlib import Path
import sys
import dispatch as d
import prepare
import remote


def root():
    value=json.loads((d.OUT/'remote.json').read_bytes())['root']
    prepare.need(re.fullmatch(r'/tmp/hc074-chitchat-control-root-20261029\.[A-Za-z0-9]{6}',value),'exact recorded root')
    return value


def status():
    value=root()
    script='''import json,os,pathlib
r=pathlib.Path(ROOT)
if (r/'result.json').is_file():
 print(json.dumps(dict(state='complete',result=json.loads((r/'result.json').read_bytes()))))
elif (r/'started.json').is_file():
 s=json.loads((r/'started.json').read_bytes());p=pathlib.Path('/proc')/str(s['owner_pid'])/'stat'
 live=p.is_file() and p.read_text().rsplit(') ',1)[1].split()[19]==s['owner_start_ticks']
 print(json.dumps(dict(state='running' if live else 'owner-lost',started=s)))
else:print(json.dumps(dict(state='not-started-inspect-same-root')))
'''.replace('ROOT',repr(value))
    result=__import__('subprocess').run(d.SSH+['sudo -n /usr/bin/python3 -B -'],input=script.encode(),capture_output=True,timeout=45)
    prepare.need(result.returncode==0,'read-only status transport failed');print(result.stdout.decode().strip())


def collect():
    value=root()
    script='''import os,pathlib,sys,tarfile
r=pathlib.Path(ROOT)
assert (r/'result.json').is_file()
with tarfile.open(fileobj=sys.stdout.buffer,mode='w|') as t:
 for folder,dirs,names in os.walk(r):
  folder=pathlib.Path(folder)
  if folder==r:dirs[:]=[n for n in dirs if n!='input']
  for name in sorted(names):
   p=folder/name
   if name=='payload.tar.gz':continue
   assert p.is_file() and not p.is_symlink()
   t.add(p,arcname=p.relative_to(r).as_posix(),recursive=False)
'''.replace('ROOT',repr(value))
    d.call('collect',d.SSH+['sudo -n /usr/bin/python3 -B -'],data=script.encode())
    raw=(d.OUT/'collect.stdout.log').read_bytes();dest=d.OUT/'collection';dest.mkdir(exist_ok=False)
    with tarfile.open(fileobj=io.BytesIO(raw)) as t:
        members=t.getmembers();prepare.need(len({m.name for m in members})==len(members),'unique collected files')
        for item in members:
            prepare.need(item.isfile() and not item.name.startswith('/') and '..' not in item.name.split('/'),'contained collection')
            body=t.extractfile(item).read();path=dest/item.name;path.parent.mkdir(parents=True,exist_ok=True)
            with path.open('xb') as out:out.write(body)
    print(json.dumps(json.loads((dest/'result.json').read_bytes())))


if __name__=='__main__':
    prepare.need(sys.argv[1:] in (['status'],['collect']),'read-only status/one collection only')
    (status if sys.argv[1]=='status' else collect)()
