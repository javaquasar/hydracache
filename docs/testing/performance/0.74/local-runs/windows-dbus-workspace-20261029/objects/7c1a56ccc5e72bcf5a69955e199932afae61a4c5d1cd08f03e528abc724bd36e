"""Unchanged owned nonroot GNU gate, now consuming reconstructed source archives."""
from pathlib import Path
import hashlib
import importlib.util
import sys

ROOT = Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
OWNER_SHA = '09ab4fb404b93af4ec5581b189d3e7fe54562fff14626bfabcbc07c11c9f51b1'


def configured(root=ROOT):
    source = root / 'tools/dbus-linux-074/native.py'
    if hashlib.sha256(source.read_bytes()).hexdigest() != OWNER_SHA:
        raise ValueError('original CI owner drift')
    spec = importlib.util.spec_from_file_location('hc074_source_package_owner', source)
    owner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    owner.BASE = root / 'target/dbus-source-package-20261028'
    owner.NATIVE = Path('/home/artur/.cache/hydracache-074-dbus-source-package/20261028-a1')
    owner.OUT = owner.BASE / 'session'
    owner.__file__ = str(Path(__file__))
    return owner


if __name__ == '__main__':
    owner = configured()
    if len(sys.argv) == 2 and sys.argv[1] == 'session':
        owner.session()
    elif len(sys.argv) == 4 and sys.argv[1] == 'worker-v5':
        owner.worker(sys.argv[2], float(sys.argv[3]))
    else:
        raise ValueError('fixed session/worker only')
