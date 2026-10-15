# Shared GNU signer declaration catalog: local evidence

Implementation is `f10f5d64bdc752dabb40b403d9c055dd5a1d59ac`, tree
`5018405379547bc02f0b61f9adfde961e6f4851b`; preregistration is
`1076bef53595b229ba0e617a43811e44f0ef6689`, baseline is
`6223d9c04797a01ca30df80e52483a8f6b0e70f9`. The shared bounded ELF parser and
four-role graph now serve both ordinary library callers and the existing owned
test fixtures. There is no catalog-to-execution-authority conversion.

## Checks and failures retained

Baseline portable signer-build tests: 9; baseline Linux runtime fixtures: 10.
New portable integration suite: 6 cases, including 256 seeded mutations and four
independent readers; repeated on Windows/Linux Rust 1.94, and passed on both
Rust 1.88 MSRV hosts. Linux example: 2 tests; compile-fail doctests: 2.
Post-change Windows build/issuer regressions: 9/6; Linux runtime/rooted-exec
regressions: 10/11. Rooted-exec includes real owned-fixture process loss, not a
production controller run. The final root performance contract suite passed all
153 tests; all 171 discovered gated tests remain registered.

All-target checks and strict all-feature Clippy passed for the supervisor on
Windows/Linux and xtask on Windows. Scoped formatting, doc-check and local
non-promotable performance-contract-check passed. `--require-ship` remained an
expected refusal. This is not full workspace verification or release qualification.

The initial red test also used a private hash helper (E0603); after correcting
the test setup, the missing API alone produced E0432. An initial source guard
incorrectly assumed rustfmt would not wrap whitespace: 152 root tests passed,
one failed. Only whitespace normalization in that guard was corrected; final
153 passed without changing parser rules or thresholds. Both failures remain.

## Real file observations

The committed example was built and invoked with a clean source worktree before
and after observations. `actual-catalog.json` is its original one-line output.
`actual-status.json` identifies the tool source and executable hash separately
from the pre-existing unsigned controller's reported source `16e94203`.

Four explicit local inputs were inspected: the retained unsigned controller,
GNU loader, libc and libgcc. The result contains their SHA256/size, interpreter,
NEEDED, SONAME and header declarations. Exact paths, independent SHA256 and
stat metadata are in `inputs-before.log`/`inputs-after.log`; these captures are
identical. Role-swapping loader/libc was refused. The initial shell capture
reported exit 1; the explicit native-child capture records exit 2. Both read-only
negative invocations are retained, not hidden as a single attempt.

The catalog is a byte-snapshot observation. It does not prove trusted ancestry,
atomic four-file lifetime, complete relocation/symbol/version validity, mapped
runtime, future loads or writer revocation. The inspected controller was not
executed; ordinary observation tools still use their host runtime. No private
key/token, signing operation, server/GitHub operation, product workload or
qualification occurred. Every custody/start/signing/admission flag remains false.
Neither the debug inspector nor the observed runtime hashes become admitted pins.

## Offline replay

From this directory, run `python verify_packet.py` and
`python test_verify_packet.py` (or `python3` on Linux). Verification reads the
retained packet and Git objects only; it does not reopen or execute inspected
inputs. It checks file hashes, fixed source/tree, frozen lock/qualification
digests, captured test counts, declarations against independent captures and
closed flags. Ten copied-packet tests include rehashed semantic substitutions.
The packet is unsigned local evidence, not tamper-proof attestation or a trust
loader; consistent replacement of all unsigned observations is not authentication.

Next: independently select and retain original runtime inputs, join the catalog
to authenticated build receipt and runtime custody, then connect reviewed
bootstrap/enrollment. Authenticated original-start and all-writer revocation
remain separate prerequisites; completion and cells 2–4 remain closed.
