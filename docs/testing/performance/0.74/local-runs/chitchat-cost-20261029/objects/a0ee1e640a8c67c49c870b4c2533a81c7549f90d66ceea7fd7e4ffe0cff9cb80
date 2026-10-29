"""Disjoint stateful fixture using byte-pinned original ownership primitives."""
from pathlib import Path
import hashlib

_old=Path(__file__).resolve().parents[1]/'chitchat-native-cost-074/common.py'
_raw=_old.read_bytes()
assert hashlib.sha256(_raw).hexdigest()=='730ea6a12376cd72eaa0e6543685a443241ab0a12e0bb039ab02543961840a96'
exec(compile(_raw,str(_old),'exec'),globals())
TOOLS=ROOT/'tools/chitchat-stateful-074'
RELBASE='target/chitchat-stateful-20261029'
BASE=ROOT/RELBASE
COPY=NATIVE/'source-root-stateful'
_inherited_source=source


def source():
    result=_inherited_source()
    for name in ('chitchat-native-cost-074','chitchat-commit-074'):
        for p in (ROOT/'tools'/name).iterdir():
            if p.is_file():result[p.relative_to(ROOT).as_posix()]=sha(p.read_bytes())
    return result


def prepare():load(TOOLS/'prepare.py','stateful_exact_preparation').main()
