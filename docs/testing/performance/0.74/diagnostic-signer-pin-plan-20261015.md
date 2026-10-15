# Selected signer issuer and unsigned prebuilt inventory

Baseline is `08dc602d5ea6cbcc17ef3865e0aeb2e14bf7d1b4`. The operator continued after
being shown the exact raw-public SHA256
`0c22ba7d6c6d16b334e4b1d4a0ce161117800135a977aa30020906caa70af884`.
Treat that as selection of this key for the separate signer-build issuer role,
not production enrollment, receipt signing or job/server authorization.

## Pin implementation

Add a separate portable module, leaving the existing reviewed controller,
observer-builder and worker pins unchanged. The selected raw key and fingerprint
are code constants, never values chosen by an incoming record, environment or
caller key argument. Check their consistency, weak-key refusal and separation
from old roles. A factory may bind this key to the existing explicitly asserted
SignerBuildStatement model; it must still return only that old bounded model,
not authenticated prebuilt custody, bootstrap or execution authority.

The original public generation record and key helper remain immutable: their
unapproved flags describe the generation event. Record current operator selection
in a separate artifact. Tests must distinguish selected issuer from unselected
artifact/runtime/job policy and prove that a synthetic receipt accepted by the
old caller-key model is rejected by the selected-key factory. Verify the original
fixed public selfcheck and its signature mutations without accessing a private
key or signing any new message. Include invalid statements, independent concurrent
selection and compile-fail protection against a caller-selected issuer override.

Capture old signer/trust baselines and a missing-module red test before code.
Run repeated focused tests, portable/current/MSRV check/clippy, source contracts,
doc/registry/local performance checks, scoped format and Python helper regression.
Commit the implementation before any actual unsigned build inventory.

## Actual local unsigned artifact

From that exact clean source, use the existing fixed Rust1.94 command:
`cargo +1.94.0 build -p hydracache-long-run-supervisor-074 --bin diagnostic_controller --release --locked --target x86_64-unknown-linux-gnu`.
Use a separate local Linux target directory with a clean build environment,
without credentials, signing/OIDC authority, profile overrides or product workload.
Retain compiler/Cargo, source/tree/lock and native exit/log observations before
and after. Inspect the resulting binary with readelf, never ldd or execution.
Record ELF interpreter, declared dependencies, loader search tags, binary size/hash
and tool versions; do not infer loaded closure or independent build provenance.

This is one unsigned diagnostic artifact, not a product candidate, reproducibility
proof or an automatically admitted runtime. Missing or unsupported actual runtime
shape is retained as a gap, not repaired by changing the target/profile. A new
hash cannot populate trusted expected inputs merely because it was just measured.
No keys, actual receipt/token, GitHub configuration/workflow or host services are
accessed. Next require separately admitted original prebuilt inputs and protected
runtime/job custody before actual signing/token integration. Worker original-start,
P2/P3 all-writer cessation, completion cells2–4 and release admission remain closed.
