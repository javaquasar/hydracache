"""Fixed first diagnostic of actual source IO/gates and independent components."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
PATHS = tuple('crates/xtask/tests/support/performance_contract_074_' + name + '.rs'
              for name in ('fixed_slice', 'closing_validation', 'held_placement'))
OPERATIONS = ('io-e2e', 'view', 'expected', 'accepts', 'storage', 'syntax', 'expected-tokens', 'actual-tokens')
BASELINE = '5648a0d8'


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def review(raw):
    lines = raw.decode().splitlines()
    def rows(marker):
        return [json.loads(line[len(marker):]) for line in lines if line.startswith(marker)]
    samples = rows('HC074_NORMALIZATION ')
    setup, observers, done = (rows(marker) for marker in
        ('HC074_NORMALIZATION_PREP ', 'HC074_NORMALIZATION_OBSERVER ', 'HC074_NORMALIZATION_DONE '))
    require(len(setup) == 3 and len(samples) == 144 and len(observers) == 6, 'fixed complete scope')
    expected = dict(blocks=6, samples=144, observer_samples=6, preparation_samples=3,
        historical_cache_prepared=True, warmup=False, component_intervals_are_disjoint=False, product=False, package_replay=False)
    require(done == [expected] and all(type(done[0][key]) is type(value) for key, value in expected.items()), 'completion')
    sizes, groups = {}, {}
    def times(row):
        require(all(type(row.get(k)) is int and row[k] >= 0 for k in ('wall_ns','parent_cpu_ns')), 'sample time')
    for path, row in zip(PATHS, setup):
        require(row.get('path') == path and row.get('historical_cache_prepared') is True and row.get('warmup') is False, 'preparation order')
        times(row)
    for block, row in enumerate(observers):
        require(type(row.get('block')) is int and row['block'] == block, 'observer order')
        times(row)
    i = 0
    for block in range(6):
        operations = OPERATIONS if block % 2 == 0 else OPERATIONS[::-1]
        for path in PATHS:
            for operation in operations:
                row = samples[i]
                i += 1
                require(type(row.get('block')) is int and (row['block'],row.get('path'),row.get('operation')) == (block,path,operation), 'sample order')
                require(type(row.get('iterations')) is int and row['iterations'] == 1, 'iterations')
                require(all(type(row.get(k)) is int and row[k] > 0 for k in ('actual_bytes','expected_bytes')), 'source lengths')
                pair = row['actual_bytes'],row['expected_bytes']
                require(sizes.setdefault(path,pair) == pair, 'source length changed')
                times(row)
                groups.setdefault(path + '/' + operation, []).append(row)
    def describe(values):
        return dict(samples=len(values),min=min(values),median=statistics.median(values),max=max(values),mean=statistics.mean(values))
    summary = {key:{metric:describe([row[metric] for row in group]) for metric in ('wall_ns','parent_cpu_ns')}
               for key,group in groups.items()}
    return dict(samples=samples,preparation=setup,observers=observers,summary=summary,
        component_intervals_are_disjoint=False,observer_overhead_subtracted=False,
        historical_cache_prepared=True,full_historical_normalizer_chain=False,
        actual_runtime_io_and_gate=True,parent_cpu_excludes_git=True,
        product_candidate=False,package_replayed=False,production_admission=False)


def capture(build,out):
    out.mkdir(parents=True,exist_ok=False)
    def write(name,value):
        with (out/name).open('x',encoding='utf-8',newline='\n') as stream:
            json.dump(value,stream,indent=2,sort_keys=True)
            stream.write('\n')
    write('intent.json',dict(paths=PATHS,operations=OPERATIONS,blocks=6,samples=144,
        timeout_seconds=180,disk_floor_bytes=4*1024**3,warmup=False,retry=False,
        historical_cache_preparation_explicit=True,server=False,services=False,clocks=False))
    process = None
    try:
        require(os.name == 'nt' and shutil.disk_usage(ROOT).free >= 4*1024**3,'platform/disk preflight')
        require(not any(key.startswith(('HC074_','RUST_TEST_')) for key in os.environ),'inherited fixture overrides')
        result = json.loads((build/'result.json').read_bytes())
        source = json.loads((build/'intent.json').read_bytes())['source']
        require(result['exit'] == 0 and result['source_unchanged'] and result['head_unchanged'],'build receipt')
        for name,digest in source.items():
            if name.endswith(('.rs','.toml','.lock')):
                require(sha((ROOT/name).read_bytes()) == digest,'compiled source drift')
        artifacts = [json.loads(line) for line in (build/'stdout.log').read_text().splitlines() if line.startswith('{')]
        artifact, = [row for row in artifacts if row.get('reason') == 'compiler-artifact'
                     and row.get('target',{}).get('name') == 'authority_context_074']
        binary = Path(artifact['executable']).resolve()
        require(binary.parent == (ROOT/'target/debug/deps').resolve(),'binary path')
        paths = []
        for name in PATHS:
            committed = subprocess.check_output(['git','show',BASELINE+':'+name],cwd=ROOT)
            current = (ROOT/name).read_bytes()
            require(committed.replace(b'\r\n',b'\n') == current.replace(b'\r\n',b'\n'),'pinned baseline source drift')
            paths.append(dict(path=name,sha256=sha(current),committed_sha256=sha(committed)))
        write('pins.json',dict(paths=paths,production_authority=False))
        write('preflight.json',dict(binary_sha256=sha(binary.read_bytes()),compiled_source=source))
        with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
            process = subprocess.Popen([str(binary),'--exact','normalization_cost_child','--nocapture','--test-threads=1'],
                cwd=ROOT,stdout=stdout,stderr=stderr,env=dict(os.environ,HC074_NORMALIZATION_INPUT=str((out/'pins.json').resolve())))
            started = time.monotonic()
            while process.poll() is None:
                require(time.monotonic()-started < 180,'outer timeout')
                require(shutil.disk_usage(ROOT).free >= 4*1024**3,'disk floor')
                try: process.wait(timeout=1)
                except subprocess.TimeoutExpired: pass
        require(process.returncode == 0,'first child refused')
        require(all(sha((ROOT/name).read_bytes()) == digest for name,digest in source.items()),'source changed')
        write('result.json',review((out/'stdout.log').read_bytes()))
        print(json.dumps(dict(samples=144,preparation_samples=3,observer_samples=6,measured_child_repeated=False)))
    except Exception as error:
        write('refusal.json',dict(type=type(error).__name__,reason=str(error),exit=None if process is None else process.poll()))
        if process is not None and process.poll() is None:
            with (out/'cleanup.log').open('xb') as log:
                subprocess.run(['taskkill','/PID',str(process.pid),'/T','/F'],stdout=log,stderr=subprocess.STDOUT,timeout=30,check=False)
            process.wait(timeout=30)
        raise


if __name__ == '__main__':
    capture(Path(sys.argv[1]).resolve(),Path(sys.argv[2]).resolve())
