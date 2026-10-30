"""Reconcile the separately approved full catalog and real registry callers."""
import importlib.util,json,re,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/chitchat-package-complete-074'));import context as s
m=s.m;c=s.c

def load_parent():
    # Generic historical helper names must not capture this child's partially
    # initialized review/packet modules. Load CURRENT reviewed source explicitly
    # into private namespaces, and restore the caller's import binding.
    folder=ROOT/'tools/chitchat-package-result-074'
    def load(name,path):
        spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
    module=load('hc074_parent_package_review',folder/'review.py');previous=sys.modules.get('review')
    try:
        sys.modules['review']=module
        return load('hc074_parent_package_packet',folder/'packet.py')
    finally:
        if previous is None:sys.modules.pop('review',None)
        else:sys.modules['review']=previous

parent=load_parent()

def owned(get,prefix,bound=1200,expect=0):
    intent=json.loads(get(prefix+'intent.json'));own=json.loads(get(prefix+'execution.json'))
    c.need(own['retired'] and own['errors']==[] and own['exit']==expect,'complete original full-lane owner')
    c.need(own['bound_seconds']==intent['bound']==bound and own['capacity_poll_seconds']==own['cleanup_seconds']==5 and intent['retry'] is False,'unchanged original bound/poll/retirement')
    return intent

def reconcile(get,old):
    prefix='capture/'+s.PARENT.name+'/'
    parent_result=parent.r.reconcile(lambda n:old[prefix+n]);c.need(not parent_result['full_library_gate_pass'],'immutable R5 negative')
    root_map=json.loads(old[prefix+'input/frozen-root-map.json']);catalogs=[]
    inherited={n.removeprefix('source/current/'):c.sha(b) for n,b in old.items()
        if n.startswith('source/current/tools/chitchat-package-r5-074/') and not n.endswith('/.gitattributes')}
    for phase in s.PHASES:
        base='native/'+phase+'/';result=json.loads(get(base+'result.json'));session=json.loads(get(base+'session-result.json'));intent=owned(get,base)
        c.need(result['complete'] and session['complete'] and all(result[k] is False for k in ('root_adopted','public_registry_qualified','release_admitted')),'complete separately scoped phase')
        c.need(intent['session_bound']==1100 and intent['floor']==4*1024**3 and all(intent['source'][n]==h for n,h in root_map.items()),'same whole original Root/floor/native frame')
        # Only Git's CRLF/license whitespace declaration changed after sealing;
        # all inherited executable helpers/recipes/inputs remain exact parent.
        c.need(inherited and all(intent['source'][n]==h for n,h in inherited.items()),'exact current parent helper bytes, no archived Python execution')
    for compiler in ('194','188'):
        names=json.loads(old[prefix+'state/catalog-'+compiler+'.json']);image=json.loads(old[prefix+'state/image-'+compiler+'.json'])
        base='native/library'+compiler+'/';intent=owned(get,base+'full-library/',780)
        c.need(intent['argv']==[image['path'],'--test-threads=1','--nocapture'] and intent['image_sha256']==image['sha256'],'same fresh original extracted image, no filter')
        result=s.full_outcome(get(base+'full-library/stdout.log'),names)
        c.need(result==json.loads(get(base+'library-review.json')),'raw whole100-case reconciliation');catalogs.append(names)
    c.need(catalogs[0]==catalogs[1],'same complete upstream suite')
    registry=json.loads(get('native/registry/registry-review.json'));c.need(registry['complete'] and registry['local_registry_resolved'] and registry['checksum_negative'] and registry['patch_fallback'] is False and registry['public_registry_qualified'] is False,'whole real local registry receipt')
    owned(get,'native/registry/registry-seed/');owned(get,'native/registry/checksum-negative/',expect=101)
    error=get('native/registry/checksum-negative/stderr.log').decode();c.need('checksum' in error.lower() and m.NAME in error,'actual corrupt-download refusal')
    metadata=json.loads(get('native/registry/registry-seed/stdout.log'));fork=[p for p in metadata['packages'] if p['name']==m.NAME]
    frame=json.loads(get('state/consumer.json'));raw=get('artifact/'+m.PREFIX+'.crate')
    c.need(len(fork)==1 and fork[0]['source']==frame['registry'] and frame['registry'].startswith('registry+file:///') and fork[0]['version']==m.VERSION and frame['checksum']==c.sha(raw)==parent_result['cargo_crate_sha256'],'actual independent exact Cargo package/source/checksum')
    consumers=0
    for compiler,version in [('194','1.94.0'),('188','1.88.0')]:
        base='native/consumer'+compiler+'/';result=json.loads(get(base+'consumer-review.json'))
        c.need(result['complete'] and result['actual_registry'] and result['passed']==6 and result['public_registry_qualified'] is False,'complete actual external caller')
        c.need(('release: '+version+'\n').encode() in get(base+'rustc-version/stdout.log'),'actual caller compiler')
        for action in ('default-check','build','check','clippy','catalog'):owned(get,base+action+'/',10 if action=='catalog' else 1200)
        argv=json.loads(get(base+'clippy/intent.json'))['argv'];c.need('--all-targets' in argv and '--all-features' in argv and argv[-3:]==['--','-D','warnings'],'same strict all-target/all-feature caller lint')
        names=json.loads(get(base+'catalog.json'));image=json.loads(get(base+'image.json'));c.need(len(names)==len(set(names))==6,'whole actual caller catalog')
        rows=[json.loads(v) for v in get(base+'build/stdout.log').decode().splitlines() if v.startswith('{')]
        built=[r for r in rows if r.get('reason')=='compiler-artifact' and r['target']['name']=='chitchat' and r['target']['kind']==['lib']]
        c.need(len(built)==1 and not built[0]['fresh'] and not built[0]['profile']['test'] and built[0]['target']['src_path']==frame['downloaded_path']+'/src/lib.rs','fresh normal registry dependency, no test overlay')
        for i,name in enumerate(names):
            folder=base+f'case-{i:02}/';intent=owned(get,folder,5);text=get(folder+'stdout.log').decode()
            c.need(intent['argv']==[image['path'],'--exact',name,'--test-threads=1','--nocapture'] and intent['image_sha256']==image['sha256'],'exact held caller image')
            c.need(re.findall(r'^test (.+?) (?:- should panic )?\.\.\. ',text,re.M)==[name] and '1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out;' in text,'whole unignored caller result');consumers+=1
    scope=json.loads(get('input/scope.json'));c.need(scope['operator_agreed_full_lane'] and scope['parent_sha256']==s.SHA and scope['library_bound']==780 and scope['short_case_bound']==5,'explicit separate lane scope')
    return dict(complete=True,full_library_gate_pass=True,full_library_cases=200,consumer_cases=consumers,local_registry_qualified=True,
        parent_sha256=s.SHA,parent_short_case_gate_pass=False,parent_first_refusal_preserved=True,cargo_crate_sha256=c.sha(raw),
        public_registry_qualified=False,Root_public_type_compatibility_qualified=False,Windows_qualified=False,upstream_integration_runtime_qualified=False,
        upstream_MTU_fixed=False,resources_qualified=False,CPU_qualified=False,RSS_qualified=False,root_adopted=False,release_admitted=False)
