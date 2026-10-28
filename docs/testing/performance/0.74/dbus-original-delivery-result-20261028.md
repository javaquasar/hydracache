# Original held/message-owner/reply compatibility on GNU

The canonical pipe message, addressed original Payload/OriginalReplyState,
HeldRequest and retained fixed channel now use one zbus 6 Message implementation
inside an isolated copy of the genuine supervisor. No conversion between major
versions is added to that Message graph. All **40 selected original wire cases
and one new metadata control pass on GNU Rust 1.94 and MSRV 1.88**. This establishes
type and selected wire compatibility, not execution of an admitted original owner.

Source commit: `0b901422daef2ce76cc812840889282505826398`.
Source/preparation/owners are in `tools/dbus-original-delivery-074/`. Root product
source, manifests/lock, deny policy and qualification manifest stay unchanged.
The preceding [real-owner type closure](dbus-original-owner-result-20261028.md)
remains evidence for its narrower earlier boundary. No new performance hypothesis
or measurement is opened; no security/backport acceptance follows implicitly.

## Scope and original validation

The copy retains all 545 original src Rust files and every original module/test
route. Eighteen direct zbus callers are closed under whole-file source pins;
fifteen are adapted, while manager/loaded/systemd legacy helpers stay on zbus 5.
This is an unpublished two-major migration fixture, not the shipping dependency
graph. The inherited held-wire test adds a sixteenth modified original file;
there is one additional test-only credential adapter, for 546 final src files.

OriginalEnrollmentFrame, consuming OriginalManagerJoin, KernelPacket/Origin
binding, ReceiverPin and watchdog-context sources remain exact. No fake owner or
successful context stub is supplied. The message-owner round remains byte-exact,
including every observe/check/require_names/revalidate and refusal transition.
The entire deliver_held_once body remains exact apart from its lexical crate alias:
same retained connection, stream registered before arm/send, one send, at most 16
reply observations, and unchanged closing inspect/revalidate/refusal.

The existing unsent addressing encode remains. No extra re-encoding, FD-table
reconstruction, reply shim or retry bridges a zbus 5 Message into zbus 6. Canonical
transport, addressed payload, ReplyState and held send agree on one concrete type.
Builder spelling/type changes preserve packet/FD/name/signature checks and all
original validation order. Valid fixed fields and prevalidated unique names still
precede builder use; build/body decoding remain fallible.

## First failures remain evidence

The production-library copy first passes all-feature check and strict Clippy on
both compilers. Test compilation then exposes source inventory and API gaps:

- The first full library test build refuses a missing external owned-case module.
  Adding its exact original source exposes a missing manifest-relative material
  `include!(concat!(...))`; neither attempt executes a test.
- A bounded lexical inventory finds 557 literal targets/621 edges. Three further
  original assets complete that inventory: owned-case controls, material controls
  and the retained status-baseline Rust object. Original bytes and roots are pinned;
  no include/test route is removed. The inventory is not a Rust semantic gate.
- The next build reports 13 errors in two test files: builder/auth/queue/MatchRule
  API, Tokio-half trait implementations and an incorrectly removed header getter
  unwrap. Rust quote characters had confused the first narrow converter. Its v2
  recognizes character/raw literals and preserves zero-argument getters.
- The first API-preparation attempt has an exact-match refusal before native
  dispatch; its partial tree and producer remain. This refusal is recorded as a
  tool observation, not invented OS streams. A subsequent test build retains two
  unavailable old-credential API/type errors and a deprecated Context warning.
- The extended controls catch an accidental DOTALL change in the historical
  converter. The old converter flags are restored; the two isolated converters
  keep their distinct expected bytes. The first control refusal remains.

The final source supplies the actual borrowed-FD ownership operation and current
Context API. Unchanged failing commands are not retried to pass; each corrected
source/input has a separate exclusive root. No lint allowance or deadline increase
is introduced. First producers, exact copied inputs, streams, accounting and
containment receipts are retained alongside successful results.

## Legacy private transport boundary

The old ignored private-bus harness used zbus 5's Tokio ReadHalf/WriteHalf methods.
Its original I/O backend is retained through explicit trait delegation; byte/FD
I/O and credential reads are not replaced by successful stubs. Auth path, queue 16,
match queue 8 and all original private-bus scenarios/bounds remain. This manual
matrix is compiled but not run, and it does not test zbus 6's built-in I/O.

A test-only adapter transfers the actual credential DTO into the new test trait.
Typed field encoding preserves absent versus empty groups/labels and UID/PID/SID.
The old API exposes a process-FD borrow, so a CLOEXEC duplicate retains the same
kernel resource; this is explicitly not preservation of its numeric descriptor
slot. The selected new control checks optional fields, duplicates/order, inode,
CLOEXEC, old-slot closure and final-alias closure. It constructs no admission,
manager connection or original context. Production writer FDs do not use this DTO
helper and receive no additional clone or cost from it.

## Actual gates and what did not run

The initial all-feature library check/strict Clippy pass on GNU 1.94/1.88. Final
default-feature full library test images build on both compilers, with recorded
Cargo source/image paths, byte sizes and SHA-256. Actual complete catalogs agree:
**1,593 tests** each. Exact-name selection executes **41** on each compiler:

- six held-wire cases: 18 aliases/names/command, one-shot job state, malformed reply
  and lost ACK, original/alias flags, duplicate/legacy properties and fixed bound;
- four original addressed-message cases: body/serial/writer binding, envelope and
  alias mutations, wrong flags/replaced aliases, and drop of only extra aliases;
- five canonical pipe-message cases: signed settings/ceiling, seeded value/name/
  order mutations, stream roundtrip/last-writer EOF and wrong descriptor roles;
- 25 unchanged dispatch/ReplyState binding cases, plus the one new DTO control.

Each image reports 41 passed, zero failed/ignored/measured and **1,552 filtered**.
The filtered cases are not passed or waived. Source leaf names must resolve to
unique actual catalog identities; rows/summary/exit, artifact receipts and compiler
catalogs agree. Strict `--lib --tests --all-features -- -D warnings` also passes on
both compilers, covering test cfg compilation, not execution of those targets.
Thirteen offline source/graph/inventory/accounting controls and the new helper's
Rust formatting pass. Existing unrelated root formatting debt is not relabeled.

Every native phase uses the same pinned CI owner, offline/locked sources,
1200-second outer/1100-second inner absolute origins, four-GiB floors and five-second
capacity/retirement controls. Sources/boot and vendor postflight agree. Original
300/60/10, two-second Operation, 250-ms method, package 780 and workspace 4800 bounds
remain exact. No system/private bus, service/clock change, server operation,
installation, product workload, repeated full product gate or performance run
occurs. No real original-owner frame is constructed by this step.

See the [retained packet](local-runs/dbus-original-delivery-20261028/index.json)
for exact source/inputs, all first failures and successful accounting. Native
build trees/test binaries are excluded; artifact/source receipts remain. The packet
contains **13,484 logical members / 1,328 distinct raw objects**. Index SHA-256:
`0b91e58856cda8a32c88cbe7cc2c771ffdcc8ac77fed23bfcdc7175b9263de0f`.
An independent inert verifier passes every raw-object hash/size, actual source
delta and phase/catalog/row/summary accounting without running archived code.
Protected root inputs are rechecked unchanged.

Next prove real held delivery through the fixed manager under the full admitted
owner, then all-writer retirement and disconnect/reconnect of the same attempt.
Legacy transport removal, complete source/effect and shipping/consumer review,
root dependency/security adoption and package/workspace gates remain open.
Durable completion/cells 2–4 and the complete SIGKILL/lost-ACK lifecycle matrix
remain independent release blockers. This step makes no performance claim.
