"""Reconcile raw functional receipts only; execute no archived code or test image."""
import io
import json
import re
import zipfile
import common as c

R0='chitchat-compat-20261029'
R1=R0+'-r1'
PEERS=((R0,'gnu-194'),(R0,'windows-194'),(R1,'gnu-188'),(R1,'windows-188'))
GROUPS=('serialize::tests::','delta::tests::','digest::tests::','message::tests::','state::tests::',
    'serialize::mtu_review::','migration_review_074::snapshot','delta::mtu_review::','delta::oversize_review::',
    'serialize::inplace_review::','serialize::state_review::','serialize::q_review::')
EXACT=('delta::cost_review::allocator_control','serialize::stateful_review::stateful_semantics_control',
    'serialize::stateful_review::stateful_capacity_control','delta::cost_review::actual_caller::fixed_caller_semantics_control',
    'delta::cost_review::actual_caller::fixed_caller_capacity_control','delta::cost_review::actual_caller::compatibility_semantic_export')
FIRST='04c99c4abf6ac08bb12d417e62dff85a0f67280ca3f66590e9ff86ebe85da67d'
J=lambda raw:json.loads(raw)


def local(name):
    return (c.ROOT/'target'/name).read_bytes()


def owner(row,bound=1200):
    c.need(row==dict(bound_seconds=bound,capacity_poll_seconds=5,classification='success',cleanup_seconds=5,
        errors=[],exit=0,observed_child_exit=0,production_writer_authority=False,retired=True),'complete original owned outcome')


def capsule(get,context):
    frame_raw=get(context+'/frame.json');frame=J(frame_raw);frozen=J(get(context+'/format/result.json'))
    body=get(context+'/source.zip')
    c.need(frozen['complete'] and c.sha(frame_raw)==frozen['frame_sha256'] and c.sha(body)==frozen['zip_sha256'],'original source capsule')
    with zipfile.ZipFile(io.BytesIO(body)) as archive:
        c.need(len(archive.namelist())==len(set(archive.namelist())) and set(archive.namelist())==set(frame)|{'target/'+context+'/frame.json'},'complete unique capsule inventory')
        for name,digest in frame.items():
            c.need(not name.startswith('/') and '..' not in name.split('/') and c.sha(archive.read(name))==digest,'contained exact source member')
        c.need(archive.read('target/'+context+'/frame.json')==frame_raw,'embedded source frame')
    return frame


def semantic(get,context,raw):
    body=c.diagnostic(raw,'HC074_COMPAT_CALLER:')
    c.need(body['seed']==0x0741028 and body['variant']=='Q' and body['table_sha256']==c.TABLE_SHA,'exact caller identity')
    c.need(all(body[k] is False for k in ('observer_active','native_C_qualified','CPU_qualified','resources_qualified')),'semantic-only scope')
    c.need(len(body['rows'])==30 and body['rows']==J(get(context+'/input/expected-caller.json')),'complete original caller wire/admissions')
    rows=c.diagnostic(raw,'HC074_SUPPORTED_DELTA:')['cases'];original=J(get(context+'/input/cases.json'))
    project=lambda data:[tuple(v[k] for k in ('mtu','value_len','accepted','wire')) for v in data]
    c.need(len(rows)==239 and len({(r['mtu'],r['value_len']) for r in rows})==239 and project(rows)==project(original),'original 239 workload/order/wire')
    return body


def peer(get,context,label,frame):
    prefix=context+'/'+label+'/'
    c.need(J(get(prefix+'result.json'))==dict(complete=True,root_adopted=False),'whole first peer completion')
    owner(J(get(prefix+'execution.json')))
    intent=J(get(prefix+'intent.json'))
    c.need(intent['bound']==1200 and intent['session_bound']==1100 and intent['retry'] is False,'unchanged full session contract')
    windows=label.startswith('windows-');host='x86_64-pc-windows-msvc' if windows else 'x86_64-unknown-linux-gnu'
    version='1.88.0' if label.endswith('188') else '1.94.0'
    if not windows:owner(J(get(prefix+'native-envelope.json')))
    inner=prefix+('' if windows else 'native/')
    c.need(J(get(inner+'session-result.json'))==dict(complete=True,root_adopted=False,performance_claim=False,
        native_C_qualified=False,CPU_qualified=False,RSS_qualified=False),'whole semantic-only inner session')
    env=J(get(inner+'environment.json'))
    c.need(env['compiler']==version and env['host']==host and env['native_C_available'] is False and env['resource_cohort'] is False,'exact platform/scope')
    c.need(set(env['names'])==set(env['value_sha256']) and len(env['names'])==len(set(env['names'])),'full environment accounting')
    frame_sha=c.sha(get(context+'/frame.json'));deadlines=[];commands={}
    for name in ('compiler','build','list','tests','check','clippy'):
        commands[name]=call=J(get(inner+name+'/intent.json'));owner(J(get(inner+name+'/execution.json')))
        c.need(call['retry'] is False and call['frame']==frame_sha and '--ignore-rust-version' not in call['argv'],'original command frame')
        c.need(re.fullmatch('[0-9a-f]{64}',call['image_sha256']) is not None,'bound actual command image')
        deadlines.append(call['deadline'])
    c.need(len(set(deadlines))==1 and type(deadlines[0]) in (int,float),'one original session deadline without reset')
    compiler=get(inner+'compiler/stdout.log').decode()
    c.need('release: '+version+'\n' in compiler and 'host: '+host+'\n' in compiler,'raw compiler qualification')
    image=J(get(inner+'artifact.json'));images=[]
    for line in get(inner+'build/stdout.log').decode().splitlines():
        row=J(line)
        if row.get('reason')=='compiler-artifact' and row.get('profile',{}).get('test') and row.get('executable'):images.append(row)
    c.need(len(images)==1 and images[0]['executable']==image['path'],'one actual built library image')
    for name in ('list','tests'):
        c.need(commands[name]['argv'][0]==image['path'] and commands[name]['image_sha256']==image['sha256'],'original built/tested image join')
    c.need(commands['compiler']['argv'][1:]==['-vV'],'compiler invocation')
    c.need(commands['list']['argv'][1:]==['--list'],'whole actual catalog invocation')
    for name in ('build','check','clippy'):
        argv=commands[name]['argv']
        c.need(argv[1]==('test' if name=='build' else name) and all(v in argv for v in ('--offline','--locked','--release','--target','--manifest-path','--target-dir')) and argv[argv.index('--target')+1]==host,'exact offline release Cargo shape')
        c.need(('/target/'+context+'/fixtures/Q/Cargo.toml') in argv[argv.index('--manifest-path')+1].replace('\\','/'),'exact Q manifest')
        if name=='build':c.need(argv[-5:]==['--lib','--features','testsuite','--no-run','--message-format=json'],'same diagnostic build')
        else:c.need('--all-targets' in argv and '--all-features' in argv,'complete check scope')
        if name=='clippy':c.need(argv[-3:]==['--','-D','warnings'],'strict Clippy')
    catalog=[line[:-6] for line in get(inner+'list/stdout.log').decode().splitlines() if line.endswith(': test')]
    c.need(catalog==J(get(inner+'catalog.json')) and len(catalog)==len(set(catalog)) and len(catalog)==(133 if windows else 132),'complete catalog')
    c.need(sorted(catalog)==J(get(context+'/input/catalog-'+('windows' if windows else 'gnu')+'.json')),'expected complete catalog')
    selected=sorted(n for n in catalog if n in EXACT or any(n.startswith(p) for p in GROUPS))
    c.need(len(selected)==85 and not any('observations' in n for n in selected),'fixed functional selection')
    c.need(commands['tests']['argv'][1:]==list(GROUPS)+list(EXACT)+['--test-threads=1','--show-output','--color=never'],'fixed complete test command')
    raw=get(inner+'tests/stdout.log').decode()
    summary=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',raw)
    c.need(summary==[('85','0','0','0',str(len(catalog)-85))],'raw complete test summary')
    actual=[v.removesuffix(' - should panic') for v in re.findall(r'^test (.+) \.\.\. ok$',raw,re.M)]
    c.need(sorted(actual)==selected,'every selected case accounted once')
    c.need(J(get(inner+'cases.json'))==dict(selected=selected,passed=85,failed=0,ignored=0,catalog=len(catalog),filtered=len(catalog)-85),'stored accounting')
    body=semantic(get,context,raw);c.need(body==J(get(inner+'semantic-export.json')),'raw semantic export')
    return dict(platform=host,compiler=version,tests=85,catalog=len(catalog),filtered=len(catalog)-85),selected,body


def reconcile(get=local):
    frames={context:capsule(get,context) for context in (R0,R1)}
    for context,frame in frames.items():
        for name,digest in c.PINS.items():c.need(frame[name]==digest,'unchanged root/ownership pin')
        c.need(frame['Cargo.toml']=='84b1986d8a2014be985460b3ecc4a809f3ae3e8f9822ef6c7134b3ecbe9af114','unchanged Root graph')
        for label in ('prepare','format','stage'):c.need(J(get(context+'/'+label+'/result.json'))['complete'],'whole preparation phase')
        owner(J(get(context+'/format/execution.json')),120);owner(J(get(context+'/stage/execution.json')));owner(J(get(context+'/stage/native-envelope.json')))
        c.need(c.sha(get(context+'/cases-r2.json'))==c.TABLE_SHA,'exact workload table')
    for name in ('observer.rs','caller.rs','cases-r2.json'):
        c.need(get(R0+'/'+name)==get(R1+'/'+name),'same recovery candidate/observer/caller')
    maps=[]
    for context,frame in frames.items():
        prefix='target/'+context+'/fixtures/Q/'
        maps.append({n[len(prefix):]:v for n,v in frame.items() if n.startswith(prefix)})
    c.need(bool(maps[0]) and maps[0]==maps[1],'complete byte-identical Q fixture across contexts')
    recovery=J(get(R1+'/recovery-context.json'))
    c.need(recovery==dict(environment_change='confirmed Modern Standby exit',original_outcome_sha256={n:FIRST for n in ('gnu-188','windows-188')},old_inner_results_promoted=False,bounds_extended=False,only_failed_peers_dispatched=True),'separate environment recovery')
    for label in ('gnu-188','windows-188'):
        raw=get(R0+'/'+label+'/execution.json');first=J(raw)
        c.need(c.sha(raw)==FIRST and first['classification']=='timeout' and first['retired'] and not first['errors'] and J(get(R0+'/'+label+'/result.json'))['complete'] is False,'unpromoted first outer refusal')
    diagnosis=J(get(R0+'/diagnosis/standby-events.json'))
    c.need(diagnosis['diagnosis_complete'] and all(diagnosis[k] is False for k in ('settings_changed','clocks_changed','services_changed','old_outcomes_promoted')),'read-only interruption diagnosis')
    c.need(any(e['id']==506 and 'Lid' in e['message'] and '<Event ' in e['xml'] for e in diagnosis['events']) and any(e['id']==507 and 'Lid' in e['message'] for e in diagnosis['events']),'retained Windows standby/exit XML')
    post=J(get(R1+'/power-postflight/result.json'))
    c.need(post['read_only'] and not post['standby_events'] and all(post[k] is False for k in ('settings_changed','clocks_changed','services_changed')),'no recorded standby during recovery peers')
    results=[];selections=[];bodies=[]
    for context,label in PEERS:
        result,selection,body=peer(get,context,label,frames[context]);results.append(dict(context=context,label=label,**result));selections.append(selection);bodies.append(body)
    c.need(all(v==selections[0] for v in selections) and all(v==bodies[0] for v in bodies),'cross-platform/compiler functional equality')
    return dict(compatibility_gate_pass=True,peers=results,test_executions=340,unique_tests=85,semantic_rows=239,caller_rows=30,
        first_outer_timeouts_retained=2,old_inner_results_promoted=False,CPU_qualified=False,RSS_qualified=False,
        native_C_qualified=False,generic_component_gate_pass=False,root_adopted=False,release_admitted=False)


if __name__=='__main__':
    out=c.BASE/'analysis';out.mkdir(exist_ok=False);result=reconcile();c.write(out,'result.json',result)
    print(json.dumps(result))
