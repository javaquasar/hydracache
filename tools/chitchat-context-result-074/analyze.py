"""Data-only C1 prerequisite admission. No archived code or timing inference."""
import hashlib
import ast
import json
from pathlib import Path
import re
import traceback

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'target/chitchat-borrowed-20261030'
FIELDS={'malloc_calls','calloc_calls','realloc_calls','free_calls','allocs','requested','failed','external_frees','live','peak'}
DOMINANCE=('allocs','realloc_calls','requested','peak','live')
CHECKPOINTS=('construct','append1','append2','finish','retirement')


def need(value,why):
    if not value:raise ValueError(why)


def reconcile(before,after):
    for value in (before,after):
        need(set(value)=={'threshold','repetitions',*CHECKPOINTS,'wire','variant','context_present','CPU_qualified','RSS_qualified','allocator_page_retention_qualified'},'complete exact observation schema')
        need(value['variant'] in ('B0','C1') and isinstance(value['context_present'],list) and all(type(x) is bool for x in value['context_present']) and value['context_present']==[False,value['variant']=='C1',value['variant']=='C1'],'actual runtime source/context identity')
        need(type(value['threshold']) is int and value['threshold']==64 and type(value['repetitions']) is int and value['repetitions']==30,'fixed original context workload')
        need(all(value[k] is False for k in ('CPU_qualified','RSS_qualified','allocator_page_retention_qualified')),'no unmeasured page/timing promotion')
        need(isinstance(value['wire'],list) and value['wire'] and all(type(v) is int and 0<=v<=255 for v in value['wire']),'whole exact wire bytes')
        previous={v:{k:0 for k in ('allocs','realloc_calls','requested','peak')} for v in ('c','rust')}
        for point in CHECKPOINTS:
            item=value[point]
            if point in CHECKPOINTS[:3]:
                need(isinstance(item,list) and len(item)==2,'complete snapshot and inventory')
                snap,inventory=item
                need(set(inventory)=={'len','capacity'} and len(inventory['len'])==len(inventory['capacity'])==3,'all original writer Vecs')
                need(all(type(a) is int and type(b) is int and 0<=a<=b for a,b in zip(inventory['len'],inventory['capacity'],strict=True)),'valid complete Vec inventory')
            else:snap=item
            need(set(snap)=={'c','rust','errors'} and type(snap['errors']) is int and snap['errors']==0,'no native counter ownership errors')
            for owner in ('c','rust'):
                row=snap[owner]
                need(set(row)==FIELDS and all(type(v) is int and v>=0 for v in row.values()),'complete nonnegative integral physical counters')
                need(row['failed']==row['external_frees']==0 and row['live']<=row['peak'],'successful complete owned region')
                for k in previous[owner]:
                    need(row[k]>=previous[owner][k],'monotonic cumulative counters')
                    previous[owner][k]=row[k]
                if point=='retirement':need(row['live']==0,'balanced complete drop')
    need(before['wire']==after['wire'],'identical actual finished wire')
    failures=[]
    for point in CHECKPOINTS:
        b,a=before[point],after[point]
        if point in CHECKPOINTS[:3]:
            for vector in range(3):
                need(a[1]['len'][vector]==b[1]['len'][vector],'same logical writer state')
                if a[1]['capacity'][vector]>b[1]['capacity'][vector]:
                    failures.append(dict(point=point,owner='Vec'+str(vector),field='capacity',before=b[1]['capacity'][vector],after=a[1]['capacity'][vector]))
            b,a=b[0],a[0]
        for owner in ('c','rust'):
            for field in DOMINANCE:
                if a[owner][field]>b[owner][field]:
                    failures.append(dict(point=point,owner=owner,field=field,before=b[owner][field],after=a[owner][field]))
    return dict(complete=True,prerequisite_passed=not failures,violations=failures,
        decision='prerequisite-subgate-only' if not failures else 'declined-context-retention-stop-combined-candidate',
        source_only=True,root_adopted=False,release_admitted=False,mtu_repaired=False,
        CPU_qualified=False,RSS_qualified=False,allocator_page_retention_qualified=False)


def analyze(read):
    load=lambda n:json.loads(read(n))
    need(load('native-r5/image-B0.json')['sha256']!=load('native-r5/image-C1.json')['sha256'],'C1 must not reuse the baseline binary from a shared Cargo target')
    for name,digest in [('Cargo.lock','8ade63f6a4aae93f27f5161798bc87804542a48093b4203f35e25e29b680a4f5'),
        ('deny.toml','16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930'),
        ('docs/testing/performance/0.74/qualification-manifest.toml','11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc')]:
        need(hashlib.sha256(read('root/'+name)).hexdigest()==digest,'unchanged exact Root/policy/qualification')
    constants={}
    for node in ast.parse(read('source/current/recipe.py')).body:
        if isinstance(node,ast.Assign) and len(node.targets)==1 and isinstance(node.targets[0],ast.Name) and node.targets[0].id in ('STATELESS','OWNED'):
            constants[node.targets[0].id]=ast.literal_eval(node.value)
    need(set(constants)=={'STATELESS','OWNED'} and all(isinstance(v,bytes) for v in constants.values()),'literal exact source transformations, no archived execution')
    before=read('fixtures/B0/src/serialize.rs');after=read('fixtures/C1/src/serialize.rs')
    need(before.count(constants['STATELESS'])==1 and before.replace(constants['STATELESS'],constants['OWNED'])==after,'only the one private context lifetime function changes')
    metadata=load('preparation/candidate.json')
    need(hashlib.sha256(before).hexdigest()==metadata['baseline_sha256'] and hashlib.sha256(after).hexdigest()==metadata['candidate_sha256'],'original pre-implementation source identities')
    for n in ('native-r5/result.json','native-r5/session-result.json'):
        value=load(n);need(value['complete'] and value['root_adopted'] is False and value['resources_qualified'] is False,'whole first native context completed without scope promotion')
    outer=load('native-r5/execution.json')
    need(outer['exit']==0 and outer['retired'] and not outer['errors'] and outer['bound_seconds']==1200 and outer['cleanup_seconds']==outer['capacity_poll_seconds']==5,'original complete outer Job')
    frame=load('native-frame-r5.json')
    for name,digest in frame.items():need(hashlib.sha256(read('native-input-r5/'+name)).hexdigest()==digest,'whole original native input frame')
    prep=load('native-input-r5/preparation.json')
    need(set(prep['fixtures'])=={'B0','C1'} and set(prep['fixtures']['B0'])==set(prep['fixtures']['C1']),'same complete input file set')
    for name,digest in prep['fixtures']['B0'].items():
        if name!='src/serialize.rs':need(prep['fixtures']['C1'][name]==digest,'no second source/dependency hypothesis')
    wrapper=read('native-input-r5/wrap.c');original=read('source/native/wrap.c');peek=read('source/current/peek.c')
    need(wrapper==original+b'\n'+peek and hashlib.sha256(original).hexdigest()==prep['unchanged_wrapper_sha256'],'exact qualified wrapper plus only read-only peek')
    need(read('native-input-r5/control.c')==read('source/native/control.c') and read('native-input-r5/wrap.h')==read('source/native/wrap.h'),'old whole C controls and ABI unchanged')
    for variant in ('B0','C1'):
        for name in prep['fixtures'][variant]:
            raw=read('fixtures/'+variant+'/'+name)
            need(hashlib.sha256(raw).hexdigest()==prep['fixtures'][variant][name],'whole immutable Windows source fixture')
            native=read('native-input-r5/fixtures/'+variant+'/'+name)
            if name=='src/serialize.rs':need(native==raw+b'\n#[cfg(test)]\n#[path="native_lifetime.rs"]\nmod native_lifetime;\n','only private diagnostic module is appended')
            else:need(native==raw,'same complete native source input')
        observer=read('source/current/native_tests.rs')
        if variant=='C1':observer=observer.replace(b'const SOURCE_VARIANT: &str = "B0";',b'const SOURCE_VARIANT: &str = "C1";').replace(b'const EXPECTED_CONTEXT: bool = false;',b'const EXPECTED_CONTEXT: bool = true;')
        need(read('native-input-r5/fixtures/'+variant+'/src/native_lifetime.rs')==observer,'exact same observer apart from two diagnostic identity literals')
    commands=('source-control','gcc-version','ld-version','compile-wrapper','compile-control','control','compile-peek_control','peek_control','compiler',
        *[action+'-'+v for v in ('B0','C1') for action in ('build','catalog','check','clippy','zstd-imports','linked-zstd','controls','observe')])
    records=[]
    for label in commands:
        intent=load('native-r5/'+label+'/intent.json');owner=load('native-r5/'+label+'/execution.json')
        need(owner['exit']==0 and owner['retired'] and not owner['errors'] and owner['cleanup_seconds']==owner['capacity_poll_seconds']==5,'complete original native owner: '+label)
        expected=5 if label in ('control','peek_control') or label.startswith(('controls-','observe-')) else 10 if label.startswith('catalog-') else 1200
        need(intent['bound']==expected and owner['bound_seconds']==expected and intent['retry'] is False,'unchanged exact command envelope')
    controls=read('native-r5/source-control/stderr.log').decode()
    need(re.search(r'^Ran 5 tests in ',controls,re.M) and len(re.findall(r' \.\.\. ok$',controls,re.M))==5 and controls.rstrip().endswith('OK'),'all five fresh source-custody controls actually execute')
    for label,marker in [('control','HC074_NATIVE_C_CONTROLS:'),('peek_control','HC074_PEEK_CONTROL:')]:
        raw=read('native-r5/'+label+'/stdout.log').decode();need(raw.count(marker)==1,'whole original C control output')
        value=json.loads(raw.split(marker)[1].splitlines()[0]);need(value['complete'] is True,'complete native control')
        if label=='control':need(value['groups']==7 and value['thread_isolation'] is True,'all original C control groups')
    for v in ('B0','C1'):
        catalog=read('native-r5/catalog-'+v+'/stdout.log').decode()
        names=[s[:-6] for s in catalog.splitlines() if s.endswith(': test')]
        old=load('catalog-'+v+'/result.json')['catalog']
        need(sorted(names)==sorted(old+['serialize::native_lifetime::'+n for n in ('allocator_controls','context_lifetime_observation')]) and catalog.splitlines()[-1]=='111 tests, 0 benchmarks','all complete original case identities')
        image=load('native-r5/image-'+v+'.json')
        built=read('native-r5/build-'+v+'/stdout.log').decode()
        rows=[json.loads(s) for s in built.splitlines() if s.startswith('{')]
        artifacts=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('executable') and r['target']['kind']==['lib'] and r['profile']['test']]
        need(len(artifacts)==1 and artifacts[0]['fresh'] is False and artifacts[0]['executable']==image['path'],'one fresh actual independent compilation')
        argv=load('native-r5/build-'+v+'/intent.json')['argv']
        need(argv.count('--target')==1 and argv[argv.index('--target')+1]=='x86_64-unknown-linux-gnu','diagnostic flags excluded from host proc macros')
        target=argv[argv.index('--target-dir')+1]
        need(target.endswith('/builds/'+v) and image['path'].startswith(target+'/x86_64-unknown-linux-gnu/release/deps/'),'independent per-input target')
        for label,test in [('controls','allocator_controls'),('observe','context_lifetime_observation')]:
            base='native-r5/'+label+'-'+v+'/'
            intent=load(base+'intent.json');need(intent['image_sha256']==image['sha256'] and intent['argv']==[image['path'],'--exact','serialize::native_lifetime::'+test,'--test-threads=1','--nocapture'],'actual exact native image and case')
            raw=read(base+'stdout.log').decode()
            need(re.findall(r'^test (.+?) \.\.\. ',raw,re.M)==['serialize::native_lifetime::'+test],'one actual case header')
            need(re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',raw,re.M)==[('1','0','0','0','110')],'whole selected result accounting')
            if label=='observe':
                marker='HC074_CONTEXT_LIFETIME:';need(raw.count(marker)==1,'one complete native observation')
                records.append(json.loads(raw.split(marker)[1].splitlines()[0]))
        imports=read('native-r5/zstd-imports-'+v+'/stdout.log').decode()
        linked=read('native-r5/linked-zstd-'+v+'/stdout.log').decode()
        need(all(re.search(r'\bU '+name+r'\b',imports) for name in ('malloc','calloc','free')),'actual static Zstd default C allocator imports')
        need('ZSTD_createCCtx' in linked and re.search(r'\bcall\s+[0-9a-f]+\s+<__wrap_calloc>',linked),'actual linked compressor creation calls wrapped calloc')
    need([v['variant'] for v in records]==['B0','C1'],'two actual runtime inputs, never two baseline observations')
    result=reconcile(*records);result['observations']=dict(zip(('B0','C1'),records,strict=True));return result


def main():
    out=BASE/'analysis-r5-final';out.mkdir(exist_ok=False)
    (out/'producer.py').write_bytes(Path(__file__).read_bytes())
    def read(name):
        if name.startswith('source/current/'):return (ROOT/'tools/chitchat-borrowed-074'/name.removeprefix('source/current/')).read_bytes()
        if name.startswith('source/native/'):return (ROOT/'tools/chitchat-native-cost-074'/name.removeprefix('source/native/')).read_bytes()
        if name.startswith('root/'):return (ROOT/name.removeprefix('root/')).read_bytes()
        return (BASE/name).read_bytes()
    try:result=analyze(read)
    except BaseException as error:
        result=dict(complete=False,first_refusal=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc()),root_adopted=False,release_admitted=False)
    (out/'result.json').write_text(json.dumps(result,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    print(json.dumps({k:v for k,v in result.items() if k!='observations'}))
    raise SystemExit(0 if result['complete'] else 1)


if __name__=='__main__':main()
