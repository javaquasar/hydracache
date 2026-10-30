# Next eight preparation packages: implementation evidence

Baseline: `98d7321b1cda568c386cbfdef1993c7e4dcebe38`. Plan:
[pre074-next-eight-implementation-plan.md](pre074-next-eight-implementation-plan.md).
Release 0.75 remains planned; W0 and production IMap remain blocked by 0.74.

| Package | State | Local evidence |
| --- | --- | --- |
| P1 managed builds | Implemented | 8 process-controller tests, 9 CI contract tests, 10 existing process tests; all-target check and strict Clippy; real Cargo/Maven preparation and package reproduction |
| P2 helper ports | Implemented | 7 helper variants in Rust/Java ports and external consumers; core/helper/async tests, all-target check, strict Clippy and unchanged Java coverage ratchet |
| P3 native observation | Implemented | Native Maven suite, strict independent Rust verifier, preparation v2 artifact/replay; actual managed packaged probe passed |
| P4 native histories | Implemented | Two real official clients; seeds 117/118 with actual overlap; Java and independent Rust DFS, strict raw/normalized binding, replay and bounded shrinking; preparation v3 artifact |
| P5 listener recovery | Implemented | Controlled writer gate and exact callback tuples; 29 native suite tests; actual packaged recovery probe independently checked, preparation v4 artifact/replay |
| P6 continuous soak | Implemented | Clean-source actual smoke: reference 19 rounds, native 5, daemon 11, all reaped/no live members; checkpoint replay creates a new segment; preparation v5 and local-archive workflow; no 6/24-hour execution claimed |
| P7 development SBOM | Implemented | Clean-source `61b08b5794e1617930d49d4707f8c02089a4905b`: 995 components; complete generation and independent reconstruction passed; 12 Rust tests plus actual five-stage graph replay and six Java tests; dependency policy Rust failed / Java unknown, unsigned |
| P8 scoped coverage | Validation in progress | LLVM/JaCoCo capture/reconstruction, strict source/tool/cfg/session binding, zero-missed changed-line gate and negative canaries implemented; focused Rust tests and strict Clippy passed; 39 native and 10 provenance Java tests with mandatory class gates passed; native source report has no missed executable lines; no Rust coverage percentage claimed before actual capture |

P1 uses a separate build execution policy. Public short proof workers retain the
60-second maximum. Deadline, output/event/stage exhaustion, cancellation and failed
cleanup stop further build stages. Failure reports distinguish complete logs from
captured prefixes. Workflow budgets begin before checkout and reserve cleanup time.

P2 uses the original helper async facades, service owners, retained ledger, credits
and leases. Read invocation identities do not create retained mutation entries.
Immutable development grants and bindings check local adapter consistency; they
are not production authentication. Prepared requests belong to their local port.
Java completion leaves a leased result usable until invocation.close, which also
closes an already delivered lease. Rust results retain their existing owned lease.

Local logs reside under ignored `target/imap-075/`; exact source-bound receipt and
remote matrix qualification are performed after the final implementation commits.
General repository CI failures inherited from 0.74 are reported separately.
