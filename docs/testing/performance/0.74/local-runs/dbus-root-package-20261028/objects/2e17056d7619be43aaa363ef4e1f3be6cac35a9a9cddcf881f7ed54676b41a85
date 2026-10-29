"""Restore the actual Cargo/owned-case caller and bind all discovered inputs."""
from pathlib import Path
import json
import subprocess

import common as c

OLD = c.ROOT/'target/dbus-root-package-20261028-cases3'
NEW = c.ROOT/'target/dbus-root-package-20261028-caller4'
TOOLS = Path(__file__).parent


def replace(body, before, after):
    c.need(body.count(before) == 1, 'exact reviewed CI helper preimage: '+before[:65])
    return body.replace(before, after)


def main():
    c.need(c.BASE == OLD and not NEW.exists(), 'one new original-caller context')
    review = json.loads((OLD/'freeze/runtime-reviewed.json').read_bytes())
    execution = json.loads((OLD/'tests-194/execution.json').read_bytes())
    c.need(execution['retired'] and not execution['errors'] and execution['exit'] != 0,
           'retain first held-entry integration refusal')
    c.guard(review)
    audit = json.loads((OLD/'caller-compatibility-review/result.json').read_bytes())
    c.need(not audit['unresolved'] and len(audit['extra']) == 13, 'complete reviewed literal external Rust/data inputs')
    baseline = (OLD/'caller-first.stderr.log').read_text()
    c.need('Ran 2 tests' in baseline and 'FAILED (failures=2)' in baseline,
           'caller-contract regressions reproduced before implementation')
    census = subprocess.run(['wsl.exe','-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B',
        '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-root-package-074/retirement.py'],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=20)
    c.need(census.returncode == 0 and not json.loads(census.stdout)['active'], 'previous context completely retired')
    saved = OLD/'helper-before-caller4'
    saved.mkdir(exist_ok=False)
    for name in review['helpers']:
        if name.startswith('tools/dbus-root-package-074/'):
            (saved/Path(name).name).write_bytes((c.ROOT/name).read_bytes())
    common = replace((TOOLS/'common.py').read_text(),
        "BASE = ROOT / 'target/dbus-root-package-20261028-cases3'",
        "BASE = ROOT / 'target/dbus-root-package-20261028-caller4'")
    old_phases = "('seed','checks','tests-194','tests-188')"
    new_phases = "('seed','checks','focus-194','focus-188','tests-194','tests-188')"
    native = replace((TOOLS/'native.py').read_text(), old_phases, new_phases)
    native = replace(native,
        "c.write(folder,'intent.json',dict(argv=argv,environment=env,deadline=deadline,bound=1200,session_bound=1100,",
        "c.write(folder,'intent.json',dict(argv=argv,environment=env,deadline=deadline,bound=1200,session_bound=1100,\n        cwd=str(c.ROOT/c.PKG) if argv[0].startswith(str(NATIVE/'builds')+'/') else str(c.ROOT),")
    native = replace(native,
        "execution=c.owned()._owned(group.LinuxGroup(),intent['argv'],str(c.ROOT),capacity,",
        "c.need(intent['cwd'] in (str(c.ROOT),str(c.ROOT/c.PKG)),'fixed original Cargo/test working directory')\n        execution=c.owned()._owned(group.LinuxGroup(),intent['argv'],intent['cwd'],capacity,")
    focus = '''def focus(compiler,review,out,deadline):
    folder=invoke(out,'build',*cargo_command(compiler,'build'),review,deadline)
    images=artifacts((folder/'stdout.log').read_bytes(),review,compiler)
    c.write(out,'images.json',images)
    selected=[v for v in images if v['kind']==['test'] and v['name']=='bootstrap_current_image']
    c.need(len(selected)==1,'one actual original held-entry integration image')
    image=selected[0];path=Path(image['path']);_,env=environment(compiler)
    listed=invoke(out,'bootstrap-list',[str(path),'--list'],env,review,deadline)
    catalog=sorted(line[:-6] for line in (listed/'stdout.log').read_text().splitlines() if line.endswith(': test'))
    c.need(len(catalog)==len(set(catalog))==17,'all original bootstrap integration cases')
    c.need(c.sha(path.read_bytes())==image['sha256'],'original focused image custody')
    tested=invoke(out,'bootstrap-full',[str(path),'--test-threads=1'],env,review,deadline)
    value=account((tested/'stdout.log').read_bytes(),catalog)
    c.write(out,'accounting.json',value)
    c.write(out,'catalog.json',catalog)
    c.need(c.sha(path.read_bytes())==image['sha256'],'original focused image postflight')
    return value


'''
    native = replace(native, 'def session(phase):', focus+'def session(phase):')
    native = replace(native,
        "result['tests']=tests(phase.split('-')[1],review,out,deadline)",
        "if phase.startswith('focus-'):result['tests']=focus(phase.split('-')[1],review,out,deadline)\n                else:\n                    for compiler in ('194','188'):\n                        c.need(json.loads((c.BASE/('focus-'+compiler)/'native-result.json').read_bytes())['passed'],'both original-caller bootstrap focuses before full catalogs')\n                    result['tests']=tests(phase.split('-')[1],review,out,deadline)")
    run = replace((TOOLS/'run.py').read_text(), old_phases, new_phases)
    test_native = replace((TOOLS/'test_native.py').read_text(), old_phases, new_phases)
    batch = replace((TOOLS/'batch.py').read_text(),
        'bound=1200, session_bound=1100, floor=4*1024**3, poll=5, retirement=5, retry=False))',
        'bound=1200, session_bound=1100, floor=4*1024**3, poll=5, retirement=5, retry=False, cwd=str(c.ROOT/c.PKG)))')
    batch = replace(batch,
        "execution = c.owned()._owned(group.LinuxGroup(), intent['argv'], str(c.ROOT), n.capacity,",
        "c.need(intent['cwd']==str(c.ROOT/c.PKG),'original Cargo test working directory')\n        execution = c.owned()._owned(group.LinuxGroup(), intent['argv'], intent['cwd'], n.capacity,")
    isolated = replace((TOOLS/'isolated.py').read_text(),
        "[str(path), '--exact', selected, '--test-threads=1'], env, deadline)",
        "[str(path), '--exact', selected, '--test-threads=1', '--color=never'], env, deadline)")
    isolated = replace(isolated, '        if catalog:\n            value = cases(',
        "        if catalog and image['kind']==['lib']:\n            value = cases(")
    isolated = replace(isolated,
        "tested = b.invoke(out, prefix+'-empty', [str(path), '--test-threads=1'], env, deadline)\n            value = n.account((tested/'stdout.log').read_bytes(), [])",
        "tested = b.invoke(out, prefix+'-test', [str(path), '--test-threads=1'], env, deadline)\n            value = n.account((tested/'stdout.log').read_bytes(), catalog)")
    retention = replace((TOOLS/'retain.py').read_text(),
        "for phase in ('checks','tests-194','tests-188'):",
        "for phase in ('checks','focus-194','focus-188','tests-194','tests-188'):")
    retention = replace(retention,
        "(c.BASE, 'cases3')):",
        "(c.ROOT/'target/dbus-root-package-20261028-cases3', 'case-entry-first'), (c.BASE, 'caller4')):")
    retention = replace(retention, 'metadata_targets=90, source_files=2232, registry_identities=644,',
        "metadata_targets=90, source_files=len(review['source']), registry_identities=644,\n        original_owned_case_frame_preserved=True, integration_harness='unfiltered serial', extra_compile_inputs=13,")
    for name, body in (('common.py',common),('native.py',native),('run.py',run),('batch.py',batch),
                       ('isolated.py',isolated),('test_native.py',test_native),('retain.py',retention)):
        (TOOLS/name).write_text(body, encoding='utf8', newline='\n')
    NEW.mkdir(exist_ok=False)
    (NEW/'freeze').mkdir()
    (NEW/'seed').mkdir()
    (NEW/'retirement-census.json').write_bytes(census.stdout)
    for name in ('result.json','native-result.json'):
        original = c.ROOT/'target/dbus-root-package-20261028/seed'/name
        value = json.loads(original.read_bytes())
        c.need(value['passed'], 'immutable verified content seed only')
        value.update(reused_from=str(original), reused_sha256=c.sha(original.read_bytes()), phase='seed-reuse')
        c.write(NEW/'seed',name,value)
    source = dict(review['source'])
    for name, value in audit['extra'].items():
        c.need(c.sha((c.ROOT/name).read_bytes())==value['raw_sha256'],'unchanged reviewed additional input')
        source[name]=value['raw_sha256']
    names = set(review['helpers']) | {p.relative_to(c.ROOT).as_posix() for p in TOOLS.iterdir() if p.is_file()}
    helpers = {name:c.sha((c.ROOT/name).read_bytes()) for name in sorted(names)}
    receipt = dict(review, source=source, helpers=helpers,
        previous_freeze_sha256=c.sha((OLD/'freeze/runtime-reviewed.json').read_bytes()),
        first_integration_refusal_sha256=c.sha((OLD/'tests-194/native-result.json').read_bytes()),
        caller_review_sha256=c.sha((OLD/'caller-compatibility-review/result.json').read_bytes()),
        helper_review='restore original owned-case outer frame; serial unfiltered integrations; original manifest cwd; bind external literal inputs',
        source_changed=False, additional_source_inputs=audit['extra'], product_checks_changed=False,
        original_owned_case_seconds=45, original_cleanup_seconds=5,
        original_activation_seconds=10, original_child_wait_seconds=5,
        focus_targets=['bootstrap_current_image'], partial_tests_reused=False,
        phases=['seed','checks','focus-194','focus-188','tests-194','tests-188'])
    c.write(NEW/'freeze','runtime-reviewed.json',receipt)
    c.need(c.hashes(source)==source and len(source)==2245,'expanded exact source/input receipt')
    print(json.dumps(dict(context=str(NEW),source_files=len(source),product_source_changed=False,partial_tests_reused=False)))


if __name__ == '__main__':
    main()
