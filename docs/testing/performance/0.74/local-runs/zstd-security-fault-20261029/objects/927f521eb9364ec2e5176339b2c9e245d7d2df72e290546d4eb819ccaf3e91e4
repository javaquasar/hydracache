"""One newly authorized bounded capture, never a replay of consumed lanes."""
import argparse
import subprocess
import traceback
from pathlib import Path

import analyze
import registration
import run as base

OUT = base.ROOT / 'target/wsl-clock-setter-20261023'
ARCHIVE = base.ROOT / 'docs/testing/performance/0.74/local-runs/wsl-clock-setter-20261023'


def configure_archive():
    base.OUT, base.ARCHIVE = OUT, ARCHIVE
    base.PLAN = base.ROOT / 'docs/testing/performance/0.74/wsl-clock-setter-plan-20261023.md'
    base.FROZEN = base.FROZEN | registration.PINS | {
        'local-runs/wsl-clock-registration-20261023/index.json':
            '39a658169264764197f24cf849309373d7002c2c9d9520e6eedc3b8ef767c501'}


def collect():
    configure_archive()
    if subprocess.check_output(['git', 'status', '--porcelain'], cwd=base.ROOT).strip():
        raise ValueError('Commit the reviewed fresh capture lane first')
    proof = base.sources()
    OUT.mkdir(parents=True, exist_ok=False)
    base.put(OUT / 'intent.json', {'sources': proof, 'frozen_inputs': base.FROZEN,
        'git': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=base.ROOT, text=True).strip(),
        'authorization': 'user continued directly after proposal of one bounded 90-second WSL capture',
        'capture_seconds_raw': 90, 'guardian_seconds_raw': 120, 'outer_timeout_seconds': 180,
        'original_windows': [300, 60, 10], 'maximum_queries': 200, 'retry': False,
        'clock_change': False, 'service_change': False, 'original_owner_run': False,
        'server': False, 'product': False, 'admission': False})
    base.command('controls', ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec',
        'python3', '-B', '-m', 'unittest', 'discover', '-s', base.linux(base.HERE), '-v'], 45)
    try:
        base.command('session', ['wsl.exe', '-d', 'Ubuntu', '-u', 'root', '--exec',
            'python3', '-B', base.linux(Path(__file__)), 'session'], 180)
    finally:
        if proof != base.sources():
            raise ValueError('Source or frozen-input drift')
    try:
        result = analyze.analyze(OUT)
    except BaseException as error:
        base.put(OUT / 'first-analysis-refusal.json', {'error': repr(error),
                 'traceback': traceback.format_exc(), 'new_live_dispatch_allowed': False})
        raise
    base.put(OUT / 'analysis.json', result)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('collect', 'session', 'retain', 'verify'))
    mode = parser.parse_args().mode
    if mode == 'session':
        import capture
        capture.OUT = OUT
        capture.collect()  # Unchanged checks, probe plan, read-only helper and deadlines.
    elif mode == 'collect':
        collect()
    else:
        configure_archive()
        getattr(base, mode)()
