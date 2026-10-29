"""One separately reviewed five-target GNU attempt, never full-package admission.

The fixed native source is intentionally not the current documentation HEAD.
Archived review inputs are data only. No fixture, deadline or test is changed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

SOURCE = '636feb166b273b5c3d6fc0b93c4fccf57560cf03'
ROOT = Path('/home/artur/.cache/hydracache-074-frontier-native') / SOURCE / 'repo'
CONTROL = Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
OUT = CONTROL / 'target/native-gnu-frontier-first-20261027'
TARGET = Path('/home/artur/.cache/hydracache-074-194-check')
REQUEST_SHA = '44aaf01ea277b78a0ac3b2a77e382e61ac92ffebe0c4ba12a615eee5efd0e3e4'
REVIEW_PACKET = 'docs/testing/performance/0.74/local-runs/package-review-v9-20261027'
REVIEW_INDEX = 'bd4ae61a825cf06bb4a1a5e90f98ff7fd04da5891809ac5a5c56130b9e7a5451'
SOURCE_PACKET = 'docs/testing/performance/0.74/local-runs/package-effect-syntax-v9-20261027'
SOURCE_INDEX = 'e2c54681e571204fc927d7df3fe99a08303474c52af7a945d1a2764bb601bfc9'
TARGETS = {
    'frontier_current_view_074': 9,
    'current_frontier_074': 7,
    'current_tree_input_audit_074': 1,
    'authority_rust_history_074': 10,
    'history_status_readers_074': 4,
}
HELPERS = {
    'contract.py': '13a92353314ab7b6b3f443f4d8244b823e38f390a35885edd050bbb73a5c2353',
    'native.py': '8badd8357fd491b01e65c3c118421cdd8f41eb952909678c97f78ac2bffe3769',
}
HEARTBEAT = {
    'run-with-heartbeat.py': 'cedf938f80b3697efa219e5e8566d5d4f869cc2769dc7de6a176efb6e3af7afb',
    'watchdog_linux_group.py': 'e5ff52835fc2486171dbdbbb1b6df47177f583b4539f561779992c2c5065a691',
    'watchdog_windows_job.py': '8b392811e106eb7e692e21ee5b55cd04ea8ac421cad015edb66ce736b08c2ab1',
}
FLOOR = 4 * 1024**3


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def decode(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs)


def index(root, packet, digest):
    raw = (root / packet / 'index.json').read_bytes()
    require(sha(raw) == digest, 'fixed review index differs')
    return decode(raw)


def member(root, packet, document, name):
    row = document['members'][name]
    digest = row['sha256']
    require(re.fullmatch('[0-9a-f]{64}', digest), 'unsafe object digest')
    raw = (root / packet / 'objects' / digest).read_bytes()
    require(sha(raw) == digest and len(raw) == row['bytes'], 'review object differs')
    return raw


def reviewed_bytes(name, native, archived):
    if native == archived:
        return 'raw-exact'
    # Only text formats already used by the source review may change CRLF to LF.
    # Extensionless content-addressed objects always require exact raw bytes.
    text = Path(name).suffix in {'.rs', '.toml', '.py', '.txt', '.md', '.json'} or Path(name).name == '.gitattributes'
    require(text and b'\0' not in native + archived, 'nontext review bytes differ: ' + name)
    native.decode('utf-8')
    archived.decode('utf-8')
    require(b'\r' not in native and native == archived.replace(b'\r\n', b'\n'), 'non-EOL source drift: ' + name)
    return 'reviewed-crlf-to-lf'


def command():
    argv = ['/home/artur/.cargo/bin/cargo', '+1.94.0', 'test', '--offline', '--locked',
            '--no-fail-fast', '--target-dir', str(TARGET), '-p', 'xtask']
    for name in TARGETS:
        argv += ['--test', name]
    return argv + ['--', '--test-threads=4']


def complete(exit_code, raw):
    text = raw.decode('utf-8', errors='strict')
    names = re.findall(r'Running tests/([a-z0-9_]+)\.rs ', text)
    rows = re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;', text)
    return (exit_code == 0 and len(names) == len(set(names)) == 5
            and set(names) == set(TARGETS) and len(rows) == 5
            and all(int(count) == TARGETS[name] for name, count in zip(names, rows)))


def space():
    require(shutil.disk_usage('/mnt/c').free >= FLOOR, 'physical backing floor')
    require(shutil.disk_usage(ROOT).free >= FLOOR, 'native filesystem floor')


def preflight():
    require(os.name == 'posix' and os.geteuid() != 0, 'nonroot local Linux only')
    blocked = [k for k in os.environ if k.startswith(('CARGO_', 'RUST_', 'RUSTFLAGS', 'HC074_', 'HYDRACACHE_', 'GIT_', 'LD_'))
               and not (k == 'GIT_PAGER' and os.environ[k] == 'cat')]
    require(not blocked, 'inherited override names: ' + ','.join(blocked))
    for name, digest in HELPERS.items():
        require(sha((CONTROL / 'tools/native-frontier-074' / name).read_bytes()) == digest, 'helper drift')
    for name, digest in HEARTBEAT.items():
        require(sha((CONTROL / 'scripts/ci' / name).read_bytes()) == digest, 'retirement helper drift')
    sys.path.insert(0, str(CONTROL / 'tools/native-frontier-074'))
    import contract as c
    import native
    raw = (CONTROL / 'target/native-frontier-20261027' / SOURCE / 'request.json').read_bytes()
    require(sha(raw) == REQUEST_SHA, 'transfer request drift')
    request = c.request(c.decode(raw))
    require(request['source'] == SOURCE, 'native source drift')
    require(native.git(ROOT, 'rev-parse', 'HEAD').decode().strip() == SOURCE, 'native HEAD drift')
    require(c.refs(native.git(ROOT, 'for-each-ref', '--format=%(objectname) %(refname)')) == request['refs'], 'native refs drift')
    require(not (ROOT / '.git/objects/info/alternates').exists() and not (ROOT / '.git/commondir').exists(), 'borrowed objects')
    native.validate_files(ROOT, request['tree'], space)
    gate_index = index(CONTROL, REVIEW_PACKET, REVIEW_INDEX)
    gate = decode(member(CONTROL, REVIEW_PACKET, gate_index, 'review/gate.json'))
    source_index = index(CONTROL, SOURCE_PACKET, SOURCE_INDEX)
    require((len(gate['sources']), len(gate['auxiliary'])) == (386, 56), 'review census drift')
    compared = {}
    for name, digest in {**gate['sources'], **gate['auxiliary']}.items():
        archived = member(CONTROL, SOURCE_PACKET, source_index, 'fixture/' + name)
        require(sha(archived) == digest, 'review source pointer drift')
        compared[name] = reviewed_bytes(name, (ROOT / name).read_bytes(), archived)
    # Actual caller environment, not preparation's sanitized Git environment.
    value = subprocess.check_output(['/usr/bin/git', 'config', '--includes', '--show-origin', '--list'], cwd=ROOT, timeout=10)
    expected_config = (b'file:.git/config\tcore.repositoryformatversion=0\n'
        b'file:.git/config\tcore.filemode=true\nfile:.git/config\tcore.bare=false\n'
        b'file:.git/config\tcore.logallrefupdates=true\nfile:.git/config\tcore.autocrlf=false\n')
    require(value == expected_config, 'unreviewed actual Git configuration')
    require(shutil.which('git') == '/usr/bin/git', 'Git executable resolution differs')
    configs = [p / '.cargo' / n for p in (ROOT, *ROOT.parents) for n in ('config', 'config.toml')]
    configs += [Path('/home/artur/.cargo') / n for n in ('config', 'config.toml')]
    require(not any(p.exists() for p in configs if p != ROOT / '.cargo/config.toml'), 'shadow Cargo configuration')
    require(sha((ROOT / '.cargo/config.toml').read_bytes()) == 'db18cd83f159c3152bcab6c10bbaac35be479c67a24b48c0f508bd1c7eca7a34', 'Cargo configuration drift')
    require(TARGET.is_dir() and TARGET.resolve() == TARGET and TARGET.stat().st_uid == os.geteuid(), 'target custody')
    require(not (ROOT / 'target').exists(), 'unexpected in-tree target before attempt')
    toolchain = subprocess.check_output(['/home/artur/.cargo/bin/rustc', '+1.94.0', '--version', '--verbose'],
        timeout=15, env=dict(os.environ, RUSTUP_AUTO_INSTALL='0')).decode()
    require('host: x86_64-unknown-linux-gnu\n' in toolchain and 'release: 1.94.0\n' in toolchain, 'GNU toolchain differs')
    space()
    return native, request, dict(source=SOURCE, request_sha256=REQUEST_SHA, compared=compared,
        full_tree_and_refs_exact=True, actual_git_config_sha256=sha(value), shadow_cargo_config_absent=True,
        inherited_overrides_absent=True, toolchain=toolchain, argv=command(), timeout_seconds=900,
        test_threads=4, disk_floor_bytes=FLOOR, source_review_scope='only five GNU targets; V9 data is a source superset, not GNU whole-package clearance',
        effect_review='Fixed historical/current Git reads retain all protected paths. In-memory string/AST mutations and owned tempfile/symlink controls only. No manager/SSH/systemctl/clock/product entry. Locked vendored protoc build writes OUT_DIR; offline compilation uses the existing owned target. No hooks, filters, remotes, alternates or lazy fetch configuration. Prior GNU timeout and V8 package refusal remain unchanged.',
        package_pass=False, release_admitted=False)


def run():
    require(OUT.parent.resolve(strict=True) == OUT.parent, 'output ancestor alias')
    OUT.mkdir(exist_ok=False)
    def write(name, value):
        with (OUT / name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, indent=2, sort_keys=True)
            stream.write('\n')
    write('intent.json', dict(source=SOURCE, argv=command(), timeout_seconds=900, test_threads=4,
        launcher_sha256=sha(Path(__file__).read_bytes()), helpers=HELPERS, heartbeat=HEARTBEAT,
        retry=False, full_package=False, server=False, services=False, clocks=False, product_load=False))
    try:
        native, request, review = preflight()
        write('preflight.json', review)
        argv = ['/usr/bin/python3', str(CONTROL / 'scripts/ci/run-with-heartbeat.py'),
            '--timeout-seconds', '900', '--heartbeat-seconds', '30', '--status-json', str(OUT / 'heartbeat.json'),
            '--log-file', str(OUT / 'frontier.log'), '--cwd', str(ROOT), '--', *command()]
        with (OUT / 'stdout.log').open('xb') as stdout, (OUT / 'stderr.log').open('xb') as stderr:
            result = subprocess.run(argv, cwd=ROOT, stdout=stdout, stderr=stderr, check=False,
                env=dict(os.environ, RUSTUP_AUTO_INSTALL='0', CARGO_INCREMENTAL='0', CARGO_BUILD_JOBS='2'))
        native.validate_files(ROOT, request['tree'], space)
        require(native.git(ROOT, 'rev-parse', 'HEAD').decode().strip() == SOURCE, 'postflight HEAD drift')
        require(native.c.refs(native.git(ROOT, 'for-each-ref', '--format=%(objectname) %(refname)')) == request['refs'], 'postflight refs drift')
        receipt = decode((OUT / 'heartbeat.json').read_bytes())
        passed = (receipt['classification'] == 'success' and receipt['exit_code'] == 0
                  and receipt['command'] == command()
                  and complete(result.returncode, (OUT / 'frontier.log').read_bytes()))
        write('result.json', dict(exit=result.returncode, frontier_complete=passed, expected_cases=31,
            full_tree_and_refs_unchanged=True, package_pass=False, release_admitted=False))
        print(json.dumps(dict(exit=result.returncode, frontier_complete=passed, package_pass=False)))
        return int(not passed)
    except Exception as error:
        write('refusal.json', dict(type=type(error).__name__, reason=str(error), package_pass=False, release_admitted=False))
        raise


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', action='store_true', help='one exclusive local five-target attempt')
    args = parser.parse_args()
    if args.run:
        raise SystemExit(run())
    print(json.dumps(preflight()[2], sort_keys=True, indent=2))
