"""Offline complete payload/source/effect review; no dispatch or archived exec."""
import hashlib
import json
from pathlib import Path
import tomllib
import host
import prepare
import remote


def review():
    root=prepare.OUT/'prepared/input';state=json.loads((prepare.OUT/'preflight.stdout.log').read_bytes());host.admit(state)
    proof=json.loads((prepare.OUT/'prepared/source-review.json').read_bytes())
    frame=json.loads((root/'frame.json').read_bytes())
    prepare.need(len(frame)==303 and prepare.sha((root/'frame.json').read_bytes())==proof['frame_sha256'],'original source frame')
    for name,digest in frame.items():prepare.need(prepare.sha((root/'source'/name).read_bytes())==digest,'fresh original source '+name)
    for name in ('Cargo.toml','Cargo.lock','.cargo/config.toml','deny.toml','docs/testing/performance/0.74/qualification-manifest.toml',
        'scripts/ci/watchdog_linux_group.py','scripts/ci/watchdog_windows_job.py','tools/windows-workspace-preflight-074/owned.py'):
        prepare.need(prepare.sha((prepare.ROOT/name).read_bytes())==frame[name],'unchanged actual Root/helper pin '+name)
    for name in ('host.py','remote.py'):prepare.need((root/name).read_bytes()==(prepare.HERE/name).read_bytes(),'reviewed current host producer')
    prepare.need((root/'owned_function.py').read_bytes()==prepare.function((prepare.ROOT/'tools/windows-workspace-preflight-074/owned.py').read_bytes(),'_owned'),'unchanged owned helper')
    prepare.need((root/'validate_function.py').read_bytes()==prepare.function((prepare.ROOT/'tools/chitchat-cost-074/analyze.py').read_bytes(),'validate'),'all original predicates/order/limits')
    prepare.need((root/'protocol.json').read_bytes()==(prepare.ROOT/'tools/chitchat-cost-074/protocol.json').read_bytes(),'fixed preregistration')
    native=json.loads((prepare.OUT/'prepared/native-export.json').read_bytes())
    for variant,row in native['images'].items():prepare.need(remote.sha(root/'images'/variant)==row['sha256']==proof['images'][variant]['sha256'],'actual original executable')
    crates=native['vendor'];prepare.need(len(crates)==91,'complete actual registry')
    for name,files in crates.items():
        expected=dict(files);expected['.cargo-checksum.json']=prepare.sha((json.dumps(dict(package=native['checksums'][name],files=files),indent=2,sort_keys=True)+'\n').encode())
        prepare.need(remote.inventory(root/'vendor'/name)==expected,'fresh original complete registry source '+name)
    prepare.need(len(json.loads((root/'semantics/input/expected-caller.json').read_bytes()))==30,'full original semantic corpus')
    return dict(complete=True,controls_only=True,source_files=303,registry_crates=91,images=3,
        predicates_changed=False,protocol_changed=False,root_adopted=False,exclusive_host=False,CPU_qualified=False,RSS_qualified=False)


if __name__=='__main__':print(json.dumps(review()))
