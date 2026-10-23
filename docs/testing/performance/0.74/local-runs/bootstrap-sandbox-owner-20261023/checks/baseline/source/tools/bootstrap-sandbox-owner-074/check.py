"""Local original-owner sandbox observer checks; no service or original delivery."""
import argparse
import importlib.util
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('installed_child_checks', ROOT / 'tools/bootstrap-installed-child-074/check.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
snapshot = previous.snapshot
logger, sha, publish = previous.logger, previous.sha, previous.publish
OUTPUT = ROOT / 'target/bootstrap-sandbox-owner-20261023'
LANES = ('baseline', 'host', '194', '188', 'metadata')
FILES = ['crates/xtask/tests/bootstrap_installed_origin_074.rs',
    'docs/testing/performance/0.74/bootstrap-sandbox-owner-plan-20261023.md',
    'tools/bootstrap-sandbox-owner-074/check.py']
def commands(lane):
    if lane == 'baseline':
        return [('missing', ['cargo', 'test', '--offline', '--locked', '-p', 'xtask', '--test',
            'bootstrap_installed_origin_074', 'bootstrap_sandbox_owner_component_precedes_admission'])]
    raise ValueError('Unknown lane')
def run(lane):
    steps = commands(lane)
    snapshot.OUTPUT, snapshot.FILES, snapshot.commands = OUTPUT, FILES, lambda actual: steps if actual == lane else []
    snapshot.run(lane)
if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=LANES)
    run(parser.parse_args().lane)
