"""Compose the existing pinned nonroot CI owner; no system bus or host authority."""
from pathlib import Path
import hashlib,importlib.util,sys
ROOT=Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
OWNER_SHA='09ab4fb404b93af4ec5581b189d3e7fe54562fff14626bfabcbc07c11c9f51b1'

def configured(attempt,entry):
    if type(attempt) is not int or attempt not in (1,2,3,4):raise ValueError('fixed context')
    source=ROOT/'tools/dbus-linux-074/native.py'
    if hashlib.sha256(source.read_bytes()).hexdigest()!=OWNER_SHA:raise ValueError('original CI owner drift')
    spec=importlib.util.spec_from_file_location('hc074_fragment_owner',source)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    module.BASE=ROOT/'target/dbus-fragments-20261028'/{1:'baseline',2:'baseline-corrected',3:'fixed',4:'fixed-io-safe'}[attempt]
    module.NATIVE=Path('/home/artur/.cache/hydracache-074-dbus-fragments')/{1:'20261028-b1',2:'20261028-b2',3:'20261028-b3',4:'20261028-b4'}[attempt]
    module.OUT=module.BASE/'session'
    # The original session uses its __file__ only for its own fixed worker argv.
    # An entry per context carries that same context into the disposable worker.
    module.__file__=str(entry)
    return module

def main(attempt,entry):
    owner=configured(attempt,entry)
    if len(sys.argv)==2 and sys.argv[1]=='session':owner.session()
    elif len(sys.argv)==4 and sys.argv[1]=='worker-v5':owner.worker(sys.argv[2],float(sys.argv[3]))
    else:raise ValueError('fixed session/worker only')

if __name__=='__main__':main(1,Path(__file__))
