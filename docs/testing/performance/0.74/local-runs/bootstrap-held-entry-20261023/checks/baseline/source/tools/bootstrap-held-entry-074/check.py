"""Local eighteen-role entry controls; no service or original-manager dispatch."""
import argparse
import importlib.util
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('held_checks', ROOT / 'tools/bootstrap-held-delivery-074/check.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
snapshot, sha, publish = previous.snapshot, previous.sha, previous.publish
OUTPUT = ROOT / 'target/bootstrap-held-entry-20261023'
LANES = ('baseline', 'host', '194', '188', 'metadata', 'host-final', '194-final', '188-final', 'metadata-final')
BASE_FILES = ['crates/xtask/tests/bootstrap_installed_origin_074.rs', 'Cargo.lock',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'docs/testing/performance/0.74/bootstrap-held-entry-plan-20261023.md', 'tools/bootstrap-held-entry-074/check.py']
FILES = list(dict.fromkeys(previous.FILES + BASE_FILES + ['tools/long-run-supervisor-074/src/' + n for n in (
    'diagnostic_held_activation.rs', 'diagnostic_bootstrap_held_mode.rs', 'diagnostic_bootstrap_held_transport.rs',
    'diagnostic_bootstrap_held_child.rs', 'diagnostic_bootstrap_held_inventory.rs')]
    + ['tools/long-run-supervisor-074/tests/support/bootstrap_held_entry_controls.rs',
       'crates/xtask/tests/support/bootstrap_held_entry_guard.rs']))

def commands(lane):
    if lane not in LANES: raise ValueError('Unknown lane')
    if lane == 'baseline':
        return [('missing', ['cargo','test','--offline','--locked','-p','xtask','--test',
            'bootstrap_installed_origin_074','bootstrap_held_entry_component_precedes_activation'])]
    kind = lane.split('-', 1)[0]
    steps = previous.commands(kind)
    if kind in ('194', '188'):
        argv = next(a for n, a in steps if n == 'child-units').copy()
        argv[argv.index('diagnostic_held_bootstrap')+1:argv.index('--')] = ['held_entry_']
        steps.insert(0, ('held-entry', argv))
    if kind == 'metadata':
        steps[0] = ('controls', ['python','-B','-m','unittest','discover','-s','tools/bootstrap-held-entry-074','-v'])
    return steps

def run(lane):
    if lane == 'baseline' and (ROOT/'tools/long-run-supervisor-074/src/diagnostic_bootstrap_held_mode.rs').exists():
        raise ValueError('Historical baseline cannot be rerun')
    steps = commands(lane)
    snapshot.OUTPUT, snapshot.FILES, snapshot.commands = OUTPUT, BASE_FILES if lane == 'baseline' else FILES, lambda actual: steps if actual == lane else []
    snapshot.run(lane)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=LANES)
    run(parser.parse_args().lane)
