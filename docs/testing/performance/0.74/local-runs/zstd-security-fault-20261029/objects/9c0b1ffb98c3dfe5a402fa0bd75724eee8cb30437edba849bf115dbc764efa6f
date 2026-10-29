"""Fresh complete catalogs after a preserved capacity stop; no source/limit change.

Only complete immutable format/compiler/focus receipts are carried forward and
explicitly labeled as reused. No partial test result or case count is copied.
The new full catalogs start at zero under the unchanged four-GiB floor.
"""
from pathlib import Path
import json
import shutil
import subprocess

import common as c

OLD = c.ROOT/'target/dbus-root-package-20261029-hash5'
NEW = c.ROOT/'target/dbus-root-package-20261029-resource6'
TOOLS = Path(__file__).parent


def main():
    c.need(c.BASE==OLD and not NEW.exists(),'one explicit corrected-resource context')
    review = json.loads((OLD/'freeze/runtime-reviewed.json').read_bytes())
    c.guard(review)
    first = json.loads((OLD/'tests-194/execution.json').read_bytes())
    c.need(first['classification']=='resource-floor' and first['retired'] and first['exit']==125,
           'preserve exact first capacity stop; never treat it as a package pass')
    cleanup_path = c.ROOT/'target/dbus-root-package-20261028/cleanup-tagged-objects/result.json'
    cleanup = json.loads(cleanup_path.read_bytes())
    c.need(cleanup['removed']==6832 and cleanup['bytes']==12035674647
           and cleanup['sources_preserved'] and cleanup['evidence_preserved']
           and cleanup['executables_preserved'] and cleanup['other_worktrees_untouched'],
           'concrete changed resource condition, never unchanged retry-to-pass')
    c.capacity()
    census = subprocess.run(['wsl.exe','-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B',
        '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-root-package-074/retirement.py'],
        stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=20)
    c.need(census.returncode==0 and not json.loads(census.stdout)['active'],'old native attempt fully retired')
    complete = {}
    for phase in ('format','checks','focus-194','focus-188'):
        folder = OLD/phase
        result = json.loads((folder/'result.json').read_bytes())
        native = json.loads((folder/'native-result.json').read_bytes())
        execution = json.loads((folder/'execution.json').read_bytes())
        c.need(result['passed'] and native['passed'] and native['source_unchanged'] and native['boot_unchanged']
               and execution['retired'] and not execution['errors'] and execution['exit']==0,
               'reuse only complete actual gate on this identical source: '+phase)
        complete[phase] = {name:c.sha((folder/name).read_bytes())
                          for name in ('result.json','native-result.json','execution.json','intent.json')}
    saved = OLD/'helper-before-resource6'
    saved.mkdir(exist_ok=False)
    for name in review['helpers']:
        if name.startswith('tools/dbus-root-package-074/'):
            (saved/Path(name).name).write_bytes((c.ROOT/name).read_bytes())
    common = (TOOLS/'common.py').read_text()
    before = "BASE = ROOT / 'target/dbus-root-package-20261029-hash5'"
    c.need(common.count(before)==1,'exact output-scope-only helper change')
    common = common.replace(before,"BASE = ROOT / 'target/dbus-root-package-20261029-resource6'")
    retention = (TOOLS/'retain.py').read_text()
    before = "(c.BASE, 'hash5')):"
    c.need(retention.count(before)==1,'exact earlier evidence scope')
    retention = retention.replace(before,
        "(c.ROOT/'target/dbus-root-package-20261029-hash5', 'hash-leaf-first'), (c.BASE, 'resource6')):")
    before = 'original_refusals_retained=True, source_worktree_uncommitted=True,'
    c.need(retention.count(before)==1,'exact receipt semantics')
    retention = retention.replace(before,before+
        "\n        complete_gate_reuse=review.get('complete_gate_reuse',{}), partial_case_counts_reused=False,")
    (TOOLS/'common.py').write_text(common,encoding='utf8',newline='\n')
    (TOOLS/'retain.py').write_text(retention,encoding='utf8',newline='\n')
    NEW.mkdir(exist_ok=False)
    (NEW/'freeze').mkdir()
    (NEW/'seed').mkdir()
    (NEW/'retirement-census.json').write_bytes(census.stdout)
    reused = {}
    pinned = []
    for phase,files in complete.items():
        folder = NEW/phase
        folder.mkdir()
        for name,digest in files.items():
            raw = (OLD/phase/name).read_bytes()
            c.need(c.sha(raw)==digest,'unchanged original complete gate receipt')
            (folder/name).write_bytes(raw)
            pinned.append((folder/name).relative_to(c.ROOT).as_posix())
        reused[phase] = dict(from_context=str(OLD),receipt_sha256=files,executed_again=False,
                            same_product_source=True,live_authority_reused=False)
        c.write(folder,'reuse.json',reused[phase])
        pinned.append((folder/'reuse.json').relative_to(c.ROOT).as_posix())
    for name in ('result.json','native-result.json'):
        original = c.ROOT/'target/dbus-root-package-20261028/seed'/name
        value = json.loads(original.read_bytes())
        c.need(value['passed'],'verified content seed only')
        value.update(reused_from=str(original),reused_sha256=c.sha(original.read_bytes()),phase='seed-reuse')
        c.write(NEW/'seed',name,value)
    names = set(review['helpers']) | {p.relative_to(c.ROOT).as_posix() for p in TOOLS.iterdir() if p.is_file()} | set(pinned)
    helpers = {name:c.sha((c.ROOT/name).read_bytes()) for name in sorted(names)}
    receipt = dict(review,helpers=helpers,complete_gate_reuse=reused,
        previous_freeze_sha256=c.sha((OLD/'freeze/runtime-reviewed.json').read_bytes()),
        first_capacity_stop_sha256=c.sha((OLD/'tests-194/execution.json').read_bytes()),
        resource_cleanup_sha256=c.sha(cleanup_path.read_bytes()),
        helper_review='new output scope and receipt bookkeeping only; exact product/test/command/profile bytes unchanged',
        product_source_changed=False,commands_changed=False,profiles_changed=False,
        original_floor_bytes=4*1024**3,physical_free_before=shutil.disk_usage(c.ROOT).free,
        fresh_full_catalogs_required=['tests-194','tests-188'],partial_tests_reused=False,
        qualification_windows_changed=False,automatic_retry=False)
    c.write(NEW/'freeze','runtime-reviewed.json',receipt)
    c.need(c.hashes(receipt['source'])==receipt['source'],'identical complete product source after capacity correction')
    print(json.dumps(dict(context=str(NEW),product_source_changed=False,partial_tests_reused=False,
        complete_gates_reused=list(reused),physical_free_before=receipt['physical_free_before'])))


if __name__=='__main__':
    main()
