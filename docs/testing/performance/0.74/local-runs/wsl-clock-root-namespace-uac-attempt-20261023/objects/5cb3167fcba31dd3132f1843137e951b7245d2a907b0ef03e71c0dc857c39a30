"""Fresh authorized registry configuration lane; reuse unchanged session guards."""
import argparse
import json
from pathlib import Path
import subprocess

import einval
import run as base

OUT = base.ROOT / 'target/wsl-clock-registration-20261023'
ARCHIVE = base.ROOT / 'docs/testing/performance/0.74/local-runs/wsl-clock-registration-20261023'
PINS = {
    'local-runs/wsl-clock-trace-20261023/index.json': '89912e476413847377bed1f22d1e52c799d2ded988fd3f861ecfedc56960da03',
    'local-runs/wsl-clock-einval-20261023/index.json': '99c84217cbe5d99e3aef1899e94178bc20bf03c5728dabf954bf135ad5c7f758',
    'local-runs/wsl-clock-writer-20261023/index.json': 'de758b208afe099e48da3f2fc176382a25b26fa9887c46b9a4ed4daa4aebec07',
}


def configure_archive():
    base.OUT, base.ARCHIVE = OUT, ARCHIVE
    base.PLAN = base.ROOT / 'docs/testing/performance/0.74/wsl-clock-registration-plan-20261023.md'
    base.FROZEN = base.FROZEN | PINS


def outcome(folder):
    def load(name):
        return json.loads((folder / name).read_text())
    steps = [json.loads(s) for s in (folder / 'operations.jsonl').read_text().splitlines()]
    post = load('postflight.json')
    clean = all(v is True for k, v in post.items() if k != 'root')
    complete = (folder / 'configuration-complete.json').exists()
    failed = [s for s in steps if s['phase'] == 'failed']
    if complete:
        receipt = load('configuration-complete.json')
        guard = load('guardian.json')
        if receipt != {'complete': True, 'activated': False} or failed or not clean:
            raise ValueError('Configuration completion contradicts failure/cleanup')
        if guard['reason'] != 'normal_completion':
            raise ValueError('Complete configuration requires normal guardian retirement')
        planned = load('session-intent.json')['definitions']
        actual = load('partial-state.json')['registered_probes'].splitlines()
        if {' '.join(s.split()) for s in actual} != {' '.join(s.split()) for s in planned}:
            raise ValueError('Registered definitions differ from intent')
    if any(s['operation'] == 'write' and Path(s['path']).name == 'tracing_on'
           and s['value'] != '0' for s in steps):
        raise ValueError('Buffer activation is outside this lane')
    return {'method': 'hc074-nonseeking-registry-configuration-v1',
            'configuration_complete': complete, 'failed_operations': failed,
            'completed_operations': sum(s['phase'] == 'after' for s in steps),
            'cleanup_context_confirmed': clean, 'configuration_only': True,
            'clock_queries': 0, 'trace_activation': False,
            'admission': False, 'server': False, 'product': False, 'original_owner_run': False}


def collect():
    configure_archive()
    if subprocess.check_output(['git', 'status', '--porcelain'], cwd=base.ROOT).strip():
        raise ValueError('Commit the reviewed fresh lane before dispatch')
    proof = base.sources()
    OUT.mkdir(parents=True, exist_ok=False)
    base.put(OUT / 'intent.json', {'sources': proof, 'frozen_inputs': base.FROZEN,
        'git': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=base.ROOT, text=True).strip(),
        'authorization': 'user continued in direct response to one fresh configuration-only WSL session proposal',
        'configuration_only': True, 'clock_queries': 0, 'trace_activation': False,
        'original_windows': [300, 60, 10], 'retry': False, 'clock_change': False,
        'service_change': False, 'original_owner_run': False, 'server': False,
        'product': False, 'admission': False})
    base.command('controls', ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec',
        'python3', '-B', '-m', 'unittest', 'discover', '-s', base.linux(base.HERE), '-v'], 45)
    try:
        base.command('session', ['wsl.exe', '-d', 'Ubuntu', '-u', 'root', '--exec',
            'python3', '-B', base.linux(Path(__file__)), 'session'], 180)
    finally:
        if proof != base.sources():
            raise ValueError('Source or frozen-input drift')
        if (OUT / 'postflight.json').exists() and (OUT / 'operations.jsonl').exists():
            base.put(OUT / 'analysis.json', outcome(OUT))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('collect', 'session', 'retain', 'verify'))
    mode = parser.parse_args().mode
    if mode == 'session':
        einval.OUT = OUT
        einval.session()  # No changes to preflight, setup order, guardian or deadlines.
    elif mode == 'collect':
        collect()
    else:
        configure_archive()
        getattr(base, mode)()
