"""One fixed Windows package command after v6 review; preflight by default."""
import argparse
import json
from pathlib import Path
import re
import runpy
import subprocess
import sys

import package_effects_review_v6 as review
from package_replay_v2 import ARGV, checked_wrappers

ROOT = Path(__file__).resolve().parents[2]
SYNTAX = 'docs/testing/performance/0.74/local-runs/package-review-v6-20261026/objects/e8c6a3141d7381fb024e712e0287a980bc0e9d062597bac437558dbce3bd6996'


def load_document(root):
    path = root / SYNTAX
    raw = path.read_bytes()
    review.require(review.sha(raw) == path.name, 'fixed syntax artifact drift')
    return json.loads(raw)


def run(label, *, execute=False, root=ROOT):
    review.require(re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', label) is not None, 'bounded label')
    out = root / 'target/reviewed-package-v6-20261026' / label
    out.mkdir(parents=True, exist_ok=False)

    def write(name, value):
        with (out / name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, sort_keys=True, indent=2)
            stream.write('\n')

    write('intent.json', dict(argv=ARGV, execute=execute, package_bound_seconds=780, case_threads=4,
                             retry=False, observer=False, services=False, clocks=False,
                             server=False, qualification=False))
    try:
        gate = review.source_review(root, load_document(root))
        gate['environment'] = review.environment_preflight(root)
        wrappers = checked_wrappers(root)
        gate['head'] = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True, timeout=10).strip()
        dirty = subprocess.check_output(['git', 'status', '--porcelain', '--untracked-files=all', '--',
                                        'Cargo.toml', 'Cargo.lock', 'deny.toml', '.cargo', 'crates', 'tools',
                                        'scripts/ci', '.gitattributes'], cwd=root, timeout=10)
        review.require(not dirty, 'source worktree must be clean including untracked inputs')
        write('gate.json', gate)
        write('coordinator.json', dict(wrappers=wrappers, preflight_only=not execute,
                                      package_result_not_assessed=True))
    except BaseException as error:
        write('preflight-refusal.json', dict(type=type(error).__name__, reason=str(error),
                                           package_dispatched=False))
        raise
    if not execute:
        return dict(preflight_complete=True, package_dispatched=False, package_pass=False)
    runner = root / 'target/run-xtask-package-780-20261024.py'
    write('dispatch.json', dict(label='package-v6-' + label, argv=ARGV, automatic_retry=False))
    original_argv = sys.argv
    try:
        sys.argv = [str(runner), 'package-v6-' + label, *ARGV]
        runpy.run_path(str(runner), run_name='__main__')
    finally:
        sys.argv = original_argv
    return dict(package_dispatched=True, package_result_not_assessed=True, package_pass=False)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('label')
    parser.add_argument('--execute', action='store_true')
    args = parser.parse_args()
    print(json.dumps(run(args.label, execute=args.execute), sort_keys=True))
