"""Build an exact extracted parser control with no Cargo dependencies or live input."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'tools/long-run-supervisor-074/src'
OUT = ROOT / 'target/status-field-scan-20261024'
BASE_SHA = '5f77f86c3c29a42156d63ffa913583f8a8c6419562f6c2bea5c34e75937260fa'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def extract(raw):
    text = raw.decode('utf-8')
    start = '#[derive(Debug, PartialEq, Eq)]\nstruct Projection {'
    end = '\nstruct Sender {'
    if text.count(start) != 1 or text.count(end) != 1:
        raise ValueError('Ambiguous parser boundaries')
    return (text[text.index(start):text.index(end)] + '\n').encode()


def build(lane):
    if lane not in ('baseline', 'candidate'):
        raise ValueError('Choose a separate baseline/candidate build')
    raw = (SOURCE / 'diagnostic_kernel_admission.rs').read_bytes().replace(b'\r\n', b'\n')
    if lane == 'baseline' and sha(raw) != BASE_SHA:
        raise ValueError('Baseline runtime drift')
    output = OUT / lane
    output.mkdir(parents=True, exist_ok=False)
    sources = {'main.rs': Path(__file__).with_name('main.rs').read_bytes(),
               'projection.rs': extract(raw),
               'projection_groups.rs': (SOURCE / 'diagnostic_projection_groups.rs').read_bytes()}
    fields = SOURCE / 'diagnostic_projection_fields.rs'
    if fields.exists():
        sources['projection_fields.rs'] = fields.read_bytes()
        sources['projection.rs'] = b'#[path="projection_fields.rs"] mod projection_fields;\n' + sources['projection.rs']
    for name, content in sources.items():
        with (output / name).open('xb') as stream:
            stream.write(content)
    wsl_path = '/mnt/' + output.drive[0].lower() + output.as_posix()[2:]
    argv = ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--cd', '/', '--exec', '/usr/bin/env',
            'RUSTUP_AUTO_INSTALL=0', '/home/artur/.cargo/bin/rustc', '+1.94.0', '--edition=2021',
            '-C', 'opt-level=3', '-C', 'codegen-units=1', '-D', 'warnings',
            wsl_path + '/main.rs', '-o', wsl_path + '/parser']
    intent = dict(lane=lane, argv=argv, source_sha256=sha(raw),
                  inputs={name: sha(content) for name, content in sources.items()},
                  runtime=False, live=False, server=False, admission=False, retry=False)
    with (output / 'intent.json').open('x') as stream:
        json.dump(intent, stream, indent=2)
    result = subprocess.run(argv, capture_output=True, timeout=120, check=False)
    for name, data in (('stdout.log', result.stdout), ('stderr.log', result.stderr)):
        with (output / name).open('xb') as stream:
            stream.write(data)
    receipt = dict(exit=result.returncode, image_sha256=sha((output / 'parser').read_bytes()) if result.returncode == 0 else None)
    with (output / 'result.json').open('x') as stream:
        json.dump(receipt, stream, indent=2)
    print(receipt)
    if result.returncode:
        print(result.stderr.decode(errors='replace'))
        raise SystemExit(result.returncode)


if __name__ == '__main__':
    build(sys.argv[1])
