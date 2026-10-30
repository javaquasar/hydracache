"""Data-only evidence; environment assertions are exported without inherited secrets."""
import json
import os
from pathlib import Path
import subprocess
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'supervisor-zstd-security-074'))
import common as c
import graph

OUT=c.ROOT/'docs/testing/performance/0.74/local-runs/supervisor-zstd-security-20261030'
SAFE_ENV={'RUSTC','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_NET_OFFLINE','CARGO_INCREMENTAL','CARGO_BUILD_JOBS','RUSTUP_TOOLCHAIN','CARGO_HOME'}

def observation(name):
    raw=(c.BASE/name/'stdout.log').read_text(encoding='utf8')
    marker='HC074_SUPERVISOR_ZSTD:'
    c.need(raw.count(marker)==1,'one complete diagnostic wire record')
    value=json.loads(raw.split(marker,1)[1].splitlines()[0])
    c.need(value['seed']==7606312 and [v['size'] for v in value['observations']]==[0,1,4095,16384,131072,262144] and value['performance_claim'] is False,'six fixed test-only graph observations')
    return value

def analysis():
    old=(c.BASE/'baseline/Cargo.lock').read_bytes();review=graph.review(old,(c.ROOT/'Cargo.lock').read_bytes())
    windows=['check-194','clippy-194','check-188','clippy-188','format-package-194',*[f'candidate-archive-{i}-{v}' for v in ['194','188'] for i in range(4)],'candidate2-probe-0-194','candidate-probe-0-188']
    native=['metadata-r4-194','format-194','check-194','clippy-194','check-188','clippy-188',*[f'candidate-archive-{i}-{v}' for v in ['194','188'] for i in range(4)]]
    for name in windows:c.need(json.loads((c.BASE/name/'result.json').read_bytes())['complete'],'complete first corrected Windows phase: '+name)
    for name in native:c.need(json.loads((c.BASE/'gnu'/name/'result.json').read_bytes())['complete'],'complete first corrected GNU phase: '+name)
    observations=[observation(n) for n in ('baseline-probe-0-194','candidate2-probe-0-194','candidate-probe-0-188')]
    c.need(observations[0]==observations[1]==observations[2],'identical six observations within separate unchanged fixture graph')
    findings=[]
    for scope,expected in [('supervisor',[]),('root',['RUSTSEC-2026-0391','RUSTSEC-2026-0398','RUSTSEC-2026-0403'])]:
        result=json.loads((c.BASE/'policy'/scope/'result.json').read_bytes())
        c.need(result['complete'] and result['policy_passed']==(not expected),'exact fresh scope verdict')
        errors=[]
        for line in (c.BASE/'policy'/scope/'stderr.log').read_text(encoding='utf8').splitlines():
            row=json.loads(line);fields=row.get('fields',{})
            if fields.get('severity')=='error':errors.append(fields)
        ids=sorted(f['advisory']['id'] for f in errors)
        c.need(ids==expected,'only the expected exact advisory IDs')
        if scope=='root':
            # Each remaining reverse graph starts at Chitchat, never supervisor.
            for fields in errors:
                text=json.dumps(fields['graphs'])
                c.need('chitchat' in text and 'hydracache-long-run-supervisor-074' not in text,'remaining old wrapper is owned only by Chitchat')
            findings=ids
    publication=json.loads((c.BASE/'publication/result.json').read_bytes())
    c.need(publication['complete'] and len(publication['publishable_roots'])==23 and publication['reachable_nodes']==599 and not publication['new_private_nodes_reached'],'actual complete public-root closure')
    return dict(**review,windows_archive_cases=8,gnu_archive_cases=8,baseline_archive_cases=4,
      publishable_roots=23,public_reachable_nodes=599,changed_private_edge_outside_public_closure=True,
      default_root_graph=True,paired_windows_gnu_msrv_checks=True,strict_clippy=True,
      windows_full_formatter_passed=False,full_gnu_formatter_passed=True,targeted_windows_formatter_passed=True,
      standalone_wire_payloads=6,standalone_wire_equal=True,standalone_wire_is_exact_root_graph=False,
      standalone_graph_scope='Separate test-only lock includes newer helper packages and zstd-safe7.3; not an exact Root wire claim.',
      supervisor_advisories_passed=True,root_remaining_advisories=findings,workspace_gate_passed=False,
      q_adopted=False,q_cpu_gate_passed=False,release_admitted=False,server=False)

def main():
    result=analysis();OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir();members={};sanitized={}
    def add(name,raw):
        c.need(name not in members,'unique evidence name');digest=c.sha(raw);p=OUT/'objects'/digest
        if p.exists():c.need(p.read_bytes()==raw,'identical content object')
        else:p.write_bytes(raw)
        members[name]=dict(sha256=digest,bytes=len(raw))
    for folder,dirs,names in os.walk(c.BASE,followlinks=False):
        folder=Path(folder)
        dirs[:]=[n for n in dirs if n not in ('__pycache__','.git','tmp')]
        for n in dirs:c.need(not (folder/n).is_symlink() and not getattr(folder/n,'is_junction',lambda:False)(),'regular evidence directory')
        for n in sorted(names):
            p=folder/n;c.need(not p.is_symlink() and p.is_file(),'regular capture')
            name='capture/'+p.relative_to(c.BASE).as_posix();raw=p.read_bytes()
            if n=='intent.json':
                value=json.loads(raw)
                if 'environment' in value:
                    original=c.sha(raw);value['environment']={k:v for k,v in value['environment'].items() if k in SAFE_ENV}
                    value['inherited_environment_not_exported']=True
                    raw=(json.dumps(value,indent=2,sort_keys=True)+'\n').encode();sanitized[name]=dict(original_private_sha256=original,exported_sha256=c.sha(raw))
            add(name,raw)
    for tool in ('supervisor-zstd-security-074','supervisor-zstd-policy-074'):
        for p in sorted((c.ROOT/'tools'/tool).iterdir()):
            if p.is_file():add('source/tools/'+tool+'/'+p.name,p.read_bytes())
    for n in ('Cargo.lock',c.PKG+'/Cargo.toml','deny.toml','docs/testing/performance/0.74/qualification-manifest.toml'):add('candidate/'+n,(c.ROOT/n).read_bytes())
    add('analysis.json',(json.dumps(result,indent=2,sort_keys=True)+'\n').encode())
    index=dict(schema='hc074-direct-supervisor-zstd-v1',members=members,result=result,sanitized_intent_exports=sanitized,
      archived_code_executed=False,compiled_images_excluded=True,native_cache_copies_excluded=True,
      source_head_observation=c.git('rev-parse','HEAD').decode().strip(),release_admitted=False)
    c.write(OUT,'index.json',index)
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for r in members.values():
        raw=(OUT/'objects'/r['sha256']).read_bytes();c.need(c.sha(raw)==r['sha256'] and len(raw)==r['bytes'],'fresh stored bytes')
    print(json.dumps(dict(index_sha256=c.sha((OUT/'index.json').read_bytes()),members=len(members),objects=len(list((OUT/'objects').iterdir())))))

if __name__=='__main__':main()
