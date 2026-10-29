"""Preflight by default. --execute is the sole, fixed Windows package dispatch.

No timeout/threads/target/filter argument or automatic retry is accepted.
Original 780-second wrapper and base wrapper must match the retained bytes.
"""
import argparse
import json
from pathlib import Path
import re
import runpy
import subprocess
import sys

import package_effects_review_v2 as review

ROOT = Path(__file__).resolve().parents[2]
ARGV = ['cargo', '+1.94.0', 'test', '--offline', '--locked', '--no-fail-fast',
        '-p', 'xtask', '--lib', '--tests', '--', '--test-threads=4']
SYNTAX = 'docs/testing/performance/0.74/local-runs/package-effect-syntax-v2-20261025/objects/07d9960365b4976ba7f0a618799b222fad89c3d6e5dffa9a29436186c802e94d'
WRAPPERS = {
    'target/run-xtask-package-780-20261024.py':'52504f2b489d16f344b477df0568f33ac224c5cadacc552f32f427f39c09d04d',
    'target/run-source-step-20261024.py':'a92482cabcfcf8f6440e95972bee8d204bb9c59c8805c45c7b57363dcc3ab430',
}


def load_document(root):
    path = root/SYNTAX
    raw = path.read_bytes()
    review.require(review.sha(raw) == path.name, 'fixed syntax artifact drift')
    return json.loads(raw)


def checked_wrappers(root):
    result = {path:review.sha((root/path).read_bytes()) for path in WRAPPERS}
    review.require(result == WRAPPERS, 'original package wrapper drift')
    return result


def run(label, *, execute=False, root=ROOT):
    review.require(re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', label) is not None, 'bounded label')
    out = root/'target/reviewed-package-v2-20261025'/label
    out.mkdir(parents=True, exist_ok=False)

    def write(name, value):
        with (out/name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, sort_keys=True, indent=2)
            stream.write('\n')

    write('intent.json', dict(argv=ARGV, execute=execute, package_bound_seconds=780,
        case_threads=4, retry=False, server=False, services=False, clocks=False, qualification=False))
    try:
        gate = review.source_review(root, load_document(root))
        gate['environment'] = review.environment_preflight(root)
        wrappers = checked_wrappers(root)
        gate['head'] = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True, timeout=10).strip()
        dirty = subprocess.check_output(['git', 'status', '--porcelain', '--untracked-files=all', '--',
            'Cargo.toml', 'Cargo.lock', 'deny.toml', '.cargo', 'crates', 'tools'], cwd=root, timeout=10)
        review.require(not dirty, 'source worktree must be clean including untracked inputs')
        write('gate.json', gate)
        write('coordinator.json', dict(wrappers=wrappers, package_result_not_assessed=True,
                                       preflight_only=not execute))
    except BaseException as error:
        write('preflight-refusal.json', dict(type=type(error).__name__, reason=str(error), package_dispatched=False))
        raise
    if not execute:
        return dict(preflight_complete=True, package_dispatched=False, package_pass=False)
    runner = root/'target/run-xtask-package-780-20261024.py'
    write('dispatch.json', dict(label='package-replay-v2-'+label, argv=ARGV, automatic_retry=False))
    original_argv = sys.argv
    try:
        sys.argv = [str(runner), 'package-replay-v2-'+label, *ARGV]
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
