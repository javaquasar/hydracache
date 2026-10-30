"""Independent data-only nominal identity accounting, explicitly not Root admission."""
import json,re,sys,tomllib
from pathlib import Path,PurePosixPath
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/chitchat-public-identity-074'));import contract as s
c=s.c;m=s.m
PHASES=('stage','resolve','build194','negative194','build188','negative188')

def owned(get,prefix,bound=780,expect=0):
    intent=json.loads(get(prefix+'intent.json'));owner=json.loads(get(prefix+'execution.json'))
    c.need(owner['retired'] and owner['errors']==[] and owner['exit']==expect,'complete original nominal owner')
    c.need(owner['bound_seconds']==intent['bound']==bound and owner['capacity_poll_seconds']==owner['cleanup_seconds']==5 and intent['retry'] is False,'same original nominal bounds/retirement')
    return intent

def source_audit(get):
    audit=json.loads(get('input/audit.json'));body=get('reference/root/crates/hydracache-cluster-chitchat/src/lib.rs')
    c.need(audit['public_methods']==21 and audit['exposed']==s.signatures(body.decode()) and audit['adapter_source_sha256']==c.sha(body),'exact full21/three public boundary audit')
    root=tomllib.loads(get('reference/root/Cargo.toml').decode())['workspace'];direct=[]
    c.need(root['dependencies']['chitchat']=='0.11.0','unchanged actual Root alias')
    for member in root['members']:
        doc=tomllib.loads(get('reference/root/'+member+'/Cargo.toml').decode())
        def visit(row,path=()):
            if not isinstance(row,dict):return
            for key,value in row.items():
                if key=='chitchat' and path and path[-1] in ('dependencies','dev-dependencies','build-dependencies'):direct.append(dict(member=member,section='.'.join(path),value=value))
                visit(value,(*path,key))
        visit(doc)
    c.need(direct==audit['direct'] and {r['member']:r['section'] for r in direct}==s.DEPENDENTS and len(direct)==4 and all(r['value']=={'workspace':True} for r in direct),'all actual direct manifests and shared alias')
    c.need(audit['full_adapter_compiled'] is False and audit['root_adopted'] is False,'no adapter compilation claim')
    for name,rows in audit['references'].items():
        text=get('reference/root/'+name).decode().splitlines()
        c.need(rows==[dict(line=i,text=line) for i,line in enumerate(text,1) if 'chitchat::' in line],'whole held conservative textual reference')

def graph(get):
    ledger=json.loads(get('input/registry.json'));c.need(len(ledger)==95 and ledger['chitchat-0.11.1']==m.UPSTREAM_SHA,'exact original94 plus official legacy root')
    reference=json.loads(get('input/registry-reference.json'));all_nodes=[]
    accepted={v:s.edges(get('input/accepted-'+v+'.lock')) for v in ('B','Z','U')};allowed={}
    for edges in accepted.values():
        for key,values in edges.items():allowed.setdefault(key,set()).update(values)
    downloads=json.loads(get('state/downloads.json'))
    for variant in s.VARIANTS:
        data=json.loads(get('native/resolve/metadata-'+variant+'/stdout.log'));nodes=m.registry_nodes(get('state/lock-'+variant+'.toml'));all_nodes.append(nodes)
        edges=s.edges(get('state/lock-'+variant+'.toml'))
        for row in nodes:
            key=row['name'],row['version']
            if row['name']==m.NAME:
                c.need(row['checksum']==s.CRATE_SHA and row['source']==reference['registry'],'exact owned package source/checksum');children=accepted['U']['chitchat','0.11.1']
            else:
                c.need(row['source']=='registry+'+m.CRATE_INDEX and ledger[row['name']+'-'+row['version']]==row['checksum'],'all actual ordinary registry nodes exact')
                children=accepted['B']['chitchat','0.11.1'] if row['name']=='chitchat' else allowed[key]
            c.need(edges[key]<=children,'no new accepted graph edge')
        root=next(v for v in data['packages'] if v['id']==data['resolve']['root']);deps={d['rename']:d for d in root['dependencies']}
        c.need(root['name']=='hc074-identity-'+variant and root['source'] is None and set(deps)=={'legacy','owned'} and deps['legacy']['name']=='chitchat' and deps['owned']['name']==m.NAME,'same actual two real normal aliases')
        for name,source,version,archive,prefix in [('chitchat','registry+'+m.CRATE_INDEX,'0.11.1','legacy.crate','chitchat-0.11.1'),(m.NAME,reference['registry'],m.VERSION,'owned.crate',m.PREFIX)]:
            matches=[v for v in data['packages'] if v['name']==name]
            c.need(len(matches)==1 and matches[0]['source']==source and matches[0]['version']==version,'actual real package metadata identity')
            row=downloads[name];files=m.codec().unpack(get('input/'+archive),prefix)
            c.need(row['source']==source and row['version']==version and row['files']=={n:c.sha(b) for n,b in files.items()} and matches[0]['manifest_path']==row['path']+'/Cargo.toml','whole actual downloaded package, no mock/overlay')
        owned(get,'native/resolve/metadata-'+variant+'/')
    c.need(all(v==all_nodes[0] for v in all_nodes),'same whole registry graph across all four controls')
    return len(all_nodes[0])

def diagnostic(rows,variant,src):
    errors=[r for r in rows if r.get('reason')=='compiler-message' and r['message']['level']=='error']
    c.need(len(errors)==1 and errors[0]['message']['code']['code']==s.ERRORS[variant],'exact expected one nominal failure')
    row=errors[0];c.need(row['target']['src_path']==src and any(v['is_primary'] and v['line_start']==2 and v['file_name'] in ('src/lib.rs',src) for v in row['message']['spans']),'actual probe span, no dependency failure')
    c.need([v['success'] for v in rows if v.get('reason')=='build-finished']==[False],'actual Cargo failure verdict')
    return dict(complete=True,expected_code=s.ERRORS[variant],actual_code=s.ERRORS[variant],probe=src,root_adopted=False)

def reconcile(get):
    source_audit(get);count=graph(get)
    parent=json.loads(get('input/parent-index.json'));c.need(c.sha(get('input/parent-index.json'))==s.PARENT_SHA and parent['result']['full_library_gate_pass'] and parent['result']['consumer_cases']==12,'immutable accepted local package parent')
    c.need(c.sha(get('input/owned.crate'))==s.CRATE_SHA and c.sha(get('input/legacy.crate'))==m.UPSTREAM_SHA,'whole two real archives')
    origin=get('reference/root-map-origin-index.json')
    c.need(c.sha(origin)==parent['members']['reference/parent-index.json']['sha256'],'same negative R5 parent origin')
    origin=json.loads(origin);row=origin['members']['capture/'+m.BASE.name+'/input/frozen-root-map.json']
    c.need(c.sha(get('reference/frozen-root-map.json'))==row['sha256'],'same complete original Root map')
    original=json.loads(get('reference/frozen-root-map.json'))
    for name in ['Cargo.toml',*[n+'/Cargo.toml' for n in tomllib.loads(get('reference/root/Cargo.toml').decode())['workspace']['members']],*json.loads(get('input/audit.json'))['references']]:
        c.need(c.sha(get('reference/root/'+name))==original[name],'whole actual audited Root source bytes')
    for phase in PHASES:
        prefix='native/'+phase+'/';intent=owned(get,prefix,1200);result=json.loads(get(prefix+'result.json'));inner=json.loads(get(prefix+'session-result.json'))
        c.need(result['complete'] and inner['complete'] and all(result[k] is False and inner[k] is False for k in ('root_adopted','adapter_compiled','release_admitted')),'complete original narrowly scoped phase')
        c.need(intent['session_bound']==1100 and intent['floor']==4*1024**3 and all(intent['source'][n]==h for n,h in original.items()),'all original Root fields and admission bounds')
        base=s.BASE.relative_to(s.ROOT).as_posix()
        for variant in s.VARIANTS:
            for name in ('Cargo.toml','src/lib.rs'):
                relative='source/'+variant+'/'+name
                c.need(intent['source'][base+'/'+relative]==c.sha(get(relative)),'whole actual fixed probe source/manifest')
        for name in ('owned.crate','legacy.crate','registry-reference.json','parent-index.json','registry.json','accepted-B.lock','accepted-Z.lock','accepted-U.lock','audit.json','scope.json'):
            relative='input/'+name;c.need(intent['source'][base+'/'+relative]==c.sha(get(relative)),'whole fixed nominal input frame')
        for name in ('.gitattributes','README.md','contract.py','runner.py','offline.py','observation.py','test_contract.py','positive.rs','config.rs','id.rs','transport.rs'):
            relative='tools/chitchat-public-identity-074/'+name
            c.need(intent['source'][relative]==c.sha(get('source/current/'+relative)),'whole actually executed nominal producer')
    passed=0;refusals=[]
    for compiler,version in [('194','1.94.0'),('188','1.88.0')]:
        prefix='native/build'+compiler+'/'
        c.need(('release: '+version+'\n').encode() in get(prefix+'rustc-version/stdout.log'),'actual positive compiler')
        for action in ('build','check','clippy'):owned(get,prefix+action+'/')
        argv=json.loads(get(prefix+'clippy/intent.json'))['argv'];c.need('--all-targets' in argv and '--all-features' in argv and argv[-3:]==['--','-D','warnings'],'strict complete positive check')
        records=[json.loads(v) for v in get(prefix+'build/stdout.log').decode().splitlines() if v.startswith('{')]
        for name in ('chitchat',m.NAME):
            built=[v for v in records if v.get('reason')=='compiler-artifact' and v['target']['name']=='chitchat' and v['target']['kind']==['lib'] and ('#'+name+'@' in v['package_id'] or '#'+name+'/' in v['package_id'])]
            c.need(len(built)==1 and not built[0]['fresh'] and not built[0]['profile']['test'],'fresh actual normal dependencies')
        names=json.loads(get(prefix+'catalog.json'));image=json.loads(get(prefix+'image.json'));c.need(len(names)==len(set(names))==3,'whole three-case positive catalog')
        owned(get,prefix+'catalog/',10)
        for i,name in enumerate(names):
            folder=prefix+f'case-{i:02}/';intent=owned(get,folder,5);text=get(folder+'stdout.log').decode()
            c.need(intent['argv']==[image['path'],'--exact',name,'--test-threads=1','--nocapture'] and intent['image_sha256']==image['sha256'],'actual held positive image/case')
            c.need(re.findall(r'^test (.+?) \.\.\. ',text,re.M)==[name] and '1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out;' in text,'whole unignored positive case');passed+=1
        for variant in ('config','id','transport'):
            folder='native/negative'+compiler+'/'+variant+'/';intent=owned(get,folder,expect=101)
            c.need('--lib' in intent['argv'] and '--message-format=json' in intent['argv'] and '--locked' in intent['argv'] and '--offline' in intent['argv'],'exact bounded negative compiler invocation')
            src=s.N.as_posix()+'/source/'+variant+'/src/lib.rs';records=[json.loads(v) for v in get(folder+'stdout.log').decode().splitlines() if v.startswith('{')]
            result=diagnostic(records,variant,src);c.need(result==json.loads(get(folder+'negative-review.json')),'same actual expected compiler diagnostic');refusals.append(dict(compiler=version,variant=variant,code=result['actual_code']))
    for mode in ('controls','syntax','format','registry-observation'):
        owned(get,'offline/'+mode+'/',1200);c.need(json.loads(get('offline/'+mode+'/result.json'))['complete'],'complete source/observation owner')
    c.need(b'Ran 15 tests' in get('offline/controls/stderr.log') and get('offline/controls/stderr.log').rstrip().endswith(b'OK'),'all15 producer controls')
    observation=json.loads(get('public-registry/response.json'))
    c.need(all(observation[k] is False for k in ('name_reserved','ownership_proven','publication_authorized','root_adopted','retry')),'no publication/ownership inference')
    if observation['response_received']:c.need(c.sha(get('public-registry/response.body'))==observation['response_sha256'] and len(get('public-registry/response.body'))==observation['response_bytes'],'whole exact first registry observation')
    proposal=json.loads(get('proposal/root-change.json'));body=get('reference/root/Cargo.toml');old=proposal['exact_before'].encode();new=proposal['exact_after'].encode()
    c.need(proposal['schema']=='hc074-chitchat-identity-migration-proposal-v1' and proposal['file']=='Cargo.toml' and body.count(old)==proposal['matching_occurrences']==1,'one exact proposed Root preimage')
    c.need(c.sha(body)==proposal['before_sha256'] and c.sha(body.replace(old,new))==proposal['after_sha256'] and proposal['crate_sha256']==s.CRATE_SHA,'whole exact unapplied proposal binding')
    c.need(proposal['rust_source_changes']==0 and proposal['external_direct_dependency_migration_required'] and all(proposal[k] is False for k in ('root_adopted','proposal_applied','publication_authorized')),'no proposal adoption or transparent compatibility inference')
    return dict(complete=True,nominal_control_gate_pass=True,public_methods=21,exposed_boundaries=3,direct_consumers=4,registry_nodes=count,
        actual_positive_cases=passed,expected_compile_refusals=refusals,mixed_identity_assignable=False,public_metadata_status=observation.get('http_status'),
        parent_sha256=s.PARENT_SHA,cargo_crate_sha256=s.CRATE_SHA,root_adopted=False,full_adapter_compiled=False,Root_public_type_compatibility_qualified=False,
        security_qualified=False,publication_authorized=False,public_registry_qualified=False,MTU_qualified=False,CPU_qualified=False,RSS_qualified=False,release_admitted=False)
