"""Disjoint clean diagnostic source; reuse exact owned/registry guards."""
from pathlib import Path
import hashlib

old=Path(__file__).resolve().parents[1]/'chitchat-compat-074/common.py'
raw=old.read_bytes();assert hashlib.sha256(raw).hexdigest()=='a3b9caa0cb423b85714f7b41d87a82ee0e3dec305478de4084c3f08590b9b32e'
exec(compile(raw,str(old),'exec'),globals())
TOOLS=ROOT/'tools/chitchat-cost-074';RELBASE='target/chitchat-cost-20261029-r1';BASE=ROOT/RELBASE
COPY=NATIVE/'source-root-clean-cost-r1';_parent_source=source


def source():
    result=_parent_source()
    for p in (ROOT/'tools/chitchat-compat-074').iterdir():
        if p.is_file():result[p.relative_to(ROOT).as_posix()]=sha(p.read_bytes())
    return result


def prepare():load(TOOLS/'prepare.py','clean_cost_exact_preparation').main()


def protocol():return json.loads((TOOLS/'protocol.json').read_bytes())
