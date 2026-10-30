"""Independent data-only accounting; no archived producer/image execution."""
import json,sys,re,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/chitchat-package-r5-074'));import model as m
BASE=m.BASE
PHASES=('stage','resolve','package','build194','build188','pure194','rest194','pure188','rest188','registry','consumer194','consumer188')

def owned(get,prefix,bound=1200,expect=0):
    value=json.loads(get(prefix+'execution.json'));intent=json.loads(get(prefix+'intent.json'))
    m.need(value['retired'] and value['errors']==[] and value['exit']==expect,'complete original retired owner')
    m.need(value['bound_seconds']==intent['bound']==bound and value['capacity_poll_seconds']==value['cleanup_seconds']==5,'unchanged ownership bounds')
    return intent

def reconcile(get):
    _,u,lock=m.parent();source=m.project(u,lock);source['Cargo.lock']=get('state/resolved.lock');m.verify_source(source,u)
    raw=get('artifact/'+m.PREFIX+'.crate');files=m.verify_crate(raw,u,source);receipt=json.loads(get('artifact/receipt.json'))
    m.need(receipt['sha256']==m.sha(raw) and receipt['members']=={n:m.sha(b) for n,b in files.items()} and receipt['root_adopted'] is False and receipt['registry_qualified'] is False,'whole actual Cargo archive, no promotion')
    root_map=json.loads(get('input/frozen-root-map.json'))
    positive=('stage','resolve','package','build194','build188','pure194')
    for phase in (*positive,'rest194'):
        base='native/'+phase+'/';result=json.loads(get(base+'result.json'));inner=json.loads(get(base+'session-result.json'))
        expected=phase in positive
        m.need(result['complete'] is expected and inner['complete'] is expected,'exact first phase verdict')
        m.need(all(result[k] is False for k in ('root_adopted','registry_qualified','release_admitted')),'no phase scope promotion')
        intent=owned(get,base,expect=0 if expected else 1)
        m.need(intent['session_bound']==1100 and intent['floor']==4*1024**3 and intent['retry'] is False,'unchanged original outer/session contract')
        m.need(all(intent['source'][n]==h for n,h in root_map.items()),'complete original Root-map frame')
    argv=owned(get,'native/package/cargo-package/',780)['argv']
    m.need('package' in argv and '--locked' in argv and '--offline' in argv and '--no-verify' not in argv,'real Cargo package/build verifier')
    catalogs=[]
    for compiler,version in [('194','1.94.0'),('188','1.88.0')]:
        base='native/build'+compiler+'/'
        m.need(('release: '+version+'\n').encode() in get(base+'rustc-version/stdout.log'),'actual compiler version')
        for action in ('build','check','clippy'):owned(get,base+action+'/')
        argv=json.loads(get(base+'clippy/intent.json'))['argv']
        m.need('--all-targets' in argv and '--all-features' in argv and argv[-3:]==['--','-D','warnings'],'full strict unchanged lint contract')
        names=json.loads(get('state/catalog-'+compiler+'.json'));m.need(len(names)==len(set(names))==100,'actual whole library catalog');catalogs.append(names)
        rows=[json.loads(s) for s in get(base+'build/stdout.log').decode().splitlines() if s.startswith('{')]
        images=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('executable') and r['target']['kind']==['lib'] and r['profile']['test']]
        image=json.loads(get('state/image-'+compiler+'.json'))
        m.need(len(images)==1 and not images[0]['fresh'] and images[0]['executable']==image['path'],'fresh actual extracted-package test image')
    m.need(catalogs[0]==catalogs[1],'same complete catalogs')
    image=json.loads(get('state/image-194.json'));passed=0;selections=[]
    for mode,limit in [('pure',57),('rest',26)]:
        prefix='native/'+mode+'194/';selection=json.loads(get(prefix+'selection.json'));selections+=selection['selected']
        m.need(set(selection['selected'])|set(selection['unexecuted'])==set(catalogs[0]),'complete declared test partition')
        for i,test in enumerate(selection['selected'][:limit]):
            folder=prefix+f'case-{i:03}/';intent=owned(get,folder,5)
            text=get(folder+'stdout.log').decode()
            m.need(intent['argv']==[image['path'],'--exact',test,'--test-threads=1','--nocapture'] and intent['image_sha256']==image['sha256'],'original exact held test image')
            m.need(re.findall(r'^test (.+?) (?:- should panic )?\.\.\. ',text,re.M)==[test] and '1 passed; 0 failed; 0 ignored; 0 measured; 99 filtered out;' in text,'whole passing outcome')
            passed+=1
    m.need(len(selections)==len(set(selections))==100 and set(selections)==set(catalogs[0]),'whole intended catalog, no omission hidden')
    prefix='native/rest194/case-026/';test='tests::test_dead_node_garbage_collection'
    intent=owned(get,prefix,5,expect=124);owner=json.loads(get(prefix+'execution.json'))
    m.need(owner['classification']=='timeout' and intent['argv']==[image['path'],'--exact',test,'--test-threads=1','--nocapture'] and intent['image_sha256']==image['sha256'],'actual five-second timeout, retired exact original image')
    text=get(prefix+'stdout.log').decode();m.need('test '+test+' ...' in text and 'test result:' not in text,'first unfinished result, no invented assertion verdict')
    m.need(json.loads(get('STOP.json'))['phase']=='rest194' and test in json.loads(get('native/rest194/session-result.json'))['first_refusal']['reason'],'same latched first refusal')
    lib=source['src/lib.rs']
    m.need(b'const DEAD_NODE_GRACE_PERIOD: Duration = Duration::from_secs(20);' in lib and b'let wait_for = DEAD_NODE_GRACE_PERIOD.add(Duration::from_secs(5));\n        time::sleep(wait_for).await;' in lib,'unchanged mandatory real25-second wait')
    m.need(b'#[tokio::test]\n    async fn test_dead_node_garbage_collection()' in lib,'original non-paused upstream clock method')
    for mode in ('controls','syntax','format'):
        owned(get,'offline/'+mode+'/');m.need(json.loads(get('offline/'+mode+'/result.json'))['complete'],'first offline source gate')
    m.need(b'Ran 18 tests' in get('offline/controls/stderr.log') and get('offline/controls/stderr.log').rstrip().endswith(b'OK'),'all18 source/archive controls')
    for name in ('owned','upstream'):
        prefix='policy/commands/'+name+'/';result=json.loads(get(prefix+'result.json'));intent=owned(get,prefix)
        m.need(result['complete'] and result['policy_passed'] and result['database_commit']=='2295466c4e9df26647fd67d8fa1c705ac2fc7f81','same complete fresh policy under both identities')
        m.need(intent['argv'][-1]=='all' and '--disable-fetch' in intent['argv'] and '--offline' in intent['argv'] and '--locked' in intent['argv'],'unchanged full policy command')
        for line in get(prefix+'stderr.log').decode().splitlines():
            try:fields=json.loads(line).get('fields',{})
            except json.JSONDecodeError:continue
            m.need(fields.get('severity')!='error','no raw policy error')
    return dict(complete=True,preparation_complete=False,full_library_gate_pass=False,terminal_phase='rest194',
        source_tar_sha256=m.sha(get('input/source.tar')),cargo_crate_sha256=m.sha(raw),crate_bytes=len(raw),source_members=len(files),
        reversible_condition_sites=8,format_only_sites=1,library_cases_passed=passed,library_cases_timed_out=1,
        library_cases_unexecuted_194=16,library_cases_unexecuted_188=100,source_controls=18,consumer_cases=0,
        mandatory_wait_seconds=25,case_bound_seconds=5,policy_identities=['owned','upstream'],
        upstream_integration_runtime_qualified=False,local_registry_qualified=False,public_registry_qualified=False,
        upstream_MTU_fixed=False,resources_qualified=False,CPU_qualified=False,RSS_qualified=False,root_adopted=False,release_admitted=False)

if __name__=='__main__':
    result=reconcile(lambda name:(BASE/name).read_bytes());folder=BASE/'analysis';folder.mkdir(exist_ok=True)
    with (folder/'result.json').open('x',encoding='utf8') as f:json.dump(result,f,indent=2,sort_keys=True)
    print(json.dumps(result))
