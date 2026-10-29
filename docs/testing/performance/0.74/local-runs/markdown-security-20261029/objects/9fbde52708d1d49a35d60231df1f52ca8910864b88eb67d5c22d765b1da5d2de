"""Seal pre-execution CI deadline hardening without changing the frozen root/code/profile."""
from pathlib import Path
import json

import common as c


def main():
    original=json.loads((c.BASE/'freeze/result.json').read_bytes())
    c.need(c.hashes(original['source'])==original['source'],'exact already frozen root source')
    c.need(not any((c.BASE/name).exists() for name in original['phases']),'no package phase consumed before helper review')
    helpers=c.hashes([p.relative_to(c.ROOT).as_posix() for p in Path(__file__).parent.iterdir() if p.is_file()])
    for name in original['helpers']:
        if name not in helpers:helpers[name]=c.sha((c.ROOT/name).read_bytes())
    allowed={'tools/dbus-root-package-074/'+n for n in ('native.py','run.py','test_native.py')}
    changed={n for n in original['helpers'] if helpers[n]!=original['helpers'][n]}
    c.need(changed.issubset(allowed),'only reviewed CI deadline helper changes')
    for name in changed:
        previous=c.BASE/'helper-review-before'/Path(name).name
        c.need(c.sha(previous.read_bytes())==original['helpers'][name],'old pre-execution helper retained')
    receipt=dict(original,helpers=helpers,previous_freeze_sha256=c.sha((c.BASE/'freeze/result.json').read_bytes()),
        helper_review='enforce existing seed/postflight/spawn deadlines before any package execution',
        changed_helpers=sorted(changed),root_source_changed=False,profiles_changed=False)
    c.write(c.BASE/'freeze','runtime-reviewed.json',receipt)
    print(json.dumps(dict(prepared=True,root_source_changed=False,changed_helpers=len(changed))))


if __name__=='__main__':main()
