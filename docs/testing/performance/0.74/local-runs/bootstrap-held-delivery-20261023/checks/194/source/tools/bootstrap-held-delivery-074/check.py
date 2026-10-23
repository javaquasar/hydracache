"""Bounded local held delivery components; never call a manager or change services."""
import argparse
import importlib.util
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('sandbox_checks', ROOT / 'tools/bootstrap-sandbox-owner-074/check.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
snapshot = previous.snapshot
sha, publish = previous.sha, previous.publish
OUTPUT = ROOT / 'target/bootstrap-held-delivery-20261023'
LANES = ('host', '194', '188', 'metadata', 'host-final', '194-final', '188-final', 'metadata-final')
FILES = list(dict.fromkeys(previous.FILES + [
    'docs/testing/performance/0.74/bootstrap-held-delivery-plan-20261023.md',
    'crates/xtask/tests/support/bootstrap_held_delivery_guard.rs',
] + ['tools/long-run-supervisor-074/src/' + name for name in (
    'diagnostic_held_child_exchange.rs', 'diagnostic_held_child_exchange_tests.rs',
    'diagnostic_held_delivery_wire.rs', 'diagnostic_held_delivery_wire_tests.rs',
    'diagnostic_manager_held_delivery.rs', 'diagnostic_original_held_delivery.rs')]
    + ['tools/bootstrap-held-delivery-074/' + name for name in ('check.py', 'test_controls.py')]))

def commands(lane):
    if lane not in LANES: raise ValueError('Unknown lane')
    kind = lane.split('-', 1)[0]
    steps = previous.commands(kind)
    if kind in ('194', '188'):
        argv = next(argv for name, argv in steps if name == 'sandbox').copy()
        argv[argv.index('bootstrap_sandbox_')] = 'held_delivery_'
        steps.insert(0, ('held', argv))
    if kind == 'metadata':
        steps[0] = ('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/bootstrap-held-delivery-074', '-v'])
    return steps

def run(lane):
    steps = commands(lane)
    snapshot.OUTPUT, snapshot.FILES, snapshot.commands = OUTPUT, FILES, lambda actual: steps if actual == lane else []
    snapshot.run(lane)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=LANES)
    run(parser.parse_args().lane)
