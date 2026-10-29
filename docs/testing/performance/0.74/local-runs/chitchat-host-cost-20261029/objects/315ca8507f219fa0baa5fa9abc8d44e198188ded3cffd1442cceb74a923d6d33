"""Byte-pinned derivative of the qualified nonmeasurement owned checks."""
from pathlib import Path
import hashlib

old=Path(__file__).resolve().parents[1]/'chitchat-stateful-074/gates.py'
raw=old.read_bytes();assert hashlib.sha256(raw).hexdigest()=='af294eea27bd7f1d45249da9d2cb321255699d4c2dbbc9662e348fdd52a1281b'
text=raw.decode();assert text.count("str(c.TOOLS/'stateful.rs')")==1
text=text.replace("str(c.TOOLS/'stateful.rs')","str(c.TOOLS/'caller.rs')")
assert text.count('chitchat-stateful-result-20261029.md')==1
text=text.replace('chitchat-stateful-result-20261029.md','chitchat-caller-result-20261029.md')
exec(compile(text,__file__+'[owned-check-derivative]','exec'),globals())
