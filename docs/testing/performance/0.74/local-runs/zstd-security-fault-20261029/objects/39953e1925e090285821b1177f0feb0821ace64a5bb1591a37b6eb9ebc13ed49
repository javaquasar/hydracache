"""Offline review of retained replay receipts, not current runtime admission."""
import argparse
import json
from pathlib import Path
import re

import view_inputs as view

ROOT=Path(__file__).resolve().parents[2]
OUTPUT=Path(view.BASE)/'authority-history-view-20261022'


def load(path):return view.decode(Path(path).read_bytes())


def receipt(directory,m):
    directory=Path(directory);row=load(directory/'result.json')
    view.need(row['schema']=='authority-history-view-first-074-v1' and row['failure'] is None,'First replay refused')
    view.need(row['history_source']==view.SOURCE and row['manifest_sha256']==view.INPUT_SHA,'First source/input drift')
    view.need(type(row['tooling_source']) is str and re.fullmatch('[0-9a-f]{40}',row['tooling_source']) is not None,'Missing tooling commit')
    for name in ('worker.stdout.json','worker.stderr.log'):
        key='worker_stdout_sha256' if name.startswith('worker.stdout') else 'worker_stderr_sha256'
        view.need(view.sha((directory/name).read_bytes())==row[key],'First worker log digest drift')
    worker=load(directory/'worker.stdout.json')
    view.need(json.dumps(worker,sort_keys=True)==json.dumps(row['worker'],sort_keys=True),'Worker receipt differs from raw stdout')
    view.need((directory/'worker.stderr.log').read_bytes()==b'','Unexpected worker stderr')
    view.need(json.dumps(row['current_baseline'],sort_keys=True)==json.dumps(dict(current_fixture_baseline_identity=True,runtime_candidate_guard_implemented=False),sort_keys=True),'Historical success cannot become candidate admission')
    view.need(row['generated_view_removed'] is True,'Generated view not retired')
    for obj in (row,worker):
        for key in ('runtime_candidate_implemented','runtime_candidate_guard_implemented','server','product'):
            view.need(obj[key] is False,'Forbidden replay admission/scope')
        view.need(type(obj['model_images_executed']) is int and obj['model_images_executed']==0,'Model image executed')
        view.need(type(obj['original_owner_profiles']) is int and obj['original_owner_profiles']==0,'Live profile executed')
    for key in ('history_only','historical_facts_equal','old_readers_unchanged','read_only_path_scope'):
        view.need(worker[key] is True,'Historical worker contract changed')
    view.need(worker['schema']=='authority-history-worker-074-v1' and worker['history_source']==view.SOURCE,'Wrong worker')
    view.need(worker['targeted_model_gate'] is True and worker['prior_broad_isolation_gate'] is False,'Original model/refused scopes changed')
    view.need(worker['original_windows_seconds']==[300,60,10] and [r['age_ns'] for r in worker['first_refusals']]==[10042390247,10010835739],'Original windows/refusals changed')
    view.need(worker['qualification_sha256']=='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc','Qualification changed')
    view.need(worker['clocks_changed'] is False and worker['services_changed'] is False,'Clock/service action')
    view.need(type(row['archive']['bytes']) is int and 0<row['archive']['bytes']<=m['bounds']['archive_bytes'],'Archive receipt bound')
    view.need(re.fullmatch('[0-9a-f]{64}',row['archive']['sha256']) is not None,'Archive digest missing')
    return row


def analyze(root=ROOT):
    root=Path(root);m=view.manifest();packet=root/OUTPUT
    expected=view.expected_members(m,lambda name:(root/name).read_bytes())
    members=0
    for name,entry in expected.items():
        if entry[0]=='data':view.check_bytes((root/name).read_bytes(),entry);members+=1
    view.need(members==1447,'Historical sealed input census drift')
    for path in Path(__file__).parent.iterdir():
        if path.suffix not in ('.py','.ps1','.json'):continue
        relative=path.relative_to(ROOT)
        view.need(view.canonical((root/relative).read_bytes())==view.canonical((packet/'method'/relative).read_bytes()),'Replay method snapshot drift')
    row=receipt(packet/'first',m);worker=row['worker']
    fingerprint=view.sha(json.dumps({name:entry for name,entry in sorted(expected.items())},sort_keys=True).encode())
    tree=dict(files=1576,sources=129,packets=24,packet_members=1423,fingerprint=fingerprint)
    view.need(row['archive']['tree']==tree and worker['tree']==tree,'Historical tree identity drift')
    view.need(worker['expected_facts_sha256']==expected[m['expected_facts']][1],'Wrong original audit facts')
    return dict(schema='authority-history-view-review-074-v1',history_source=view.SOURCE,tooling_source=row['tooling_source'],
        manifest_sha256=view.INPUT_SHA,input_packets=m['packets'],input_members_verified=1423,tree=tree,
        historical_replay_passed=True,archived_facts_equal=True,current_fixture_baseline_at_first_run=True,
        runtime_candidate_guard_implemented=False,runtime_candidate_implemented=False,current_runtime_admitted=False,
        old_measurements_reexecuted=False,model_images_executed=0,original_owner_profiles=0,
        original_windows_seconds=worker['original_windows_seconds'],first_refusals=worker['first_refusals'],
        qualification_sha256=worker['qualification_sha256'],full_sigkill_matrix=False,reconnect_proven=False,
        clocks_changed=False,services_changed=False,server=False,product=False)


def publish(path,value,verify=False):
    view.need(Path(path).resolve()==(ROOT/OUTPUT/'facts.json').resolve(),'Separate replay output required')
    for node in (Path(path),*Path(path).parents):view.need(not node.is_symlink() and not node.is_junction(),'Linked replay output')
    if verify:view.need(json.dumps(load(path),sort_keys=True)==json.dumps(value,sort_keys=True),'Retained replay facts drift');return
    with Path(path).open('x',encoding='utf-8',newline='\n') as stream:json.dump(value,stream,indent=2,sort_keys=True);stream.write('\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--verify',action='store_true');args=parser.parse_args()
    publish(args.output,analyze(),args.verify)
    print('Retained historical replay verified; no current runtime or server admission')
