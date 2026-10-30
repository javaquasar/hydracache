"""Explicit adaptations of the live, pinned r5 owner; never archived Python."""
from pathlib import Path
import recipe as p
c=p.c
PINS={
 'tools/chitchat-borrowed-074/native.py':'25be974c0e32e228076ede2f1150913235c26c504c26a2d608b4d2e59409521c',
 'tools/chitchat-borrowed-074/parallel_source.py':'89278fa755e744dc17d4c0192c32960c74ca2e8f487fe561d7744b1d00fe26fb',
 'tools/chitchat-borrowed-074/native_tests.rs':'456958f7c7df1b80eee4c84b52e80709754b5152903322b22856acbe07d45272'}
CORE=p.BASE/'producer/native_core.py'


def source():
    for name,digest in PINS.items():c.need(c.sha((c.ROOT/name).read_bytes())==digest,'live qualified owner/observer source: '+name)
    raw=(c.ROOT/'tools/chitchat-borrowed-074/native.py').read_bytes()
    def replace(before,after):
        nonlocal raw
        c.need(raw.count(before)==1,'one exact native adaptation: '+before.decode());raw=raw.replace(before,after)
    raw=raw.replace(b"'C1'",b"'C2'").replace(b'"C1"',b'"C2"')
    replace(b'/home/artur/.cache/hydracache-074-borrowed/20261030-c1-r5',b'/home/artur/.cache/hydracache-074-call-context/20261030-r1')
    replace(b"INPUT=p.BASE/'native-input-r5'",b"INPUT=p.BASE/'native-input-r1'")
    replace(b"OUT=p.BASE/'native-r5'",b"OUT=p.BASE/'native-r1'")
    replace(b"FRAME=p.BASE/'native-frame-r5.json'",b"FRAME=p.BASE/'native-frame-r1.json'")
    replace(b"p.HERE/'native-plan.md'",b"p.HERE/'plan.md'")
    replace(b'/tools/chitchat-borrowed-074/native.py',b'/target/chitchat-call-context-20261030/producer/native_core.py')
    replace(b"    value[FAST]=FAST_SHA\n    return value",b"    value[FAST]=FAST_SHA\n    import native_codegen\n    value.update(native_codegen.held())\n    return value")
    replace(b".replace(b'const EXPECTED_CONTEXT: bool = false;',b'const EXPECTED_CONTEXT: bool = true;')",b'')
    replace(b"text.splitlines()[-1]=='111 tests, 0 benchmarks'",b"text.splitlines()[-1]==str(len(old)+len(added))+' tests, 0 benchmarks'")
    replace(b"value['context_present']==[False,v=='C2',v=='C2']",b"value['context_present']==[False,False,False]")
    header=b'''# Current maintained producer, derived by exact edits from the pinned live owner.
import sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[3]/'tools/chitchat-call-context-074'))
'''
    return header+raw


def held():
    expected=source();c.need(CORE.is_file() and not CORE.is_symlink() and CORE.read_bytes()==expected,'exact current native producer body')
    return {**PINS,CORE.relative_to(c.ROOT).as_posix():c.sha(expected)}


def materialize():
    raw=source();CORE.parent.mkdir(exist_ok=True)
    if CORE.exists():c.need(CORE.read_bytes()==raw,'same current native producer, no overwrite')
    else:CORE.write_bytes(raw)
    return CORE
