# Fixed diagnostic campaign admission: local evidence

This is a local control-plane packet for W11, dated 2026-10-11. It is not a
HydraCache product benchmark, host installation, diagnostic enrollment or
release acceptance. Baseline is `f3d9045f36eba442f52fdd94f9c58bb5f20cbaaa`;
the [preregistered contract](../../diagnostic-fixed-admission-local-contract.toml)
and [design](../../diagnostic-fixed-admission-local-design.md) precede code.

## Change and result

Canonical Linux production acquire/recovery now checks existing fixed storage
under the original common campaign flock. Any entry excludes admission, not
only a decoded active state. Unknown, pending, terminal, non-UTF-8, directory,
FIFO and dangling symlink entries are never ignored or followed. Missing or
unsafe storage refuses without repair. Original storage ancestry, campaigns
directory and lock identity span pre/post marker handling; a late failure
preserves any marker. Each scan opens a fresh descriptor relative to the
retained leaf, avoiding shared enumeration offsets. The short legacy diagnostic
lock entry checks fixed context too. Unlocked absence is not admission.

Private nonroot models exercise the actual claim/recovery algorithms, not a
public fixture-to-production selector. Empty storage keeps ordinary legacy
campaign recovery, including its existing ownership; it does not enroll a
diagnostic publisher or convert worker ownership into root authority.
Nonproduction roots and non-Linux behavior remain unchanged.

## Receipts and limits

Final focused Linux repeats, captures 18-20, each pass 13 cases with seed
`0x747`. Final Linux regression, capture 21, passes 526 cases with one existing
ignored real-systemd-bus test; the ignored test is not claimed as coverage.
Windows package regression, capture 25, passes 196 cases. Native root registry,
governance and performance-contract tests, capture 26, pass 155 cases, including
113 performance-contract cases. Windows Linux-gated targets report zero cases;
this packet does not count them as Linux execution.

Baseline capture 01 retains the previous short-fence negative case: a
nonproduction legacy fixture admits a campaign after guard drop despite fixed
Reserved context. New private production-predicate models instead block after
guard drop with valid canonical schema-1 Reserved bytes. Baseline is historical,
not a current production bypass claim.

Capture 02 is the expected missing-implementation compile failure. Capture 03
retains an initial test failure: a symlink campaign root was wrongly expected
to be accepted. The old rejection was kept; only the test expectation was
corrected. Captures 04-17 are intermediate source results, even where a filename
contains "final". They are not substituted for final repeats/regression 18-24.
Intermediate captures 13-17 retain success output but lack a shell completion
receipt; their observed exits are explicitly null, not inferred as zero.
Scanner cases were initially outside the focused filter and later included.
A racy global-fd reuse assertion was removed from the scanner test; functional
I/O refusal/no-repair remains covered. No fd-leak measurement is claimed.

Rust 1.94.0 and MSRV 1.88.0 all-target/all-feature checks, warnings-denied Clippy,
format, rustdoc, compile-fail doctests, local performance contract, doc-check
and native mdbook receipts are retained separately. Exact commands, exit
statuses (or an explicitly missing intermediate receipt), raw byte counts/hashes
and per-capture test totals are in
[manifest.json](manifest.json). Repeated tests are functional results, not
timings or proof of physical host enrollment, process races or power-loss
durability.

Qualification dry-run remains genuinely red on the inherited main-plan digest
mismatch. All eight frozen qualification inputs and manifest bytes are
unchanged; no declared digest or threshold was adjusted. Require-ship also
remains red. This packet grants no worker execution, qualification, server
operation, performance claim or release permission.

## Reproduce verification and next boundary

Run native Windows Python from the repository:

```powershell
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-fixed-admission-20261011/verify-packet.py
```

The verifier binds current implementation to its Git commit/tree, final
receipts and W11 registration, and historical evidence to its own original
commit/tree without requiring today's extended source to equal old source.
It also checks frozen qualification inputs and local documentation targets.

Before publication, coordinate a reviewed common-binary deployment that
excludes older supervisor binaries, then establish quiescent trusted campaign
ownership for every participant. Original authority/intent/state composition,
trusted reservation and consumption publication under the same fence, and
output/start/watchdog/owned cleanup remain separate packages. There is no
complete publisher serialization or installed-host proof in this change.
