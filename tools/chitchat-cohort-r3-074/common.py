"""Original local owned checks, with immutable maintenance/protocol source pins."""
from pathlib import Path
import hashlib

old=Path(__file__).resolve().parents[1]/'chitchat-native-cost-074/common.py'
raw=old.read_bytes();assert hashlib.sha256(raw).hexdigest()=='730ea6a12376cd72eaa0e6543685a443241ab0a12e0bb039ab02543961840a96'
exec(compile(raw,str(old),'exec'),globals())
TOOLS=ROOT/'tools/chitchat-cohort-r3-074';BASE=ROOT/'target/chitchat-cohort-r3-20261030-r1'
_base_source=source


def source():
    result=_base_source();name='tools/get-owner-scheduled-controls-074/maintenance_d3.py'
    result[name]=sha((ROOT/name).read_bytes());need(result[name]=='fcb8a53eba5ebc7e1364cbfa0d2bcb9e0ad1098f75b4e53dae56b7503c02ed38','original maintenance base')
    name='tools/chitchat-cost-074/protocol.json';result[name]=sha((ROOT/name).read_bytes())
    need((ROOT/name).read_bytes()==(ROOT/'target/chitchat-host-cost-20261029/prepared/input/protocol.json').read_bytes(),'original frozen numerical protocol')
    return result
