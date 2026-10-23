"""Offline checks and first-result retention for one local original-service fixture."""
import argparse
import importlib.util
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('entry_checks',ROOT/'tools/bootstrap-held-entry-074/check.py')
previous=importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
snapshot,sha,publish=previous.snapshot,previous.sha,previous.publish
OUTPUT=ROOT/'target/bootstrap-original-service-20261023'
BASE=['crates/xtask/tests/bootstrap_installed_origin_074.rs','docs/testing/performance/0.74/bootstrap-original-service-plan-20261023.md','tools/bootstrap-original-service-074/check.py']
FILES=list(dict.fromkeys(previous.FILES + BASE + [
    'docs/testing/gated-test-registry.toml',
    'crates/xtask/tests/support/original_service_gate.txt',
    'crates/xtask/tests/support/held_material_live_guard.rs',
    'crates/xtask/tests/support/bootstrap_current_image_guard.rs',
    'crates/xtask/tests/support/bootstrap_held_entry_guard.rs',
    'tools/bootstrap-original-service-074/native_fixture.py',
    'tools/bootstrap-original-service-074/native_fixture.sh',
    'tools/bootstrap-original-service-074/test_controls.py',
] + ['tools/long-run-supervisor-074/tests/support/' + name for name in (
    'consuming_held_live.rs','consuming_held_live_documents.rs',
    'held_original_service_live.rs','held_original_service_documents.rs','held_original_service_scope.rs')]))

def commands(lane):
    if lane not in tuple(kind+suffix for kind in ('host','194','188','metadata') for suffix in ('','-reviewed','-final')):
        raise ValueError('Unknown local check lane')
    kind=lane.split('-',1)[0]
    steps=previous.commands(kind)
    if kind == 'metadata':
        steps[0]=('controls',['python','-B','-m','unittest','discover','-s','tools/bootstrap-original-service-074','-v'])
    if kind == 'host':
        next(a for n,a in steps if n == 'canaries').extend(['--test','held_material_live_074','--test','held_preparation_cost_074'])
    if kind in ('194','188'):
        argv=next(a for n,a in steps if n == 'held').copy()
        argv[argv.index('held_delivery_')]='original_service_'
        steps.insert(0,('original-units',argv))
        if kind == '194':
            build=argv[:argv.index('test')]+['test','--offline','--locked','--release','-p','hydracache-long-run-supervisor-074','--all-features','--lib','--no-run','--message-format=json']
            steps.append(('native-libtest',build))
    return steps

def run(lane):
    steps=commands(lane)
    snapshot.OUTPUT,snapshot.FILES,snapshot.commands=OUTPUT,FILES,lambda _:steps
    snapshot.run(lane)
def baseline():
    if (ROOT/'tools/long-run-supervisor-074/tests/support/held_original_service_live.rs').exists():
        raise ValueError('Never rerun the historical absence baseline')
    snapshot.OUTPUT,snapshot.FILES=OUTPUT,BASE
    snapshot.commands=lambda _: [('missing',['cargo','test','--offline','--locked','-p','xtask','--test','bootstrap_installed_origin_074','bootstrap_original_service_fixture_precedes_live_admission'])]
    snapshot.run('baseline')
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane',choices=['baseline']+[kind+suffix for kind in ('host','194','188','metadata') for suffix in ('','-reviewed','-final')])
    lane=parser.parse_args().lane
    baseline() if lane == 'baseline' else run(lane)
