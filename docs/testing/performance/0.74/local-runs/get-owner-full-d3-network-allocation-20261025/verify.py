"""Offline raw-byte, wire, source, host-retirement and fixed-estimator replay."""
from pathlib import Path
import hashlib
import importlib.util
import json
import re
import subprocess
import sys
import tomllib
import types

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[5]
PREFIX=HERE.relative_to(ROOT).as_posix()+'/'


def require(value,message):
    if not value:raise ValueError(message)


def decode(raw):
    def pairs(items):
        result={}
        for key,value in items:
            require(key not in result,'duplicate JSON field')
            result[key]=value
        return result
    return json.loads(raw,object_pairs_hook=pairs)


def module(name):
    spec=importlib.util.spec_from_file_location(name,ROOT/'tools/get-owner-scheduled-controls-074'/ (name+'.py'))
    result=importlib.util.module_from_spec(spec)
    previous=sys.modules.get('fcntl')
    if sys.platform=='win32':sys.modules['fcntl']=types.ModuleType('fcntl')
    try:spec.loader.exec_module(result)
    finally:
        if sys.platform=='win32':
            if previous is None:sys.modules.pop('fcntl')
            else:sys.modules['fcntl']=previous
    return result


def pinned_module(name,raw):
    result=types.ModuleType(name)
    result.__file__='<indexed-'+name+'>'
    previous=sys.modules.get('fcntl')
    if sys.platform=='win32':sys.modules['fcntl']=types.ModuleType('fcntl')
    try:exec(compile(raw,result.__file__,'exec'),result.__dict__)
    finally:
        if sys.platform=='win32':
            if previous is None:sys.modules.pop('fcntl')
            else:sys.modules['fcntl']=previous
    return result


def verify(read):
    index=decode(read('index.json'))
    require(index['schema_version']==1,'index schema')
    for name in ['candidate_admitted','timings_or_retention_proven','thresholds_changed']:
        require(index[name] is False,'unearned admission: '+name)
    require(index['full_d3_terminal_rejection'] is True,'terminal result lost')
    for name,item in index['members'].items():
        require(not name.startswith('/') and '..' not in name.split('/'),'unsafe member')
        require(re.fullmatch('[0-9a-f]{64}',item['sha256']) is not None,'digest shape')
        require(type(item['bytes']) is int and 0<=item['bytes']<=16*1024**2,'member byte bound')
        raw=read('objects/'+item['sha256'])
        require(len(raw)==item['bytes'] and hashlib.sha256(raw).hexdigest()==item['sha256'],'raw digest: '+name)
        require(not name.endswith(('/material.json','/private-material.json')),'private key file retained')
    def member(name):return read('objects/'+index['members'][name]['sha256'])
    def doc(name):return decode(member(name))
    plan=doc('stage/plan.json')
    seal=doc('stage/seal.json')
    require(hashlib.sha256(member('stage/plan.json')).hexdigest()==seal['plan_sha256'],'plan seal')
    policy=tomllib.loads(member('contracts/get-owner-full-d3-allocation-contract.toml').decode())
    v2=tomllib.loads(member('contracts/get-owner-full-d3-allocation-v2-contract.toml').decode())
    require(policy['matrix']['maximum_affected_gross_allocation_ratio']==.8 and policy['matrix']['maximum_native_gross_allocation_ratio']==1.05,'cost floors changed')
    require(policy['matrix']['maximum_aa_gross_allocation_relative_noise']==.01,'AA floor changed')
    require(v2['total_attempts']==240 and v2['resp']['physical_connections']==1 and v2['resp']['execution_slots']==128,'placement contract changed')
    cells=[cell['id'] for cell in policy['cell']]
    require(len(cells)==12 and list(dict.fromkeys(a['cell'] for a in plan['attempts']))==cells,'cell registry order')
    for name in ['cohort-complete.json','all-writers-stopped.json','restored.json']:
        require('stage/'+name in index['members'],'missing terminal '+name)
    final=doc('stage/host-final.snapshot.json')
    restored=doc('stage/restored.json')
    require(final['supervisor']==restored['supervisor'] and final['boot']==plan['boot'],'restored generation')
    require(final['installed_and_markers_verified'] is True and final['supervisor']['ActiveState']=='active' and final['supervisor']['SubState']=='running' and final['supervisor']['NRestarts']=='0','host postflight')
    require(doc('stage/all-writers-stopped.json')['original_groups_empty'] is True,'original writers still live')
    for sequence,a in enumerate(plan['attempts']):
        require(a['receipt_pins']['source_commit_from_coordinator']==seal['source'],'source identity drift')
        intent=doc('stage/'+str(sequence)+'.intent.json')
        require(intent['attempt']==a and intent['boot']==plan['boot'],'original attempt drift')
        require(intent['group'].endswith('/'+plan['unit']+'/writers-'+str(sequence)),'owned placement drift')
        require(doc('stage/'+str(sequence)+'.retired.json')['events']['populated']=='0','populated writer')
    build='server/get-owner-allocation-v2-build-first/'
    require(doc(build+'result.json')['passed'] is True,'GNU build/test refusal')
    for variant in ['off','on']:
        image=doc(build+'image-'+variant+'.json')
        require(image['source_commit']==seal['source'] and image['sha256']==seal['images'][variant]['sha256'],'image/source receipt')
        log=member(build+variant+'-wire-semantic.stdout.log').decode()
        require(log.count('observed=Some(50) error=None shutdown=true')==4 and log.count('observed=Some(10) error=None shutdown=true')==4,'first GNU wire control')
    first='windows/get-owner-v2-semantic-compiled-first/'
    require(doc(first+'result.json')['exit']==101,'first Windows semantic refusal lost')
    require(b'error=Some("allocation observation' in member(first+'stdout.log'),'first refusal changed')
    fixtures=doc('server/get-owner-maintenance-fixtures-final/result.json')
    require(fixtures['passed'] is True and [c['mode'] for c in fixtures['cases']]==['healthy','deadline','owner-kill'],'coordinator fault gate')
    coordinator=hashlib.sha256(member('stage/coordinator.py')).hexdigest()
    require(plan['inputs'][next(p for p in plan['inputs'] if p.endswith('/coordinator.py'))]==coordinator,'coordinator seal')
    for case in fixtures['cases']:
        prefix='server/'+case['root'].rsplit('/',1)[1]+'/'
        require(hashlib.sha256(member(prefix+'coordinator.py')).hexdigest()==coordinator,'fault source differs')
        require(doc(prefix+'restored.json')['verified'] is True and doc(prefix+'all-writers-stopped.json')['original_fence_acquired'] is True,'fault retirement/restoration')
    gate=pinned_module('allocation_gate',member('analysis/allocation_gate.py'))
    maintenance=pinned_module('maintenance_d3',member('stage/coordinator.py'))
    result=gate.analyse(lambda name:member('stage/'+name),maintenance.validate_receipt)
    require(result==doc('analysis-first.json'),'first fixed-estimator result changed')
    require(result['classification']==index['classification']=='rejected-mandatory-full-d3-allocation-floor','classification drift')
    require(result['rejected_cells']==[c['id'] for c in policy['cell'] if c['affected'] and '-mtls-' in c['id']],'terminal cells drift')
    return dict(raw_members=len(index['members']),attempts=240,terminal_rejection=True,
                rejected_cells=result['rejected_cells'],supervisor=final['supervisor'],
                timing_or_retention_proven=False,candidate_admitted=False)


if __name__=='__main__':
    if sys.argv[1:]==['--staged']:
        reader=lambda name:subprocess.check_output(['git','cat-file','blob',':'+PREFIX+name])
    else:
        require(not sys.argv[1:],'expected optional --staged')
        reader=lambda name:(HERE/name).read_bytes()
    print(json.dumps(verify(reader),indent=2))
