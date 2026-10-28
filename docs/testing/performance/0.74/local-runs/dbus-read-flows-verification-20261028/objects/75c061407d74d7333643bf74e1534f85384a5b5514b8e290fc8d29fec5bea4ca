# D-Bus read flows: selected protocol, loaded decoder and legacy cache

This is a continuation of the [owner/property controls](dbus-owner-properties-result-20261028.md).
It extends the isolated Windows shadow, not the installed supervisor or root dependency graph.
No system/session bus, real manager, service, clock, server, product workload or performance
measurement is involved. The release is not admitted by this result.

## Source boundary

Independent SHA pins cover diagnostic_manager.rs, diagnostic_loaded.rs and systemd_unit.rs.
The producer preserves complete selected source regions: ObservedUnit, read_unit and its
identity decoder, the loaded settings types/inventory/decoder, and the D-Bus middle of
inspect_in_worker. That middle preserves owner-before, GetUnit, two Unit/Service GetAll
pairs, equality and owner-after. Its test-only adapter accepts a connection and asserted
unit name, then returns the selected observations. The real scope validation, fixed socket,
boot checks, ManagerSnapshot validation and worker/admission boundary are outside this
projection; they are not replaced with successful stubs.

The complete legacy inspect_loaded_unit and optional wrapper retain their original bodies
except injected connection plumbing. Name validation, lazy-cache defaults, property order
and UnitError mapping remain selected original code. One loaded decoder import changes
module spelling to self::take. No root caller or dependency is changed.

## First results and corrections

The first Rust 1.94 build rejects the added return adapter with E0505: the manager Proxy
still borrows owner. This is a test adapter mistake. A separate v2 copies owner at return,
keeping the original proxies alive through both owner observations. Its original source,
compiler error and receipts remain immutable.

The first v2 native run completes seven preceding tests, then waits in the first legacy
lazy-cache scenario. The original 1200-second owned envelope expires (exit 124), the Job
retires within its original cleanup contract without errors, and incomplete native
identity accounting explicitly refuses it. Seven successes are not a passing 30-case gate.
Both complete streams and the refusal remain retained; there is no unchanged retry.

Source inspection identifies a missing fixture exchange: lazy PropertiesChanged setup
resolves the well-known destination with GetNameOwner before GetAll. The initial peer
expected GetAll directly and panicked on the unexpected request, leaving cache setup
waiting for an unanswered raw exchange. This is a defective peer script, not evidence of
an admitted supervisor failure. A separate v3 answers that service lookup before each
lazy cache initialization. The selected original reader is unchanged.

V3 also makes peer assertion failures close the test connection before propagating the
panic. A dedicated negative intentionally sends the wrong unit name, requires the caller
to return an error and the peer join to fail, and verifies exactly one received request.
Thus a mock mismatch cannot be silently counted as a valid snapshot. No production
connection, timeout, validation predicate or check count is changed.

## Coverage and limits

The script asserts each request's destination, path, interface, member and decoded body.
The protocol cases cover unchanged observations, owner replacement after reads, changed
second unit state, absence from the admitted sender and a foreign error sender. Loaded
settings cover the complete decoder inventory and Unit.Description precedence, then
missing identity/description/property, invalid InvocationID length, wrong property type
and an oversized field. These are decoder controls, not InvocationGuard/policy acceptance.

The legacy cases cover lazy Unit/Service GetAll snapshots, optional absence, invalid unit
namespace before I/O, and cache initialization failure. Synthetic UID 0/PID 1, names and
headers do not establish kernel credentials or host authority. Real bus routing,
PropertiesChanged invalidation, Unix descriptors, original delivery and all-writer
cessation remain separate gates.

## Local result

The full actual catalog passes **31/31 on Rust 1.94 and 31/31 on Rust 1.88**: the original
21 identities, nine read-flow functions and one peer-failure control. Six Python catalog
controls reject missing/foreign/duplicate identities, false summaries, an old catalog
without the new control and duplicated baseline authority. Rust formatting passes.

Sixteen new flow scenarios assert 97 complete received request routes/bodies per image;
the preceding fifteen owner/property scenarios assert their original 31 requests. The
new intentional mismatch separately records one rejected request. Traces are compared
between compilers. A zero-request namespace refusal is retained as its own scenario.
Every error path requires the original error and script truncation, including no final
owner lookup after changed observations or a first-pair decoder error.

The two image identities come from actual Cargo compiler-artifact messages, are bound to
source/target paths and SHA-256, and are listed before execution. There are no ignored or
filtered tests. This is not paired execution against an old zbus 5 runtime, nor proof of
all old/new API semantics. Complete selected caller source is held exact instead.

## Dependency and effect review

Only the standalone probe gains exact serde_json 1.0.150 and thiserror 2.0.18 dependencies
needed by the selected original types. Offline resolution adds six locked identities,
without removing/upgrading prior identities: itoa 1.0.18, serde_json 1.0.150, syn 2.0.119,
thiserror 2.0.18, thiserror-impl 2.0.18 and zmij 1.0.23. Existing syn edges become explicitly
qualified to syn 3.0.9; this is lock disambiguation rather than a changed selected version.
This probe graph must not be described as byte-identical to the root graph.

All six registry archive checksums and 347 extracted source members match cached primary
archives. Their declared minima are 1.68 or 1.71; actual 1.88 compilation remains a distinct
check. Build-script review finds serde_json target-width cfg selection, zmij rustc-version
and optimization cfg selection, and thiserror's OUT_DIR private module/compiler probes and
own probe-directory cleanup. These compile-time effects are not read-only execution.
Proc-macro/parser code and the immutable scoped policy still require the recorded gates.
No ignore-rust-version, lint suppression, deny exception or root lock change is introduced.

Original live predicates/frequency and 300/60/10 windows, package/workspace 780/4800,
local 1200-second owned envelope, 4 GiB floor and five-second capacity/retirement remain.
No performance or non-regression claim follows from protocol test duration.

## Checks and retention

Both complete probe all-target/all-feature checks and strict Clippy pass on Rust 1.88 and
1.94; scoped unchanged cargo-deny passes advisories/bans/licenses/sources on both. Every
owned command retires without errors. Whole source, HEAD, fixture, overlay and protected
root guards hold. The first timeout remains an incomplete refusal, not a test pass.
The root Cargo.lock, Cargo.toml, deny policy, supervisor sources and qualification manifest
stay exact. There is no full product package/workspace replay in this continuation.

The source-only commit is `f3d21b96274df564c033fc5ca6e2996a1bfd6755`.
The [primary packet](local-runs/dbus-read-flows-20261028/index.json) contains 1007 logical
references and 397 distinct raw objects. Index SHA-256:
`4be475b3f92e348e8ccde151356d73607e0b609ebcacf406c4e13fc586131fb2`.
It retains all three producer/fixture revisions, first streams/refusals, native image
identities/catalogs/accounting, exact projected regions, source/effect audit, added primary
crate archives and the predecessor index. Native executables and target trees are excluded.
Inert verification checks bytes rather than executing archived code. Advisory DB state
is a clean postflight observation, not a per-lookup seal.

The first inert packet verifier also refuses before checking members because its
placeholder index pin was not filled after a documentation utility stopped on Windows'
default text encoding. No packet bytes or native outcomes change. The document update
is repaired using explicit UTF-8 and original Git text; a separately named corrected
verification uses the exact sealed index pin. That utility refusal is retained too.

## Next boundary

GNU/MSRV and Unix FD/original ReplyState/partial-I/O/disconnect/cleanup checks, complete
source/effect/distribution review and fresh full product package/workspace gates precede
dependency adoption. Real authenticated original execution, autonomous writer retirement,
durable completion and reconnect to the same attempt remain open. Server preparation and
admission are not inferred from an in-process peer.
