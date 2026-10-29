"""Review the same first resolved lock offline, with exact primary feature proof."""
from pathlib import Path
import json
import tarfile
import importlib.util

import feature_review

spec = importlib.util.spec_from_file_location('native_transport_saved_preparation',
                                             Path(__file__).with_name('prepare.py'))
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)


def held_manifest(name):
    with tarfile.open(p.BASE/'input/archives'/(name+'.crate'), 'r:gz') as tar:
        return tar.extractfile(name+'/Cargo.toml').read()


def main():
    review = json.loads((p.BASE/'freeze/result.json').read_bytes())
    p.need(p.files(p.BASE/'input') == review['files'], 'whole first source input preserved')
    p.need(all(p.sha((p.ROOT/name).read_bytes()) == digest for name,digest in review['root_inputs'].items()),
           'protected root input unchanged')
    session = p.BASE/'session'
    original = json.loads((session/'native-result.json').read_bytes())
    phase = json.loads((session/'fd-194-check/result.json').read_bytes())
    execution = json.loads((session/'fd-194-check/execution.json').read_bytes())
    p.need(not original['passed'] and original['first_refusal']['reason'] ==
           'only old root dependency may disappear' and phase['exit'] == 0
           and original['source_unchanged_except_resolved_lock'] and original['boot_unchanged']
           and execution['retired'] and not execution['errors'], 'exact first resolver/audit outcome')
    manifests = dict(old=held_manifest('zbus-5.19.0'),tokio=held_manifest('tokio-1.53.3'),
        qualified=(p.PREVIOUS/'input/fd/source/zbus/Cargo.toml').read_bytes(),
        candidate=(p.BASE/'input/fd/source/zbus/Cargo.toml').read_bytes())
    result = feature_review.review((p.BASE/'input/fd/Cargo.lock').read_bytes(),
        (session/'Cargo.lock').read_bytes(),manifests['old'],manifests['tokio'],
        manifests['qualified'],manifests['candidate'])
    out=p.BASE/'feature-audit'
    out.mkdir(exist_ok=False)
    (out/'producer.py').write_bytes(Path(__file__).read_bytes())
    for name,raw in manifests.items():
        (out/(name+'-Cargo.toml')).write_bytes(raw)
    p.write(out,'result.json',dict(result,resolved_lock_sha256=p.sha((session/'Cargo.lock').read_bytes()),
                                 source_unchanged=True,passed=True,tests_executed=False))
    print(json.dumps(dict(offline_audit_passed=True,old_packages=result['old_packages'],
                         new_packages=result['new_packages'],new_cargo_run=False)))


if __name__ == '__main__':
    main()
