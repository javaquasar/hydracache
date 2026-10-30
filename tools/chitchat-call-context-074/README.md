# C2 call-lifetime prerequisite

The separate successor to declined C1 uses a private append RAII guard. It shares
the lazy context within one append and releases ownership on normal return/unwind.
The MTU formula/admission/wire remain original. No Root/package publication or
installation follows this prerequisite.

`recipe.py` restores byte-verified original B0/T1, adds the common diagnostic hook,
and refuses to materialize C2 until both T1 tests actually fail. `run.py` owns the
Windows compiler/behavior modes in separate targets. `native_codegen.py` derives
the current owner by exact edits of live pinned code, preserving all checks and
bounds; it adds no archived-producer execution. `native.py` enters that checked
current producer. Faults and the native observer are cfg(test), not timing code.

The [result](../../docs/testing/performance/0.74/chitchat-call-context-result-20261030.md)
distinguishes the positive lifetime/compatibility gates from still-red MTU and
unqualified generic/full resource/CPU/RSS/Root gates. The source implementation,
two first failures and eight semantic peers remain in the separate source packet.
