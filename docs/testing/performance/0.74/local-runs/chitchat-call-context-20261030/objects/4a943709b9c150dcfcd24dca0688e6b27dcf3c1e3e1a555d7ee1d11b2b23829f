"""Reconcile every additional peer and exact semantic corpus, data only."""
import json
import re
from pathlib import Path
import review as r

PURE=('serialize::tests::','delta::tests::','message::tests::','digest::tests::','state::tests::','security_review::',
 'serialize::borrowed_context_control::buffer_error','serialize::borrowed_context_control::independent_writers')
OBS=('serialize::security_stream_review::observe_streams','delta::security_delta_review::supported_delta_operations')
MARKERS=('HC074_STREAM_BOUNDS:','HC074_SUPPORTED_DELTA:','HC074_SECURITY_MALFORMED:')
LEGACY='serialize::borrowed_context_control::retains_context_for_its_writer'
CALL_CASES=('disabled_hook_has_no_effect','context_is_absent_at_append_boundary','panic_after_context_creation_releases_only_context',
 'serialization_panic_keeps_original_prefix','oversized_item_is_refused_before_serialization',
 'zero_length_generic_item_keeps_original_behavior','separate_writers_and_threads_keep_their_own_frames')
NEGATIVES=('serialize::security_stream_review::minimal_bound_contract','serialize::security_stream_review::all_stream_bounds_hold')


def selected(names,v):return [n for n in names if n.startswith(PURE) or n in OBS or v=='C2' and n.startswith('serialize::call_context_review::')]


def account(raw,names,tests,negative=False):
    text=raw.decode().replace('\r\n','\n')
    headers=re.findall(r'^test (.+?) (?:- should panic )?\.\.\. ',text,re.M)
    r.need(sorted(headers)==sorted(tests),'exact complete selected identities')
    summary=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',text,re.M)
    r.need(summary==[('FAILED' if negative else 'ok','0' if negative else str(len(tests)),'1' if negative else '0','0','0',str(len(names)-len(tests)))],'complete native outcome counts')


def owner(read,base,expected=0,bound=None):
    result=json.loads(read(base+'execution.json'));intent=json.loads(read(base+'intent.json'))
    r.need(result['exit']==expected and result['retired'] and not result['errors'] and result['cleanup_seconds']==result['capacity_poll_seconds']==5,'complete original owner: '+base)
    if bound is not None:r.need(result['bound_seconds']==intent['bound']==bound,'original bound: '+base)
    return intent


def matrix(read):
    load=lambda n:json.loads(read(n));corpora=[];peers={}
    parent=load('parents/c1-index.json');raw=read('parents/c1-catalog.json');row=parent['members']['catalog-B0/result.json']
    r.need(r.sha(raw)==row['sha256'] and len(raw)==row['bytes'],'original whole library catalog')
    expected_names=sorted(json.loads(raw)['catalog']+['serialize::call_context_review::'+n for n in CALL_CASES])
    for compiler in ('194','188'):
        for v in ('B0','C2'):
            suffix='-'+v+('-188' if compiler=='188' else '')
            names=load('catalog'+suffix+'/result.json')['catalog'];tests=selected(names,v)
            r.need(names==expected_names and len(names)==len(set(names))==116 and len(tests)==(63 if v=='B0' else 70),'complete Windows catalog/selection')
            listed=read('catalog'+suffix+'/stdout.log').decode();r.need(sorted(s[:-6] for s in listed.splitlines() if s.endswith(': test'))==names,'raw Windows catalog identities')
            for action in ('build','catalog','control','compat','check','clippy'):
                base=action+suffix+'/';r.need(load(base+'result.json')['complete'],'complete Windows peer part')
                owner(read,base,bound=10 if action=='catalog' else 5 if action=='control' else 1200)
            account(read('compat'+suffix+'/stdout.log'),names,tests)
            text=read('compat'+suffix+'/stdout.log').decode()
            corpora.append({m:json.loads(text.split(m)[1].splitlines()[0]) for m in MARKERS if text.count(m)==1})
            r.need(set(corpora[-1])==set(MARKERS),'all original diagnostic corpora')
            if v=='C2':
                for action,negative,test in [('panic',False,'serialize::call_context_review::panic_after_context_creation_releases_only_context'),('legacy',True,LEGACY)]:
                    base=action+suffix+'/';r.need(load(base+'result.json')['complete'],'complete extra control');owner(read,base,101 if negative else 0,5)
                    account(read(base+'stdout.log'),names,[test],negative)
            peers['windows-'+v+'-'+compiler]=dict(passed=len(tests),catalog=116)
    for compiler in ('194','188'):
        for v in ('B0','C2'):
            peer='gnu-'+v+'-'+compiler;base='semantic/'+peer+'/'
            r.need(load(base+'result.json')['complete'] and load(base+'session-result.json')['complete'],'complete GNU whole peer')
            owner(read,base,bound=1200)
            catalog=load(base+'catalog.json');names=catalog['all'];tests=selected(names,v)
            extra=['serialize::native_lifetime::allocator_controls','serialize::native_lifetime::context_lifetime_observation'] if compiler=='194' else []
            r.need(names==sorted(expected_names+extra) and catalog['negative']==list(NEGATIVES)+([LEGACY] if v=='C2' else []),'original mandatory and negative case identities')
            listed=read(base+'catalog/stdout.log').decode();r.need(sorted(s[:-6] for s in listed.splitlines() if s.endswith(': test'))==names,'raw GNU catalog identities')
            r.need(len(names)==len(set(names))==(118 if compiler=='194' else 116) and tests==catalog['selected'] and len(tests)==(63 if v=='B0' else 70),'exact GNU catalog/selection')
            r.need(catalog['full_library_passed'] is False,'no full-library promotion')
            image=load(base+'image.json');tables={}
            for i,test in enumerate(tests+catalog['negative']):
                negative=test in catalog['negative'];folder=base+f'test-{i:03}/'
                intent=owner(read,folder,101 if negative else 0,5)
                r.need(intent['argv']==[image['path'],'--exact',test,'--test-threads=1','--nocapture'] and intent['image_sha256']==image['sha256'],'exact observed semantic image')
                raw=read(folder+'stdout.log');account(raw,names,[test],negative)
                if not negative:
                    text=raw.decode()
                    for marker in MARKERS:
                        if marker in text:
                            r.need(text.count(marker)==1 and marker not in tables,'one complete diagnostic');tables[marker]=json.loads(text.split(marker)[1].splitlines()[0])
            r.need(set(tables)==set(MARKERS),'all required GNU corpora');corpora.append(tables)
            if compiler=='188':
                for action in ('build','check','clippy'):owner(read,base+action+'/',bound=1200)
            peers[peer]=dict(passed=len(tests),catalog=len(names),known_negatives=len(catalog['negative']))
    r.need(all(table==corpora[0] for table in corpora),'complete same workload/decision/wire/cursor bodies across eight peers')
    return dict(peers=peers,positive_case_executions=sum(p['passed'] for p in peers.values()),same_full_corpora=True,
        full_library_passed=False,mtu_gate_passed=False,root_adopted=False,release_admitted=False)


def full(read):
    result=r.review(read);result['compatibility']=matrix(read);return result


def main():
    out=r.BASE/'final-analysis';out.mkdir(exist_ok=False)
    (out/'producer.py').write_bytes(Path(__file__).read_bytes())
    result=full(r.live_read)
    (out/'result.json').write_text(json.dumps(result,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    print(json.dumps({k:v for k,v in result.items() if k!='observations'}))


if __name__=='__main__':main()
