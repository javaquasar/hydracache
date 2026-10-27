# Focused package repairs and remaining PID composition

Explicit fixture interpreter selection and six exact historical input compositions
now pass focused Windows 1.94 and MSRV 1.88 controls. Each final composed command
completes seven targets with 30 passes, zero failures and zero ignored cases.
The separate PID candidate is rejected and reverted. GNU validation and the full
package gate remain open; the earlier 153-target result stays red at ten failures.

Python selection: `c14fa94916dbb5412e4d489b2234910b799e12a2`.
Six input compositions: `9ea585b6cc494a79bf3ddbebe9bb634fb0137eae`.

## Explicit Python selection

Two memory fixtures previously called `python` on every platform. The shared
test-only selector now chooses `python` on Windows and `python3` on Unix. It does
not probe alternatives, change PATH, install an alias, modify the scripts or
weaken their exit checks. A pure control checks both choices and the actual host
branch. All original fixture arguments, assertions and test names remain.

The public-API script's selected `--check` route validates the pinned manifest,
baseline Git tag and complete publishable-package inventory via `cargo metadata`;
it does not enter the semver execution route. The provider fixture invokes only
normalization of synthetic files in owned temporary directories, including its
reordered-input refusal and redaction assertions. The complete script bodies and
`system.py`/`provider_common.py` import path were reviewed. These are not product
workloads or production memory measurements.

The first Windows cohort passes twelve cases. All-targets check, strict
all-feature Clippy and targeted rustfmt pass. Workspace `cargo fmt --all --check`
first refuses with Windows OS error 206, before formatting: command-line length
is too large. That result remains retained; targeted formatting is not labelled
a whole-workspace format pass.

## Exact source input compositions

Six explicit consumer/path pairs restore named Git producers only after checking
the complete current body. The compiled rule bytes have an independent SHA256;
ten ordered inverse edits require exact offsets and text, followed by the complete
predecessor SHA256. Only the original CRLF-to-LF representation is applied.
Unknown consumers refuse; other paths retain their original readers/checks.
There are no added Git calls or filesystem effects inside the adapter, no cache
of current observations and no runtime authorization or admission API.

- Two cache inputs account for the later `successor::project` installation while
  retaining the original cache/delegation assertions against the cache producer.
- Two remaining-reader inputs account for already installed held-security and
  current-frontier projections. The seven old construction rules stay unchanged.
- The census input accounts for the installed security projection and imports;
  the original provider/caller reconstruction remains independently checked.
- The registry input accounts for the exact added diagnostic registrations;
  the historical baseline and all deadline/mutation assertions remain unchanged.

The compatibility-document producer explicitly advances from `a67a5299a` to
`d3256fc08b8da2df424d6da5bce99c0e168939b0`, matching the already installed
diagnostic additions. Its independent predecessor stays
`3c52c0aa34b8d00b2fac44375d5ba8fdf532bb9f`. This is a reviewed input binding,
not a change to thresholds or an arbitrary new expected hash.

Three new source-only controls bind all six current/prior bodies to their actual
Git objects, test CRLF representation, stale/empty/truncated/appended inputs and
32 seeded mutations per row, and check success after refusal. They also compare
every original test name and assertion token in all five legacy test files with
the pre-repair source. The final five-target cohort passes eighteen cases.
All-targets check, strict all-feature Clippy and targeted rustfmt pass again.

## Rejected PID candidate

The first nine-row implementation has a compile refusal because this locked
SHA256 array does not implement `LowerHex`. Its source and result remain. The
byte-wise encoder fixes that defect without changing digest semantics.

The next candidate completes six targets with eighteen passes and three failures.
It incorrectly applies the raw admission-body pin after the existing runtime
reader has already verified and returned its named predecessor. It also changes
the PID test itself, which correctly trips the immutable old whole-source guard
in the remaining-reader cohort. Do not bypass either refusal.

Only the three PID rows and PID-test edits are removed. The PID test is restored
to its original bytes; no product or shared legacy guard is rolled back. A proper
PID repair must distinguish the original conditional reader and subsequent
document/custody/PID stages, preserving their order, current-input checks and
independent source bindings. Merely accepting both hashes would not prove the
join. The original PID failure is not closed by this step.

Saved logs also contain intentionally caught panics. In two remaining-reader
failures the first panic is not the terminal failure: the last uncaught assertions
compare the changed caller and newer compatibility document. All seven frozen
construction baselines independently match their existing `before_sha256` values.
Those pins are not corrupt and are not replaced. Initial raw logs and the earlier
first-panic index remain unchanged; this refines their diagnosis.

## Evidence and gate scope

The [180-member packet](local-runs/package-closure-repairs-20261027/index.json)
retains 85 content-addressed objects, thirteen capture phases, exact rejected
candidate snapshots, the nine-row producer/diff audit and accepted source witnesses.
Index SHA256:
`ef6fb6bdc0fba560d2d82e977e0f7113e0fa93a9b8fefc064434389ac9126647`.
No archived source is executed. Separate Windows/MSRV final cohorts are green;
their passes are not summed or joined into a full package result.

Doc-check and the local non-promotable performance-contract check pass. Their
[24-member packet](local-runs/package-closure-repair-checks-20261027/index.json)
has index SHA256
`18a92db4626c40b4606acb465118e142f177cac7c0c63e91a0daf0ea237f5888`.
These local checks do not waive the unresolved PID, GNU or full-package gates.

The first Python test's old source map did not include its then-untracked helper.
Later staged and composed checks bind that helper. Three provider-script witnesses
match the prior Git bodies but were outside the old wrapper's pre/post source map;
the packet records this limitation explicitly. Before a new GNU/full command,
the independent source/effect inventory must include these actual script/import
inputs and interpreter/Cargo resolution in the real test-child environment.

Next finish the isolated PID stage composition, validate the changed code and
scripts on GNU, and perform a fresh complete source/effect/environment review.
The new ordinary source-control target changes the future target catalog; the
old fixed 153-target review must not be reused or extended with a suffix. Only
then admit a distinct full command under the same 780-second/four-thread/floor
contract. No unchanged full run, new performance hypothesis, dependency update,
server/service/clock action, product load or qualification occurs. Qualification
identity, all live checks and original 300/60/10 origins/windows remain unchanged.

The subsequent [PID stage repair](pid-stage-join-result-20261027.md) closes the
focused Windows/MSRV composition without editing either original reader or the
six rules above. It preserves these first rejected outcomes. GNU validation and
a fresh full-package admission remain the next requirements.
