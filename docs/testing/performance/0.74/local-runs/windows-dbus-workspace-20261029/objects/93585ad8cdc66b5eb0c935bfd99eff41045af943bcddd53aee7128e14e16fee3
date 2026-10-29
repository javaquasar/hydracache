"""Fixed local attribution. Exclusive output directories retain first results."""
from pathlib import Path
import difflib
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OLD = ROOT / 'target/chitchat-mtu-20261028'
BASE = ROOT / 'target/chitchat-preview-cost-20261028'
sys.path.insert(0, str(ROOT / 'tools/windows-workspace-preflight-074'))
import owned
import prepare as p


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def write(folder, name, value):
    (folder / name).write_text(json.dumps(value, indent=2, sort_keys=True) + '\n', encoding='utf-8', newline='\n')


def files(folder):
    paths = [n for n in sorted(folder.rglob('*')) if n.is_file() and '__pycache__' not in n.parts]
    p.require(all(not n.is_symlink() and not n.is_junction() for n in paths), 'plain source members')
    return {n.relative_to(folder).as_posix(): sha(n.read_bytes()) for n in paths}


def prepare():
    out = BASE / 'prepare'
    out.mkdir(parents=True, exist_ok=False)
    (out / 'producer.py').write_bytes(Path(__file__).read_bytes())
    prior = json.loads((OLD / 'preparation/S/result.json').read_bytes())
    p.require(files(OLD / 'fixtures/S') == prior['files'], 'exact retained S')
    b = json.loads((OLD / 'preparation/B/result.json').read_bytes())
    p.require(files(OLD / 'fixtures/B') == b['files'], 'exact retained B')
    roots = prior['root_inputs']
    for name, digest in {**roots, **prior['overlays']}.items():
        p.require(sha((ROOT / name).read_bytes()) == digest, 'original input: ' + name)
    overlay = ROOT / 'tools/chitchat-preview-cost-074/cost_tests.rs'
    overlays = dict(prior['overlays'], **{overlay.relative_to(ROOT).as_posix(): sha(overlay.read_bytes())})
    source = (OLD / 'fixtures/B/src/serialize.rs').read_text(encoding='utf-8')
    start = '    pub fn serialized_len_upperbound_after<'
    stop = '    /// Appends a new item'
    original = source[source.index(start):source.index(stop)]
    result = dict(root_inputs=roots, overlays=overlays, variants={},
                  retry=False, release_admitted=False, compiler='1.94.0', profile='release')
    for variant in ('C0', 'C1'):
        dest = BASE / 'fixtures' / variant
        dest.parent.mkdir(exist_ok=True)
        shutil.copytree(OLD / 'fixtures/S', dest)
        path = dest / 'src/serialize.rs'
        before = path.read_text(encoding='utf-8')
        after = before
        if variant == 'C0':
            after = before[:before.index(start)] + original + before[before.index(stop):]
        observer = '''
#[cfg(test)]
impl CompressedStreamWriter {
    pub(crate) fn cost_inventory(&self) -> ([usize; 3], usize) {
        ([self.output.len(), self.uncompressed_block.len(), self.compressed_block.len()], self.block_threshold)
    }
}
'''
        path.write_text(after + observer, encoding='utf-8', newline='\n')
        append = '\n#[cfg(test)]\n#[path = ' + json.dumps(overlay.as_posix()) + ']\nmod cost_review;\n'
        path = dest / 'src/delta.rs'
        path.write_bytes(path.read_bytes() + append.encode())
        write(out, variant + '-source.json', files(dest))
        (out / (variant + '.patch')).write_text(''.join(difflib.unified_diff(
            before.splitlines(True), (after + observer).splitlines(True),
            fromfile='S/src/serialize.rs', tofile=variant + '/src/serialize.rs')), encoding='utf-8', newline='\n')
        members = files(dest)
        identity = sha(json.dumps(dict(files=members, overlays=overlays,
            compiler='1.94.0', profile='release'), sort_keys=True).encode())
        result['variants'][variant] = dict(files=members, target=str(BASE / 'targets' / variant / identity))
    p.require((BASE / 'fixtures/C0/Cargo.lock').read_bytes() == (BASE / 'fixtures/C1/Cargo.lock').read_bytes(), 'identical dependency graph')
    write(out, 'result.json', result)
    print('prepared C0/C1; root graph unchanged')


def snapshot():
    names = subprocess.check_output(['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard', '--',
        'Cargo.toml', 'Cargo.lock', 'deny.toml', '.cargo', 'crates', 'tools', 'scripts'], cwd=ROOT).split(b'\0')
    return {n.decode(): sha((ROOT / n.decode()).read_bytes()) for n in sorted(set(names)) if n}


def guard(review):
    for name, digest in {**review['root_inputs'], **review['overlays']}.items():
        p.require(sha((ROOT / name).read_bytes()) == digest, 'fixed input: ' + name)
    for variant, row in review['variants'].items():
        p.require(files(BASE / 'fixtures' / variant) == row['files'], 'complete fixture: ' + variant)


def inventory(raw, baseline):
    lines = raw.decode().splitlines()
    names = [line[:-6] for line in lines if line.endswith(': test')]
    extras = ['delta::cost_review::' + n for n in ('allocator_control', 'allocation_observations', 'cpu_observations')]
    p.require(len(baseline) == len(set(baseline)) == 106, 'original extended catalog')
    p.require(len(names) == len(set(names)) == 109 and lines[-1] == '109 tests, 0 benchmarks', 'whole attribution catalog')
    p.require(sorted(names) == sorted(baseline + extras), 'only exact three additions')
    return sorted(names)


def payload(raw, marker):
    rows = [line[len(marker):] for line in raw.splitlines() if line.startswith(marker)]
    p.require(len(rows) == 1, 'one complete diagnostic: ' + marker.decode())
    return json.loads(rows[0])


def semantic(rows, original):
    p.require(len(rows) == len(original) == 239, 'complete original operation matrix')
    project = lambda values: [(r['mtu'], r['value_len'], r['accepted'], r['wire']) for r in values]
    p.require(len({(r['mtu'], r['value_len']) for r in rows}) == 239, 'unique workload cases')
    p.require(project(rows) == project(original), 'identical original workload/admission/exact wire')


def run(label, worker):
    match = re.fullmatch(r'(build|list|control|delta|check|clippy)-(C0|C1)|cohort-([1-4])', label)
    p.require(match is not None, 'fixed step')
    action, variant, trial = match.groups()
    if trial:
        action, variant = 'cohort', ('C0', 'C1', 'C1', 'C0')[int(trial)-1]
    review = json.loads((BASE / 'prepare/result.json').read_bytes())
    row = review['variants'][variant]
    folder = BASE / 'fixtures' / variant
    target = Path(row['target'])
    p.require(target.resolve().is_relative_to(BASE / 'targets' / variant), 'disjoint held target')
    common = ['--offline', '--locked', '--manifest-path', str(folder / 'Cargo.toml'), '--target-dir', str(target), '--release']
    artifact = None
    selection = None
    if action == 'build':
        argv = ['cargo', '+1.94.0', 'test', *common, '--lib', '--features', 'testsuite', '--no-run', '--message-format=json']
    elif action in ('check', 'clippy'):
        argv = ['cargo', '+1.94.0', action, *common, '--all-targets', '--all-features'] + (['--', '-D', 'warnings'] if action == 'clippy' else [])
    else:
        build = json.loads((BASE / ('build-' + variant) / 'result.json').read_bytes())
        p.require(build['exit'] == 0 and build['unchanged'], 'successful original build')
        artifact = build['artifact']
        image = Path(artifact['path'])
        p.require(image.resolve().is_relative_to(target) and not image.is_symlink() and not image.is_junction()
            and sha(image.read_bytes()) == artifact['sha256'], 'original bound executable')
        if action == 'list':
            argv = [str(image), '--list']
        else:
            listing = json.loads((BASE / ('list-' + variant) / 'result.json').read_bytes())
            p.require(listing['exit'] == 0 and listing['unchanged'] and listing['artifact'] == artifact, 'held catalog')
            selection = dict(control='delta::cost_review::allocator_control',
                delta='delta::mtu_review::supported_delta_operations', cohort='delta::cost_review::')[action]
            if action == 'cohort':
                for name in ('control', 'delta'):
                    control = json.loads((BASE / (name + '-' + variant) / 'result.json').read_bytes())
                    p.require(control['exit'] == 0 and control['unchanged'] and control['artifact'] == artifact, 'original control: ' + name)
                    if name == 'delta':
                        p.require(control['original_semantics_equal'], 'original semantic control')
                if trial != '1':
                    preceding = json.loads((BASE / ('cohort-' + str(int(trial)-1)) / 'result.json').read_bytes())
                    p.require(preceding['exit'] == 0 and preceding['unchanged'], 'no continuation after prior cohort refusal')
            argv = [str(image), selection, *([] if action == 'cohort' else ['--exact']), '--test-threads=1', '--show-output']
    out = BASE / label
    if worker:
        guard(review)
        def capacity():
            if shutil.disk_usage(ROOT).free < p.FLOOR:
                raise owned.CapacityError('original 4 GiB floor')
        result = owned._owned(owned.load_job().WindowsJob(), argv, str(ROOT), capacity, bound=1200)
        write(out, 'execution.json', result)
        raise SystemExit(result['exit'])
    out.mkdir(exist_ok=False)
    (out / 'launcher.py').write_bytes(Path(__file__).read_bytes())
    with (out / 'stdout.log').open('xb') as stdout, (out / 'stderr.log').open('xb') as stderr:
        try:
            before = snapshot()
            head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT).decode().strip()
            write(out, 'intent.json', dict(argv=argv, source=before, source_head=head,
                variant=variant, cohort_trial=trial, artifact=artifact, bound=1200,
                floor=p.FLOOR, capacity_poll=5, retirement=5, retry=False, server=False,
                services=False, clocks=False, release_admitted=False))
            guard(review)
            process = subprocess.Popen([sys.executable, '-B', str(Path(__file__)), label, '--worker'], cwd=ROOT,
                stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr,
                env=dict(os.environ, CARGO_INCREMENTAL='0', CARGO_BUILD_JOBS='2', PROPTEST_RNG_SEED='7606312'))
            try:
                code = process.wait(timeout=1210)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
                raise ValueError('worker lost; no PID fallback')
            guard(review)
            execution = json.loads((out / 'execution.json').read_bytes())
            unchanged = before == snapshot() and head == subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT).decode().strip()
            p.require(unchanged and execution['retired'] and not execution['errors'], 'source/head and owned retirement postflight')
            result = dict(exit=code, unchanged=unchanged, artifact=artifact, release_admitted=False)
            raw = (out / 'stdout.log').read_bytes()
            if action == 'build' and code == 0:
                messages = [json.loads(line) for line in raw.splitlines()]
                found = [m for m in messages if m.get('reason') == 'compiler-artifact' and m['target']['kind'] == ['lib']
                    and m['target']['name'] == 'chitchat' and m['profile']['test'] and m.get('executable')]
                p.require(len(found) == 1 and [m['success'] for m in messages if m.get('reason') == 'build-finished'] == [True], 'complete build')
                image = Path(found[0]['executable'])
                p.require(image.resolve().is_relative_to(target) and Path(found[0]['target']['src_path']).resolve() == (folder / 'src/lib.rs').resolve(), 'actual artifact source')
                result['artifact'] = dict(path=str(image), sha256=sha(image.read_bytes()), bytes=image.stat().st_size)
            if artifact:
                p.require(sha(Path(artifact['path']).read_bytes()) == artifact['sha256'], 'image postflight')
            if action == 'list' and code == 0:
                result['inventory'] = inventory(raw, json.loads((OLD / 'list-S-194/result.json').read_bytes())['inventory'])
            if selection:
                accounting = p.load(ROOT / 'tools/chitchat-mtu-review-074/accounting.py', 'cost_account')
                result['accounting'] = accounting.account(raw, listing['inventory'], selection, action != 'cohort')
                p.require((code == 0) == result['accounting']['test_pass'], 'native status/count agreement')
            if action in ('delta', 'cohort') and code == 0:
                original = payload((OLD / 'delta-B-194/stdout.log').read_bytes(), b'HC074_SUPPORTED_DELTA:')
                observed = payload(raw, b'HC074_SUPPORTED_DELTA:' if action == 'delta' else b'HC074_PREVIEW_ALLOC:')
                p.require(observed['seed'] == original['seed'] == 0x0741028, 'same input seed')
                semantic(observed['cases' if action == 'delta' else 'rows'], original['cases'])
                result['original_semantics_equal'] = True
            write(out, 'result.json', result)
        except BaseException as error:
            write(out, 'refusal.json', dict(type=type(error).__name__, reason=str(error), release_admitted=False))
            raise
    print(json.dumps(dict(label=label, exit=code, unchanged=unchanged)))
    raise SystemExit(code)


if __name__ == '__main__':
    p.require(len(sys.argv) in (2, 3) and (len(sys.argv) == 2 or sys.argv[2] == '--worker'), 'fixed invocation')
    if sys.argv[1] == 'prepare':
        p.require(len(sys.argv) == 2, 'prepare is inert')
        prepare()
    else:
        run(sys.argv[1], len(sys.argv) == 3)
