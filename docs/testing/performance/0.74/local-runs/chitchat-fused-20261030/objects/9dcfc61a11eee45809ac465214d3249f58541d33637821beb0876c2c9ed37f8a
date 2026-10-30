"""Closed historical F review: data only, no archived producers or executable use."""
import hashlib
import json
import re

sha=lambda raw:hashlib.sha256(raw).hexdigest()
def need(ok,reason):
    if not ok:raise ValueError(reason)

def reconcile(get):
    def j(name):return json.loads(get('capture/'+name))
    def phase(name,bound,passed):
        result=j(name+'/result.json');owner=j(name+'/execution.json');intent=j(name+'/intent.json')
        need(result['complete'] and result.get('passed',passed)==passed,'complete selected outcome: '+name)
        need(owner['exit']==0 and owner['retired'] and owner['errors']==[],'retired selected owner: '+name)
        need(owner['bound_seconds']==intent['bound']==bound and owner['capacity_poll_seconds']==owner['cleanup_seconds']==5 and intent['floor']==4*1024**3,'unchanged envelopes: '+name)
        if passed:
            text=get('capture/'+name+'/stdout.log').decode().replace('\r\n','\n')
            rows=re.findall(r'^test (.+?) (?:- should panic )?\.\.\. (ok|FAILED)$',text,re.M)
            need(len(rows)==passed and len({n for n,_ in rows})==passed and all(r=='ok' for _,r in rows),'actual selected identities: '+name)
            need(re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',text,re.M)==[(str(passed),'0','0','0',str(126-passed))],'complete actual summary: '+name)
        return result
    def artifact(name,variant):
        phase(name,1200,0)
        rows=[json.loads(s) for s in get('capture/'+name+'/stdout.log').decode().splitlines() if s.startswith('{')]
        images=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('executable') and r['profile']['test'] and r['target']['kind']==['lib']]
        need(len(images)==1 and images[0]['fresh'] is False and [r['success'] for r in rows if r.get('reason')=='build-finished']==[True],'fresh actual library artifact')
        need(images[0]['target']['src_path'].replace('\\','/').endswith('/fixtures/'+variant+'/src/lib.rs'),'actual artifact source identity')
        return images[0]
    baseline=j('counterexample-B/result.json')
    need(baseline['complete'] and baseline['expected_failure'] and baseline['failed']==1,'immutable first baseline red')
    need('assertion failed: !w.try_append_borrowed(&item, budget)' in get('capture/counterexample-B/stdout.log').decode(),'MTU refusal is the baseline failure')
    for suffix in ('','-188'):
        artifact('build3-F'+suffix,'F')
        catalog=phase('length-guard/catalog-F'+suffix,10,0)['catalog']
        need(len(catalog)==len(set(catalog))==126,'complete final Windows catalog')
        for action in ('check','clippy'):phase('length-guard/'+action+'-F'+suffix,1200,0)
        phase('length-guard/compat-F'+suffix,1200,70)
        for i in range(10):phase('length-guard/case'+str(i)+'-F'+suffix,5,1)
        for i in range(2):phase('length-guard/old'+str(i)+'-F'+suffix,5,1)
    def diagnostic(raw,marker):
        lines=[s.split(marker,1)[1] for s in raw.decode().splitlines() if marker in s]
        need(len(lines)==1,'one diagnostic marker');return json.loads(lines[0])
    # Independent original B source/bytes: linked immutable C2 packet member.
    original=get('reference/compat-B0/stdout.log')
    peers=[get('capture/length-guard/compat-F'+s+'/stdout.log') for s in ('','-188')]
    for marker,fields,count in [('HC074_SUPPORTED_DELTA:',('accepted','actual','mtu','remaining','value_len','wire'),239),
       ('HC074_STREAM_BOUNDS:',('actual','id','payload','remaining','threshold','wire'),37)]:
        data=[diagnostic(raw,marker) for raw in [original]+peers]
        need(data[1]==data[2],'complete F compiler agreement')
        need(all(len(v['cases'])==count and v['seed']==7606312 for v in data),'whole frozen corpus/seed')
        projected=[[[row[k] for k in fields] for row in v['cases']] for v in data]
        need(projected[0]==projected[1]==projected[2],'original admissions/wire/cursor equality')
        if marker=='HC074_STREAM_BOUNDS:':need(all(r['bound_held'] for r in data[1]['cases']),'all supported final bounds')
    malformed=[diagnostic(raw,'HC074_SECURITY_MALFORMED:') for raw in [original]+peers]
    need(malformed[0]==malformed[1]==malformed[2] and len(malformed[0]['rows'])==7,'full malformed/cursor decisions')
    reports={};images={}
    for v,build in [('B','build-B'),('F','build3-F')]:
        artifact('retention/'+build,v)
        owner=j('retention/probe-'+v+'/execution.json');intent=j('retention/probe-'+v+'/intent.json')
        result=j('retention/probe-'+v+'/result.json')
        need(owner['exit']==0 and owner['retired'] and not owner['errors'] and owner['bound_seconds']==intent['bound']==5,'original resource owner')
        raw=diagnostic(get('capture/retention/probe-'+v+'/stdout.log'),'HC074_RETAINED_FINISH:')
        need(result['complete'] and result['observation']==raw,'raw whole resource observation')
        need(raw['variant']==v and raw['records'][0]==raw['records'][1] and len(raw['records'])==2,'both predeclared observations, exact identity')
        need(raw['scope']=='returned Vec capacity only' and not raw['CPU_qualified'] and not raw['RSS_qualified'],'closed resource scope')
        reports[v]=raw;images[v]=intent['image_sha256']
    need(images['B']!=images['F'],'different actual baseline/candidate images')
    b,f=reports['B'],reports['F']
    for key in ('seed','name','threshold','budget','payload'):need(b[key]==f[key],'same resource workload')
    need((b['seed'],b['name'],b['threshold'],b['budget'])==(7606312,'immediate-exact-tail',64,167),'frozen first stateful case')
    seed=7606312;payload=[]
    for _ in range(128):
        seed^=(seed<<13)&((1<<64)-1);seed^=seed>>7;seed^=(seed<<17)&((1<<64)-1);payload.append(seed&255)
    need(b['payload']==(bytes(payload)+b'r'*32).hex(),'original payload recipe')
    before,after=b['records'][0],f['records'][0]
    need(before['wire']==after['wire'] and before['finish_len']==after['finish_len']==155,'same complete accepted bytes')
    need((before['finish_capacity'],after['finish_capacity'])==(256,268),'first exact retained-output counterexample')
    need(before['append_capacity']==[256,256,64] and after['append_capacity']==[134,128,64],'original append checkpoints')
    analysis=j('retention/analysis.json')
    need(analysis['complete'] and analysis['delta_bytes']==12 and analysis['decision']=='declined-first-retained-output-regression','final refusal classification')
    for key in ('resource_gate_pass','full_allocation_gate_pass','further_measurements_allowed','CPU_qualified','RSS_qualified','root_adopted','release_admitted'):
        need(analysis[key] is False,'negative scope remains negative: '+key)
    stop=j('STOP.json');need(stop['sha256']==sha(get('capture/retention/analysis.json')) and stop['further_measurements_allowed'] is False,'sealed first resource refusal')
    need(j('native-r1/session-result.json')['complete'] is False and 'clippy-F' in j('native-r1/session-result.json')['first_refusal']['reason'],'first GNU refusal preserved')
    need('int_plus_one' in get('capture/native-r1/clippy-F/stderr.log').decode(),'diagnostic lint attribution')
    need(j('native-r2-not-executed.json')['executed'] is False,'no second native run')
    root_pins={'Cargo.lock':'8ade63f6a4aae93f27f5161798bc87804542a48093b4203f35e25e29b680a4f5',
      'deny.toml':'16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930',
      'qualification-manifest.toml':'11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'}
    for name,digest in root_pins.items():need(sha(get('reference/root/'+name))==digest,'unchanged Root/qualification input')
    # Exact live source bytes for the successful last Windows and resource peers.
    for name in ('capture/length-guard/compat-F/intent.json','capture/length-guard/compat-F-188/intent.json','capture/retention/probe-F/intent.json'):
        intent=json.loads(get(name))
        for path,digest in intent['source'].items():
            if path.startswith(('tools/chitchat-fused-074/','tools/chitchat-fused-result-074/')):
                need(sha(get('capture/contexts/pre-stop/'+path))==digest,'exact original executed producer bytes')
            elif path.startswith('target/chitchat-fused-20261030/fixtures/') or path.startswith('target/chitchat-fused-20261030/retention/fixtures/'):
                relative=path.removeprefix('target/chitchat-fused-20261030/')
                need(sha(get('capture/'+relative))==digest,'exact complete executed source fixture')
    return dict(complete=True,decision=analysis['decision'],resource_gate_pass=False,retained_delta_bytes=12,
      final_Windows_positive_executions=164,retention_observations_per_image=2,
      further_measurements_allowed=False,CPU_qualified=False,RSS_qualified=False,root_adopted=False,release_admitted=False)
