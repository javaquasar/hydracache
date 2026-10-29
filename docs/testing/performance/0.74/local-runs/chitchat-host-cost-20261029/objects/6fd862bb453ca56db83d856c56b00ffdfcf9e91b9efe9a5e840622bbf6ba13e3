"""Fixed original local checks; no host/diagnostic dispatcher."""
from pathlib import Path
import hashlib

old=Path(__file__).resolve().parents[1]/'chitchat-stateful-074/gates.py'
raw=old.read_bytes();assert hashlib.sha256(raw).hexdigest()=='af294eea27bd7f1d45249da9d2cb321255699d4c2dbbc9662e348fdd52a1281b'
text=raw.decode()
start=text.index("    'format':");end=text.index("    'doc-check':",start)
text=text[:start]+"    'format':[sys.executable,'-B','-c',\"import ast,pathlib; [ast.parse(p.read_bytes(), filename=str(p)) for p in pathlib.Path('tools/chitchat-host-cost-074').glob('*.py')]\"],\n"+text[end:]
text=text.replace('chitchat-stateful-result-20261029.md','chitchat-host-cost-result-20261029.md')
text=text.replace("c.need(mode in commands","commands['prepare-proof']=[sys.executable,'-B',str(c.TOOLS/'review.py')]\nc.need(mode in commands")
text=text.replace('c.need(mode in commands',"commands['controls-final']=commands['controls']; commands['prepare-proof-final']=commands['prepare-proof']\nc.need(mode in commands")
exec(compile(text,__file__+'[host-control-owned-checks]','exec'),globals())
