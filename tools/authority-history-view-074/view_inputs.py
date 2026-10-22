"""Bounded generated historical views, never a runtime candidate admission API."""
from contextlib import contextmanager
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import stat
from unittest.mock import patch
import zipfile

SOURCE='33c54b0cda8c330b115009cdf092ad002077cedf'
INPUT_SHA='8959f5c799bab4aef03e5b90cec48fbff461cf4f032bf5dce724f69794091d60'
BASE='docs/testing/performance/0.74/local-runs'
FIXTURE='tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs'
FIXTURE_SHA='3b3c75f41dbd8a8db1f73d61af3b0c701a666c3ebeed326ae49012ed0c01fb6c'


def need(value,label):
    if not value:raise ValueError(label)


def sha(data):return hashlib.sha256(data).hexdigest()
def canonical(data):return data.decode('utf-8-sig').replace('\r','').encode()


def unique(items):
    result={}
    for key,value in items:
        need(key not in result,'Duplicate JSON key');result[key]=value
    return result


def decode(data):
    def invalid(value):raise ValueError('Non-finite JSON constant: '+value)
    return json.loads(data,object_pairs_hook=unique,parse_constant=invalid)


def safe_name(name):
    need(type(name) is str and re.fullmatch(r'[A-Za-z0-9_./-]+',name) is not None,'Unsafe archive name')
    path=PurePosixPath(name)
    need(bool(path.parts) and not path.is_absolute() and path.as_posix()==name and all(p not in ('.','..') for p in path.parts),'Archive traversal/alias')
    need(all(not p.endswith('.') for p in path.parts),'Windows path alias')
    need(all(p.split('.')[0].upper() not in {'CON','PRN','AUX','NUL',*(f'COM{i}' for i in range(10)),*(f'LPT{i}' for i in range(10))} for p in path.parts),'Windows device name')
    return name


def validate_manifest(m):
    need(m['schema']=='authority-history-view-inputs-074-v1' and m['source_commit']==SOURCE,'Wrong frozen source')
    need(m['source_representation']=='utf-8-lf-no-bom','Wrong original text representation')
    need(len(m['source_files'])==129 and len(m['reader_modules'])==18 and len(m['packets'])==24,'Wrong fixed input membership')
    need(m['packet_members']==1423 and type(m['packet_members']) is int,'Wrong sealed member count')
    need(m['bounds']==dict(files=2048,archive_bytes=67108864,total_bytes=67108864,file_bytes=16777216),'Frozen view bounds changed')
    seen=set()
    for name,pin in m['source_files'].items():
        safe_name(name);need(not name.startswith(BASE+'/'),'Source/evidence scopes overlap')
        need(name.casefold() not in seen,'Case-aliased source');seen.add(name.casefold())
        need(type(pin) is str and re.fullmatch('[0-9a-f]{64}',pin) is not None,'Wrong source digest')
    need(len(set(m['reader_modules']))==18 and all(p in m['source_files'] and p.endswith('.py') for p in m['reader_modules']),'Unpinned reader module')
    need(m['reader']=='tools/offline-authority-integration-audit-074/audit_integration.py' and m['reader'] in m['reader_modules'],'Wrong replay reader')
    for name,pin in m['packets'].items():
        need(re.fullmatch('[a-z0-9-]+',name) is not None and re.fullmatch('[0-9a-f]{64}',pin) is not None,'Wrong packet identity')
    need(m['expected_facts']==BASE+'/authority-integration-audit-20261022/facts.json','Wrong historical output')
    need(m['source_files'][FIXTURE]==FIXTURE_SHA,'Wrong baseline fixture')
    return m


def manifest():
    data=Path(__file__).with_name('inputs.json').read_bytes()
    need(sha(canonical(data))==INPUT_SHA,'Input manifest drift')
    return validate_manifest(decode(data))


def expected_members(m,read):
    expected={name:('source',pin,None) for name,pin in m['source_files'].items()};members=0
    for name,pin in m['packets'].items():
        prefix=BASE+'/'+name+'/';inventory=prefix+'inventory.json';data=read(inventory)
        need(sha(data)==pin,'Historical inventory digest drift')
        expected[inventory]=('data',pin,len(data))
        entries=decode(data)['files'];need(type(entries) is list,'Wrong packet member list')
        for entry in entries:
            path=prefix+safe_name(entry['path'])
            need(path not in expected and type(entry['bytes']) is int and 0<=entry['bytes']<=m['bounds']['file_bytes'],'Duplicate/oversized packet member')
            need(type(entry['sha256']) is str and re.fullmatch('[0-9a-f]{64}',entry['sha256']) is not None,'Wrong member digest')
            expected[path]=('data',entry['sha256'],entry['bytes']);members+=1
    need(members==m['packet_members'] and len(expected)<=m['bounds']['files'],'Historical member count drift')
    need(len({p.casefold() for p in expected})==len(expected),'Case-aliased view member')
    return expected


def check_bytes(data,entry,archived=False):
    kind,pin,size=entry
    need(size is None or len(data)==size,'Member byte length drift')
    need(sha(canonical(data) if kind=='source' and archived else data)==pin,'Member source/content drift')


def parents(names):
    return {p.as_posix() for name in names for p in PurePosixPath(name).parents if p.as_posix()!='.'}


def extract(archive,destination,m):
    destination=Path(destination)
    need(destination.is_dir() and not any(destination.iterdir()) and not destination.is_symlink() and not destination.is_junction(),'Fresh nonlinked view required')
    with zipfile.ZipFile(archive) as z:
        entries=z.infolist();need(len(entries)<=4096,'Archive entry bound')
        names={};directories=set();seen=set();total=0
        for entry in entries:
            name=safe_name(entry.filename[:-1] if entry.is_dir() else entry.filename)
            need(name.casefold() not in seen,'Duplicate/case-aliased archive entry');seen.add(name.casefold())
            kind=stat.S_IFMT(entry.external_attr>>16)
            need(kind in (0,stat.S_IFDIR if entry.is_dir() else stat.S_IFREG),'Linked/special archive entry')
            need(not entry.flag_bits&1,'Encrypted archive entry')
            if entry.is_dir():directories.add(name);continue
            need(0<=entry.file_size<=m['bounds']['file_bytes'],'Archive member bound')
            total+=entry.file_size;need(total<=m['bounds']['total_bytes'],'Archive total bound');names[name]=entry
        def read(name):
            need(name in names,'Missing archived inventory');return z.read(names[name])
        expected=expected_members(m,read)
        need(set(names)==set(expected) and directories<=parents(expected),'Unknown/missing archived member')
        # Validate every byte before materialization or importing historical code.
        for name,entry in expected.items():check_bytes(read(name),entry,archived=True)
        for name in expected:
            path=destination/name;path.parent.mkdir(parents=True,exist_ok=True)
            data=read(name)
            if expected[name][0]=='source':data=canonical(data)
            check_bytes(data,expected[name])
            with path.open('xb') as output:output.write(data)
    return verify_tree(destination,m)


def verify_tree(view,m):
    view=Path(view);need(not view.is_symlink() and not view.is_junction(),'Linked historical root');view=view.resolve()
    def read(name):
        path=view/name
        need(path.resolve().is_relative_to(view),'Historical input escapes view')
        need(all(not p.is_symlink() and not p.is_junction() for p in (path,*path.parents) if p.is_relative_to(view)),'Linked historical input ancestry')
        need(path.is_file(),'Missing historical input')
        return path.read_bytes()
    expected=expected_members(m,read);files=set();directories=set();total=0
    for path in view.rglob('*'):
        need(not path.is_symlink() and not path.is_junction(),'Linked historical tree entry')
        name=path.relative_to(view).as_posix()
        if path.is_dir():directories.add(name);continue
        need(path.is_file() and name in expected,'Unknown historical tree entry');files.add(name)
        need(path.stat().st_size<=m['bounds']['file_bytes'],'View member bound')
        data=read(name);total+=len(data);need(total<=m['bounds']['total_bytes'],'View total bound');check_bytes(data,expected[name])
    need(files==set(expected) and directories==parents(expected),'Missing/extra historical tree entry')
    fingerprint=sha(json.dumps({name:entry for name,entry in sorted(expected.items())},sort_keys=True).encode())
    return dict(files=len(files),sources=len(m['source_files']),packets=len(m['packets']),packet_members=m['packet_members'],fingerprint=fingerprint)


@contextmanager
def read_only_scope(view):
    """Diagnostic Path.open guard for trusted pinned readers, not a sandbox."""
    view=Path(view);need(not view.is_symlink() and not view.is_junction(),'Linked historical root');view=view.resolve();original=Path.open
    def guarded(path,mode='r',*args,**kwargs):
        need(mode in ('r','rb','rt'),'Historical reader attempted a write')
        resolved=Path(path).resolve();need(resolved.is_relative_to(view),'Historical reader attempted outside-view access')
        return original(path,mode,*args,**kwargs)
    with patch.object(Path,'open',guarded):yield


def current_fixture_identity(root):
    need(sha((Path(root)/FIXTURE).read_bytes())==FIXTURE_SHA,'Current fixture differs; no candidate semantic guard exists')
    return dict(current_fixture_baseline_identity=True,runtime_candidate_guard_implemented=False)
