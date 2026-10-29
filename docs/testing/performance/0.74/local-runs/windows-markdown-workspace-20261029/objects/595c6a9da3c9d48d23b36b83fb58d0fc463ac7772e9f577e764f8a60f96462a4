"""Explicit new CI custody context after retaining a terminal first refusal.

No source/lock/profile/runtime predicate changes. The full source frame surrounds
the compiled-image batch; individual images retain hashes and owned retirement.
This command does not rerun any phase or overwrite any previous result.
"""
from pathlib import Path
import json
import subprocess

import common as c

OLD = c.ROOT/'target/dbus-root-package-20261028'
NEW = c.ROOT/'target/dbus-root-package-20261028-custody2'
TOOLS = Path(__file__).parent


def main():
    c.need(c.BASE == OLD and not NEW.exists(), 'one explicit new CI context')
    review = json.loads((OLD/'freeze/runtime-reviewed.json').read_bytes())
    execution = json.loads((OLD/'checks/execution.json').read_bytes())
    refusal = json.loads((OLD/'checks/refusal.json').read_bytes())
    c.need(execution['retired'] and not execution['errors'] and execution['exit'] != 0
           and refusal, 'terminal retained first refusal, never retry a running attempt')
    census = subprocess.run(['wsl.exe','-d','Ubuntu','-u','artur','--cd','/','--exec',
        '/usr/bin/python3','-B',
        '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-root-package-074/retirement.py'],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=20)
    c.need(census.returncode == 0 and not json.loads(census.stdout)['active'],
           'old native session/worker/build completely gone before helper replacement')
    c.guard(review)
    for name in ('source-custody-full-control',):
        c.need(json.loads((OLD/name/'result.json').read_bytes())['passed'], 'complete 2232-file custody control')
    c.need('OK' in (OLD/'batch-inert.stderr.log').read_text(), 'complete inert accounting controls')
    saved = OLD/'helper-before-custody2'
    saved.mkdir(exist_ok=False)
    for name in review['helpers']:
        if name.startswith('tools/dbus-root-package-074/'):
            (saved/Path(name).name).write_bytes((c.ROOT/name).read_bytes())
    common = (TOOLS/'common.py').read_text()
    before = 'ROOT, BASE, PKG = app.ROOT, app.BASE, app.PKG'
    after = "ROOT, PKG = app.ROOT, app.PKG\nBASE = ROOT / 'target/dbus-root-package-20261028-custody2'"
    c.need(common.count(before) == 1, 'exact original common context')
    common = common.replace(before, after)
    before = 'def hashes(names):\n    result = {}'
    after = "def hashes(names):\n    if os.name == 'posix':\n        import source_guard\n        return source_guard.hashes(ROOT, names)\n    result = {}"
    c.need(common.count(before) == 1, 'exact old source-custody function')
    common = common.replace(before, after)
    native = (TOOLS/'native.py').read_text()
    first, last = native.index('def tests('), native.index('def session(')
    replacement = '''def tests(compiler,review,out,deadline):
    folder=invoke(out,'build',*cargo_command(compiler,'build'),review,deadline)
    images=artifacts((folder/'stdout.log').read_bytes(),review,compiler)
    c.write(out,'images.json',images)
    _,env=environment(compiler)
    invoke(out,'complete-test-batch',[sys.executable,'-B',str(Path(__file__).parent/'batch.py'),
           'run',compiler,str(deadline)],env,review,deadline)
    return json.loads((out/'totals.json').read_bytes())


'''
    native = native[:first]+replacement+native[last:]
    native = native.replace("    c.guard(review)\n    boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip()\n    deadline=time.monotonic()+1100",
        "    deadline=time.monotonic()+1100\n    c.guard(review)\n    boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip()")
    # Both originals are retained before these instrumentation-only replacements.
    (TOOLS/'common.py').write_text(common, encoding='utf8', newline='\n')
    (TOOLS/'native.py').write_text(native, encoding='utf8', newline='\n')
    NEW.mkdir(exist_ok=False)
    (NEW/'retirement-census.json').write_bytes(census.stdout)
    (NEW/'freeze').mkdir()
    (NEW/'seed').mkdir()
    for name in ('result.json', 'native-result.json'):
        value = json.loads((OLD/'seed'/name).read_bytes())
        c.need(value['passed'], 'reuse only complete checksum-bound offline seed')
        value.update(reused_from=str(OLD/'seed'/name),
                     reused_sha256=c.sha((OLD/'seed'/name).read_bytes()), phase='seed-reuse')
        c.write(NEW/'seed', name, value)
    names = set(review['helpers']) | {p.relative_to(c.ROOT).as_posix() for p in TOOLS.iterdir() if p.is_file()}
    helpers = {name: c.sha((c.ROOT/name).read_bytes()) for name in sorted(names)}
    result = dict(review, helpers=helpers,
        previous_freeze_sha256=c.sha((OLD/'freeze/runtime-reviewed.json').read_bytes()),
        first_execution_sha256=c.sha((OLD/'checks/execution.json').read_bytes()),
        first_refusal_sha256=c.sha((OLD/'checks/refusal.json').read_bytes()),
        helper_review='fresh kernel-confined source reads; one compiled-image batch frame; independent per-image custody/retirement',
        source_changed=False, profiles_changed=False, runtime_live_checks_changed=False,
        qualification_windows_changed=False, automatic_retry=False,
        seed_reused=True, original_seed=str(OLD/'seed'))
    c.write(NEW/'freeze', 'runtime-reviewed.json', result)
    c.need(c.hashes(result['source']) == result['source'], 'frozen actual root still exact')
    print(json.dumps(dict(context=str(NEW), source_changed=False, previous_refusal_retained=True)))


if __name__ == '__main__':
    main()
