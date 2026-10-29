"""Seal fresh case-isolated full-catalog context; never reuse partial tests."""
from pathlib import Path
import json
import subprocess

import common as c

OLD = c.ROOT/'target/dbus-root-package-20261028-custody2'
NEW = c.ROOT/'target/dbus-root-package-20261028-cases3'
TOOLS = Path(__file__).parent


def main():
    c.need(c.BASE == OLD and not NEW.exists(), 'one new complete case-isolated context')
    review = json.loads((OLD/'freeze/runtime-reviewed.json').read_bytes())
    execution = json.loads((OLD/'tests-194/execution.json').read_bytes())
    c.need(execution['retired'] and not execution['errors'] and execution['exit'] == 124
           and (OLD/'tests-194/refusal.json').is_file(), 'retain terminal first serial catalog timeout')
    c.guard(review)
    inert = (OLD/'isolation-inert.stderr.log').read_text()
    c.need('Ran 2 tests' in inert and '\nOK\n' in inert, 'exact-selection and full-catalog controls passed')
    census = subprocess.run(['wsl.exe','-d','Ubuntu','-u','artur','--cd','/','--exec',
        '/usr/bin/python3','-B',
        '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-root-package-074/retirement.py'],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=20)
    c.need(census.returncode == 0 and not json.loads(census.stdout)['active'], 'all first-session processes retired')
    saved = OLD/'helper-before-cases3'
    saved.mkdir(exist_ok=False)
    for name in review['helpers']:
        if name.startswith('tools/dbus-root-package-074/'):
            (saved/Path(name).name).write_bytes((c.ROOT/name).read_bytes())
    common = (TOOLS/'common.py').read_text()
    before = "BASE = ROOT / 'target/dbus-root-package-20261028-custody2'"
    c.need(common.count(before) == 1, 'exact prior context')
    common = common.replace(before, "BASE = ROOT / 'target/dbus-root-package-20261028-cases3'")
    native = (TOOLS/'native.py').read_text()
    before = "str(Path(__file__).parent/'batch.py')"
    c.need(native.count(before) == 1, 'exact prior full test-batch dispatcher')
    native = native.replace(before, "str(Path(__file__).parent/'isolated.py')")
    # Tighten only the existing CI deadline before actual execution, after custody.
    before = "        execution=c.owned()._owned(group.LinuxGroup(),intent['argv'],str(c.ROOT),capacity,"
    c.need(native.count(before) == 1, 'exact original CI owner call')
    native = native.replace(before, "        c.need(time.monotonic()<intent['deadline'],'existing CI deadline before owner spawn')\n"+before)
    batch = (TOOLS/'batch.py').read_text()
    before = "    n.capacity()\n    folder = out / label"
    c.need(batch.count(before) == 1, 'exact original per-case preflight')
    batch = batch.replace(before, "    n.capacity()\n    c.need(time.monotonic() < deadline, 'original deadline after per-case preflight')\n    folder = out / label")
    before = "        execution = c.owned()._owned(group.LinuxGroup(), intent['argv'], str(c.ROOT), n.capacity,"
    c.need(batch.count(before) == 1, 'exact independent original per-case owner')
    batch = batch.replace(before, "        c.need(time.monotonic() < intent['deadline'], 'existing case deadline before owner spawn')\n"+before)
    retirement = (TOOLS/'retirement.py').read_text()
    retirement = retirement.replace("prefix+'native.py', prefix+'batch.py'", "prefix+'native.py', prefix+'batch.py', prefix+'isolated.py'")
    retention = (TOOLS/'retain.py').read_text()
    before = "for scope, prefix in ((OLD, 'first-context'), (c.BASE, 'custody2')):"
    c.need(retention.count(before) == 1, 'exact prior evidence collection scope')
    retention = retention.replace(before,
        "for scope, prefix in ((OLD, 'first-context'), (c.ROOT/'target/dbus-root-package-20261028-custody2', 'serial-context'), (c.BASE, 'cases3')):")
    retention = retention.replace('full_default_package_tests=True,',
        "full_case_isolated_default_catalog=True, monolithic_serial_complete=False, case_processes=4, case_threads=1,")
    for name, body in (('common.py',common),('native.py',native),('batch.py',batch),('retirement.py',retirement),('retain.py',retention)):
        (TOOLS/name).write_text(body, encoding='utf8', newline='\n')
    NEW.mkdir(exist_ok=False)
    (NEW/'freeze').mkdir()
    (NEW/'seed').mkdir()
    (NEW/'retirement-census.json').write_bytes(census.stdout)
    for name in ('result.json','native-result.json'):
        original = c.ROOT/'target/dbus-root-package-20261028/seed'/name
        value = json.loads(original.read_bytes())
        c.need(value['passed'], 'reuse only complete content seed, never test/clock authority')
        value.update(reused_from=str(original), reused_sha256=c.sha(original.read_bytes()), phase='seed-reuse')
        c.write(NEW/'seed', name, value)
    names = set(review['helpers']) | {p.relative_to(c.ROOT).as_posix() for p in TOOLS.iterdir() if p.is_file()}
    helpers = {name:c.sha((c.ROOT/name).read_bytes()) for name in sorted(names)}
    result = dict(review, helpers=helpers,
        previous_freeze_sha256=c.sha((OLD/'freeze/runtime-reviewed.json').read_bytes()),
        first_serial_timeout_sha256=c.sha((OLD/'tests-194/execution.json').read_bytes()),
        helper_review='four independent FD/process tables; all original cases once in fresh full catalogs; no partial result reuse',
        case_processes=4, case_threads=1, local_selection_filtered='catalog size minus one, globally zero omitted',
        source_changed=False, profiles_changed=False, runtime_live_checks_changed=False,
        qualification_windows_changed=False, automatic_retry=False, partial_tests_reused=False,
        source_inventory_scope='complete compiled-image batch boundaries plus original full source/frame checks')
    c.write(NEW/'freeze','runtime-reviewed.json',result)
    c.need(c.hashes(result['source']) == result['source'], 'exact original applied/formatted source')
    print(json.dumps(dict(context=str(NEW), source_changed=False, partial_tests_reused=False)))


if __name__ == '__main__':
    main()
