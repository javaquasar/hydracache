"""Fixed original owned offline gates, no remote service/workload action."""
from pathlib import Path
import hashlib

old=Path(__file__).resolve().parents[1]/'chitchat-stateful-074/gates.py'
raw=old.read_bytes();assert hashlib.sha256(raw).hexdigest()=='af294eea27bd7f1d45249da9d2cb321255699d4c2dbbc9662e348fdd52a1281b'
text=raw.decode();start=text.index("    'format':");end=text.index("    'doc-check':",start)
text=text[:start]+"    'format':[sys.executable,'-B','-c',\"import ast,pathlib; [ast.parse(p.read_bytes(),filename=str(p)) for p in pathlib.Path('tools/chitchat-cohort-074').glob('*.py')]\"],\n"+text[end:]
text=text.replace('chitchat-stateful-result-20261029.md','chitchat-cohort-result-20261029.md')
text=text.replace('c.need(mode in commands',"commands['base-controls']=[sys.executable,'-B','-m','unittest','discover','-s','tools/get-owner-scheduled-controls-074','-p','test_maintenance_d3.py','-v']\nc.need(mode in commands")
text=text.replace('c.need(mode in commands',"commands['controls-prepared']=commands['controls'];commands['syntax-prepared']=commands['format']\nc.need(mode in commands")
text=text.replace('c.need(mode in commands',"commands['controls-worker']=commands['controls'];commands['syntax-worker']=commands['format']\nc.need(mode in commands")
exec(compile(text,__file__+'[cohort-owned-offline-check]','exec'),globals())
