# Ordinary owned original probe local evidence

Baseline `f22ebc7c11dbb1a4104ca2b0dda207ceb66488f1`, preregistration
`0f6a3145a5898c39d6a8797d93725d57a94e6268`, implementation
`e3c91ad2d42a5af0d6ce089fb93c925622fa272a`, tree
`b7d832f62805986721e1c4d002b1f3d8b49f481f`. See the
[design](../../diagnostic-signer-owned-probe-design.md) and
[contract](../../diagnostic-signer-owned-probe-contract.toml).

This unsigned local consistency packet binds final source, frozen inputs,
unchanged independent sources and normalized merged tool output (UTF-8 LF,
one terminal LF). It does not attest execution or clean-source provenance.
Baseline, red and development captures intentionally do not execute the final
implementation. Final checks precede the source commit, except the separately
repeated ordinary child/registry checks. No real signer or production receipt
is involved; all success lines explicitly keep admission false.

## Outcome

- Baselines: 11 old first-exec cases and ordinary child/model 5/7. Missing normal
  binary initially fails the Cargo compile-time binary variable before mutation.
- Initial normal owner suites passed 7/1/4. Repeats then exposed ETXTBSY in the
  outer integration launcher on current Rust and MSRV; both failures remain.
  The original writer was not enumerated. Parallel copy/fork inheritance is an
  inference consistent with the old retained-writer fixture and the new held-
  writer control, not an authenticated attribution of the failed descriptor.
- A test-only mutex now serializes private copy and Command startup through
  exec; the independent normal owner lifetimes remain concurrent. The fixed
  control requires ETXTBSY while its original writer remains open, even at 0555,
  with no retry. Only explicit writer drop permits a separate successful probe.
- Pre-commit review added a required default/non-autoreaping SIGCHLD policy
  before fork. Actual inherited SIG_IGN refuses; SA_NOCLDWAIT is a pure model
  negative. The invoking parent's disposition is never rewritten.
- Five final Linux Rust 1.94 repeats each pass unit/portable/process suites
  9/1/5. The normal artifact is copied into private 0555 test images; no Cargo
  artifact, checkout permission, account, service or cgroup is modified.
- Real original-child SIGKILL/drop occurs at initial and first-exec stops;
  foreign equal-byte object, malformed/writable/hardlinked image, internal-mode,
  environment/argument, reaped/phase and wrong-terminal-output cases refuse.
  The missing-pidfd field control is a model, not injected syscall failure.
- Status classification checks 327680 cases, 256 mutations at seed 0x0740c07a,
  and four independent owners. Normal process cases cover native/x32/i386 and
  12 children under four owners with parent seccomp/core/cwd unchanged. A parent
  success line is data, not independent evidence of installed filter bytes.
- Linux current/MSRV all-target/all-feature checks and strict current Clippy
  pass; Linux MSRV suites pass 9/1/5. Windows current check/strict lint and
  current/MSRV portable regressions pass 7/1 each; Windows also lints xtask.
  A first lint failed on err().expect(); explicit unexpected-child cleanup
  replaces it without suppression. The failure capture remains.
- Root contracts: 157. Old first-exec/namespace/handoff/constructor-loss/creator-
  loss suites: 11/13/13/7/8, unchanged fixtures. Ordinary child/model: 5/7.
  Registry: 174 discovered gates, all registered. Format/docs/local performance
  contract pass. Ship admission intentionally refuses exit1; its wrapper checks
  that expected refusal and exits zero.

Observed unsigned debug binary SHA256 after checks: Rust 1.94
`219dd28ed620a48fe3fc77d9a7407b6ff09c267dd76b1db1abefb21439174b33`,
Rust 1.88
`b341862ea61d75bee6fe1aad13f6a31dd3466baa36180976b30b18e198d03c8e`.
These identify local artifacts only, not reproducible builds or admission.

## Reproduce without credentials or a server

```text
# Nonroot Linux x86_64; integration tests prepare their own readonly image:
cargo test -p hydracache-long-run-supervisor-074 --bin diagnostic_signer_owned_probe --test diagnostic_signer_owned_probe --test diagnostic_signer_owned_cli --locked --offline -- --nocapture
# Portable CLI and fixed policy regressions:
cargo test -p hydracache-long-run-supervisor-074 --test diagnostic_signer_owned_cli --test diagnostic_signer_isolation --locked --offline
python docs/testing/performance/0.74/local-runs/diagnostic-signer-owned-probe-20261015/verify_packet.py
python -m unittest discover -s docs/testing/performance/0.74/local-runs/diagnostic-signer-owned-probe-20261015 -p test_verify_packet.py
```

Normal Cargo binaries are writable and therefore are not accepted directly as
owned images. Use the harness; do not chmod build output or run this code inside
an existing service. The private owner has no public constructor or guard export.

No selected signer/root/runtime or protected-job custody, all-boundary normal
parent-loss proof, mapped instruction attestation, actual key/token, authenticated
worker start, all-writer revocation, completion/later cells, signing, server or
qualification admission is enabled. Next: selected root/runtime composition and
original-owner/creating-task loss before credential handoff. The old 64-instruction
policy and fixtures, frozen lock/manifest, thresholds, dependencies and workflows
remain unchanged. No cache performance was measured and no push was performed.
