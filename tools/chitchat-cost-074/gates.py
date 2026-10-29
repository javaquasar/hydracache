"""Fixed nonmeasurement offline gates under original ownership and bounds."""
from pathlib import Path
import hashlib

old=Path(__file__).resolve().parents[1]/'chitchat-stateful-074/gates.py'
raw=old.read_bytes();assert hashlib.sha256(raw).hexdigest()=='af294eea27bd7f1d45249da9d2cb321255699d4c2dbbc9662e348fdd52a1281b'
text=raw.decode()
text=text.replace("str(c.TOOLS/'stateful.rs')","str(c.TOOLS/'probe.rs'),*[str(c.BASE/('probe-'+v+'.rs')) for v in ('B','P','Q')]")
text=text.replace('chitchat-stateful-result-20261029.md','chitchat-cost-result-20261029.md')
assert text.count('c.need(mode in commands')==1
text=text.replace('c.need(mode in commands',"commands['controls-final']=commands['controls']\nc.need(mode in commands")
exec(compile(text,__file__+'[clean-cost-owned-checks]','exec'),globals())
