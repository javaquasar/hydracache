"""Fixed actual-caller derivative of the pinned native owned dispatcher."""
from pathlib import Path
import hashlib

old=Path(__file__).resolve().parents[1]/'chitchat-native-cost-074/run.py'
raw=old.read_bytes();assert hashlib.sha256(raw).hexdigest()=='0de608634554774fda9d3e60d3c2418d22ba44ee31d6280206a26a15d8944902'
text=raw.decode()


def change(before,after,count=1):
    global text
    assert text.count(before)==count,before;text=text.replace(before,after)


change("'check-B','check-P','clippy-B','clippy-P','cohort-1','cohort-2','cohort-3','cohort-4'", "'check-B','check-P','clippy-B','clippy-P','build-Q','control-Q','check-Q','clippy-Q','cohort-1','cohort-2','cohort-3','cohort-4','cohort-5','cohort-6'")
change("('B','P','P','B')[position-1]","('B','P','Q','Q','P','B')[position-1]")
change("target=c.NATIVE/'targets'/variant","target=c.NATIVE/'caller-r2-targets'/variant")
change("str(c.TOOLS/'control.c')","str(c.ROOT/'tools/chitchat-native-cost-074/control.c')")
change("str(c.NATIVE/'c-control-scope')","str(c.NATIVE/'c-control-caller-r2')",2)
change("'delta::cost_review::native_rust_tag_control','delta::mtu_review::supported_delta_operations'", "'delta::cost_review::native_rust_tag_control','delta::mtu_review::supported_delta_operations','serialize::stateful_review::stateful_semantics_control','serialize::stateful_review::stateful_capacity_control','delta::cost_review::actual_caller::fixed_caller_semantics_control','delta::cost_review::actual_caller::fixed_caller_capacity_control'")
change("'delta::cost_review::native_operation_observations','--exact'", "'delta::cost_review::actual_caller::fixed_caller_observations','--exact'")
change("c.diagnostic(raw,'HC074_NATIVE_ALLOC:');c.semantics(value['rows'])", "c.diagnostic(raw,'HC074_CALLER:');c.need(value['variant']==variant and len(value['rows'])==30,'complete actual-caller body')")
change("argv=[str(tool),'--edition','2024','--config','skip_children=true',str(c.BASE/'observer.rs'),str(c.TOOLS/'native_tests.rs')]", "argv=[str(tool),'--edition','2024','--config','skip_children=true',str(c.BASE/'observer.rs'),str(c.TOOLS/'caller.rs'),*[str(p) for p in sorted((c.BASE/'fixtures').glob('*/src/serialize.rs'))],*[str(p) for p in sorted((c.BASE/'fixtures').glob('*/src/delta.rs'))]]")
change("({'tools/chitchat-native-cost-074/native_tests.rs'} if label=='format' else set())","({'tools/chitchat-caller-074/caller.rs'} if label=='format' else set())")
change("root+'/tools/chitchat-native-cost-074/run.py'","root+'/tools/chitchat-caller-074/run.py'")
exec(compile(text,__file__+'[owned-derivative]','exec'),globals())
