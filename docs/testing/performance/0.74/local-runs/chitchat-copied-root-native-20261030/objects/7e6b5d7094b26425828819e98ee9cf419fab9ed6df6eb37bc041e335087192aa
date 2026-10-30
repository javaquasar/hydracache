"""Pure semantic accounting of retained actual Cargo inputs and outputs."""
import hashlib,json,re,tomllib
CRATES='registry+https://github.com/rust-lang/crates.io-index'
OWNED='hydra-chitchat';VERSION='0.11.1-hydra.1'
CRATE_SHA='0296600cf25af830741ee575df5da4e050d48e7bc561ffbdcda63ebab12877c2'
ROOT='/home/artur/.cache/hydracache-074-copied-root/20261030-a3'
PACKAGES={'adapter':'hydracache-cluster-chitchat','testkit':'hydracache-cluster-testkit','raft':'hydracache-cluster-raft','sandbox':'hydracache-sandbox'}

def need(ok,why):
    if not ok:raise ValueError(why)

def edges(rows):
    result={}
    for row in rows:
        key=(row['name'],row['version'],row.get('source'));need(key not in result,'unique whole lock identity');children=set()
        for text in row.get('dependencies',[]):
            fields=text.split(' ',2);matches=[p for p in rows if p['name']==fields[0] and (len(fields)==1 or p['version']==fields[1]) and (len(fields)<3 or p.get('source')==fields[2][1:-1])]
            need(len(matches)==1,'unambiguous semantic dependency');p=matches[0];children.add((p['name'],p['version'],p.get('source')))
        result[key]=children
    return result

def lock_review(before,after,ledger,registry):
    a=tomllib.loads(before.decode())['package'];b=tomllib.loads(after.decode())['package']
    old={(p['name'],p['version'],p.get('source')):p for p in a};new={(p['name'],p['version'],p.get('source')):p for p in b}
    ea,eb=edges(a),edges(b);removed=set(old)-set(new);added=set(new)-set(old)
    for p in b:
        if p.get('source'):
            if p['name']==OWNED:need((p['version'],p['source'],p['checksum'])==(VERSION,registry,CRATE_SHA),'exact maintained archive/registry identity')
            else:need(p['source']==CRATES and ledger[p['name']+'-'+p['version']]==p['checksum'],'no unrelated registry identity change')
    need(any(k[0]=='chitchat' for k in removed) and any(k[0]==OWNED for k in added),'actual selected identity substitution')
    need(all(k[0] in ('chitchat','lru','zstd','zstd-safe') for k in removed) and all(k[0] in (OWNED,'lru') for k in added),'only approved node delta')
    for key in old.keys()&new.keys():
        need({n:v for n,v in old[key].items() if n!='dependencies'}=={n:v for n,v in new[key].items() if n!='dependencies'},'same whole retained node')
        if ea[key]!=eb[key]:
            need(key[0] in PACKAGES.values(),'only actual four callers change edges')
            expected={(OWNED,VERSION,registry) if n=='chitchat' else (n,v,s) for n,v,s in ea[key]}
            need(expected==eb[key],'only exact selected identity semantic edge substitution')
    return dict(removed=[list(x) for x in sorted(removed,key=str)],added=[list(x) for x in sorted(added,key=str)])

def aliases(data,variant,registry):
    root=ROOT+'/copies/'+variant;need(data['workspace_root']==root,'actual complete copied workspace')
    names=[p for p in data['packages'] if p['name'] in PACKAGES.values()];need({p['name'] for p in names}==set(PACKAGES.values()) and len(names)==4,'all four actual caller packages')
    chosen=[p for p in data['packages'] if p['name']==('chitchat' if variant=='B' else OWNED)]
    wrong=[p for p in data['packages'] if p['name']==(OWNED if variant=='B' else 'chitchat')]
    need(len(chosen)==1 and not wrong,'one coherent selected identity')
    package=chosen[0];need(package['version']==('0.11.1' if variant=='B' else VERSION) and package['source']==(CRATES if variant=='B' else registry),'exact selected normal dependency source/version')
    for p in names:
        need(p['manifest_path']==root+'/crates/'+p['name']+'/Cargo.toml','real caller path, no mock')
        node=next(n for n in data['resolve']['nodes'] if n['id']==p['id']);dep=[d for d in node['deps'] if d['name']=='chitchat']
        need(len(dep)==1 and dep[0]['pkg']==package['id'],'every caller uses identical nominal identity')
    return dict(package=package['name'],version=package['version'],source=package['source'],callers=sorted(p['name'] for p in names),packages=len(data['packages']))

def reconcile(data):
    get=lambda name:json.loads(data[name]);prefix='third/native/';result=dict(stage_complete=False,metadata_complete=False,accepted_lanes=[],first_refusal=None,root_adopted=False,release_admitted=False,performance_claim=False)
    if prefix+'stage/result.json' in data:
        result['stage_complete']=get(prefix+'stage/result.json')['complete']
    phase=prefix+'metadata/'
    if phase+'result.json' in data and get(phase+'result.json')['complete']:
        registry=get('first/input/registry-reference.json')['registry'];ledger=get('first/input/registry.json')
        need(data['third/state/lock-B.toml']==data['first/input/base.lock'],'byte-exact baseline lock')
        result['lock_delta']=lock_review(data['first/input/base.lock'],data['third/state/lock-O.toml'],ledger,registry)
        result['aliases']={v:aliases(get('third/state/metadata-'+v+'.json'),v,registry) for v in ('B','O')}
        result['metadata_complete']=True
    for name in sorted(data):
        match=re.fullmatch(r'third/native/(check|lint|test)-(B|O)-(194|188)-(adapter|testkit|raft|sandbox)/result\.json',name)
        if not match:continue
        action,v,compiler,short=match.groups();phase=name.rsplit('/',1)[0]+'/';row=get(name)
        if not row['complete']:continue
        need(result['stage_complete'] and result['metadata_complete'],'actual admitted parent gates')
        argv=get(phase+'cargo-'+action+'/intent.json')['argv'];tool='/home/artur/.rustup/toolchains/'+('1.94.0' if compiler=='194' else '1.88.0')+'-x86_64-unknown-linux-gnu/bin/cargo'
        expected=[tool,'clippy' if action=='lint' else action,'--manifest-path',ROOT+'/copies/'+v+'/Cargo.toml','--target-dir',ROOT+'/builds/'+compiler,'-p',PACKAGES[short],'--locked','--offline']
        expected+=['--all-targets','--all-features',*(['--','-D','warnings'] if action=='lint' else [])] if action!='test' else ['--tests','--all-features','--no-fail-fast','--message-format=json','--','--test-threads=4']
        need(argv==expected,'exact unchanged focused package command/profile')
        if v=='O':need(get(prefix+action+'-B-'+compiler+'-'+short+'/result.json')['complete'],'same baseline before candidate')
        if action!='check':need(get(prefix+('check' if action=='lint' else 'lint')+'-'+v+'-'+compiler+'-'+short+'/result.json')['complete'],'exact check/lint prerequisite')
        lane=dict(action=action,variant=v,compiler=compiler,package=PACKAGES[short])
        if action=='test':
            raw=data[phase+'cargo-test/stdout.log'].decode();runs=re.findall(r'^running (\d+) tests?$',raw,re.M);rows=re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',raw,re.M)
            need(runs and len(runs)==len(rows),'whole unfiltered actual default catalogs');passed=ignored=0
            for count,outcome in zip(runs,rows,strict=True):
                a,b,c,d,e=map(int,outcome);need(b==d==e==0 and a+c==int(count),'complete original case accounting');passed+=a;ignored+=c
            images=get(phase+'cargo-test/images.json');need(len(images)==len(rows),'all actual executed test images')
            lane.update(passed=passed,ignored=ignored,targets=len(rows))
        result['accepted_lanes'].append(lane)
    if 'third/STOP.json' in data:result['first_refusal']=get('third/STOP.json')['phase']
    result['full_focused_matrix_complete']=len(result['accepted_lanes'])==48
    if 'third/cleanup-result-next.json' in data:
        rows=get('third/cleanup-result-next.json')['deleted'];need(len(rows)==8 and all(r['absent'] for r in rows),'exact inactive old cache cleanup')
        result['additional_generated_cache_bytes_removed']=sum(r['bytes'] for r in rows)
    return result
