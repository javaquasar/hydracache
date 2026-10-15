# Public decoder refusal followup

Source `4528d175dbf856bf2cd2e0c7632a66c197dc9fcf` adds explicit sanitization of
decoder RecursionError and a deterministic injected-exception check. This affects
only the public role parser, before any private open, without changing crypto,
filenames, schema, budgets, trust pins or workflow admission. No key was generated
again; a post-fix inspection matched the existing public candidate exactly.

The initial hypothesis was that 1500 nested arrays would reproduce an uncaught
decoder recursion error. On Windows that test passed even before the catch was
added: `signer-custody-depth-red.log` is despite its name a GREEN test, not an
observed parser failure. The surrounding command expected exit 1 and failed its
own expectation. Do not relabel that result as a reproduced security defect.
Interpreter depth behavior is not a portable correctness signal. The final test
also injects RecursionError explicitly and requires the fixed sanitized refusal;
it does not increase limits until a particular decoder happens to fail.

Final 24-case tests pass three times on nonroot Linux and once on Windows with 14
Linux-only skips. Earlier 24-case captures precede the explicit mocked branch;
they are retained but are not the final source gate. Only two Python files changed
after the prior source; the Rust public verifier, signer/trust regressions, strict
lint/MSRV and 149 contracts retain their earlier exact source-specific evidence.
No cache performance or production claim follows.

The [original custody packet](../diagnostic-signer-custody-20261015/README.md)
remains byte-identical to commit `02b3bab2dcddcbd9868b40b64bed7f0fcb51b244`.
Its full source/current-files checks and nine mutation tests passed on both
platforms before this followup. Its source remains `2b90c665`, which performed
the sole actual key generation; do not run its historical source gate against
the two changed files and call the mismatch corruption. The separate archived
byte check still passes on Windows/Linux. The manifest here binds final source
hashes, all raw followup captures and the unchanged raw public record hash.

Public fingerprint approval, production custody, actual signing/token workflow,
authenticated original worker start, all-writer cessation and server/release
admission remain closed. This followup does not approve a pin or access old keys.
