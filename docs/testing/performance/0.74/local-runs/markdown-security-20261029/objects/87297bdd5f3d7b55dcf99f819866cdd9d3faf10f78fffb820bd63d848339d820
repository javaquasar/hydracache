"""First-only, bounded fresh-command screen. No installed-helper replacement."""
from pathlib import Path
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time

import pipe_review
import review

ROOT = Path(__file__).resolve().parents[2]


def capture(build_label, out_label):
    for value in (build_label, out_label):
        pipe_review.require(re.fullmatch(r'[a-z0-9-]+', value), 'fixed local label')
    out = ROOT / 'target' / out_label
    out.mkdir(exist_ok=False)

    def write(name, value):
        with (out / name).open('x', encoding='utf-8') as stream:
            json.dump(value, stream, indent=2, sort_keys=True)
            stream.write('\n')

    write('intent.json', dict(build=build_label, bound_seconds=120, log_budget_bytes=16*1024**2,
        disk_floor_bytes=4*1024**3, retry=False, workers=4, samples=96, package=False,
        server=False, services=False, clocks=False, admission=False))
    process = None
    try:
        pipe_review.require(os.name == 'nt', 'Windows only')
        pipe_review.require(not any(k.startswith(('HC074_', 'RUST_TEST_')) for k in os.environ), 'inherited overrides')
        pipe_review.require(shutil.disk_usage(ROOT).free >= 4*1024**3, 'disk floor')
        build = ROOT / 'target/source-continuation-20261024' / build_label
        source = json.loads((build/'intent.json').read_bytes())
        result = json.loads((build/'result.json').read_bytes())
        pipe_review.require(result['exit'] == 0 and result['outer_refusal'] is None
                            and result['source_unchanged'] and result['head_unchanged'], 'successful frozen build')
        pipe_review.require(source['head'] == subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(), 'build HEAD')
        pipe_review.require(all(hashlib.sha256((ROOT/name).read_bytes()).hexdigest() == digest
                                for name, digest in source['source'].items()), 'compiled source drift')
        artifacts = [json.loads(line) for line in (build/'stdout.log').read_text().splitlines() if line.startswith('{')]
        matches = [v['executable'] for v in artifacts if v.get('reason') == 'compiler-artifact'
                   and v['target']['name'] == 'direct_output_cost_074' and v.get('executable')]
        pipe_review.require(len(matches) == 1, 'exact build artifact')
        executable = Path(matches[0])
        executable.resolve().relative_to((ROOT/'target').resolve())
        executable_sha = hashlib.sha256(executable.read_bytes()).hexdigest()
        write('preflight.json', dict(executable=str(executable), executable_sha256=executable_sha,
            compiled_source=source['source'], head=source['head'], protocol_sha256=hashlib.sha256(
                (ROOT/'tools/xtask-package-attribution-074/pipe-execution-protocol.toml').read_bytes()).hexdigest()))
        with (out/'stdout.log').open('xb') as stdout, (out/'stderr.log').open('xb') as stderr:
            process = subprocess.Popen([str(executable), '--exact', 'direct_output_cost_child', '--nocapture', '--test-threads=1'],
                cwd=ROOT, stdout=stdout, stderr=stderr, env=dict(os.environ, HC074_PIPE_COST='v1'))
            start = time.monotonic()
            while process.poll() is None:
                pipe_review.require(time.monotonic()-start < 120, 'outer timeout')
                pipe_review.require(shutil.disk_usage(ROOT).free >= 4*1024**3, 'disk floor')
                pipe_review.require(sum((out/name).stat().st_size for name in ('stdout.log', 'stderr.log')) <= 16*1024**2, 'log budget')
                try: process.wait(timeout=0.25)
                except subprocess.TimeoutExpired: pass
        raw = (out/'stdout.log').read_bytes()
        pipe_review.require(process.returncode == 0 and not (out/'stderr.log').read_bytes(), 'successful producer')
        pipe_review.require(len(raw) <= 16*1024**2, 'post-exit log budget')
        lines = raw.decode().splitlines()
        starts = [review.START.fullmatch(v) for v in lines if review.START.fullmatch(v)]
        ends = [review.END.fullmatch(v) for v in lines if review.END.fullmatch(v)]
        pipe_review.require(len(starts) == len(ends) == 1 and int(starts[0][1]) == 1, 'single test')
        pipe_review.require(ends[0][1] == 'ok' and list(map(int, ends[0].group(2, 3, 4, 5, 6))) == [1, 0, 0, 0, 4], 'exact verdict')
        pipe_review.require(lines.count('test direct_output_cost_child ... ') == 1 and lines.count('ok') == 1, 'exact selected case')
        derived = pipe_review.read(raw)
        pipe_review.require(hashlib.sha256(executable.read_bytes()).hexdigest() == executable_sha, 'executable drift')
        pipe_review.require(all(hashlib.sha256((ROOT/name).read_bytes()).hexdigest() == digest
                                for name, digest in source['source'].items()), 'postflight source drift')
        pipe_review.require(source['head'] == subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(), 'postflight HEAD drift')
        write('result.json', dict(exit=process.returncode, first_outcome=True, retry=False, package_pass=False, derived=derived))
        return derived
    except BaseException as error:
        if process is not None and process.poll() is None:
            with (out/'cleanup.log').open('xb') as stream:
                subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'], stdout=stream,
                               stderr=subprocess.STDOUT, timeout=30, check=False)
            process.wait(timeout=30)
        write('refusal.json', dict(type=type(error).__name__, reason=str(error),
            exit=None if process is None else process.returncode, retry=False))
        raise


if __name__ == '__main__':
    print(json.dumps(capture(*sys.argv[1:]), indent=2, sort_keys=True))
