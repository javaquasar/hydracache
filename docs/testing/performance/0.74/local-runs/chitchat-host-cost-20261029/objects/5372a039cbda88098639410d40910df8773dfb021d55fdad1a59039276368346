"""Distinct fixed-caller source/image custody with original ownership guards."""
from pathlib import Path
import hashlib

_old=Path(__file__).resolve().parents[1]/'chitchat-native-cost-074/common.py'
_raw=_old.read_bytes();assert hashlib.sha256(_raw).hexdigest()=='730ea6a12376cd72eaa0e6543685a443241ab0a12e0bb039ab02543961840a96'
exec(compile(_raw,str(_old),'exec'),globals())
TOOLS=ROOT/'tools/chitchat-caller-074';RELBASE='target/chitchat-caller-20261029-r2';BASE=ROOT/RELBASE
COPY=NATIVE/'source-root-caller-r2';_inherited_source=source
TABLE_SHA='4a3308a82d014fb58df808a9f3984922537a80c02219610cdf5ee6fa1ec4a1f9'


def source():
    result=_inherited_source()
    for name in ('chitchat-native-cost-074','chitchat-commit-074','chitchat-stateful-074'):
        for p in (ROOT/'tools'/name).iterdir():
            if p.is_file():result[p.relative_to(ROOT).as_posix()]=sha(p.read_bytes())
    need(result['tools/chitchat-caller-074/cases-r2.json']==TABLE_SHA,'original frozen request table');return result


def prepare():load(TOOLS/'prepare.py','caller_exact_preparation').main()
