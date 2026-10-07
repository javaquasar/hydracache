# Local scheduled/native instrumentation only

No timing CLI, daemon or qualification launcher is provided. The opt-in
`allocation-diagnostics` binary is separate and must never certify timing.
Use the library's bounded `scheduled::run`, not the source-referenced legacy
`rate::run_open_loop`. All observations are non-promotable and not product claims.

See `docs/testing/performance/0.74/get-response-owner-scheduled-controls-design.md`
and its contract for exact latency/accounting, security and remaining-proof limits.
The standalone lock must be shared by off/on builds in any later sealed cohort;
never mix measurements from this lock with historical tool binaries.

From the repository root:

```powershell
cargo fmt --manifest-path tools/get-owner-scheduled-controls-074/Cargo.toml --check
cargo test --manifest-path tools/get-owner-scheduled-controls-074/Cargo.toml --all-targets --locked -- --test-threads=1
cargo test --manifest-path tools/get-owner-scheduled-controls-074/Cargo.toml --all-targets --features get-owner --locked -- --test-threads=1
cargo check --manifest-path tools/get-owner-scheduled-controls-074/Cargo.toml --all-targets --all-features --locked
cargo clippy --manifest-path tools/get-owner-scheduled-controls-074/Cargo.toml --all-targets --all-features --locked -- -D warnings
```

Tests are semantic/instrumentation fixtures only, not performance evidence. No
current-source hosted CI or full workspace verification is implied by these checks.

RESP2 GET now has a bounded per-response sidecar on one real loopback TCP
connection, with outstanding ceilings 1/10/50 (not fixed-size response-paced
batches). Cancelled callers retain FIFO tombstones until their reply/close.
`RespControl::run` is single-use, shares the original calendar with the driver and
includes wire-owner drain in elapsed goodput accounting. It reports frame-observed
timestamps separately from byte-validated operation completion; neither is batch
time divided by depth. RESP3/mTLS semantic adapters are now available; sealed
numerical execution remains absent.

Native 32/128 fixtures synchronize GET/PUT clients with a barrier, verify real HC2
connection accounting, exact values and shutdown. They assert no real-clock
performance threshold or native nonregression.

`start_connections` additionally supports 1/8/32/128 actual accepted TCP sockets
and GET or SET of the same fixed preloaded payload. Each socket owns its FIFO,
parser, permits and local wire ordinal; `(connection_id, wire_ordinal)` is the
wire identity. Sequence modulo socket count fixes the route before any response.
The fixture store is shared within a control, and setup/final GET byte oracles
check it through every connection. SET success checks exact `+OK`; final GET
checks retained bytes. No global socket ordering or conflicting-write oracle is
claimed. Wire drain has one five-second group budget. This remains semantic-only,
with no numerical series or B0 retry. Separate multi-key semantic
controls are described below; they do not add a global cross-socket order claim.

Multi-key controls now support `Mget`, fixed-value `Mset`, live-key `Exists` and
`DelMissing` at batch sizes 1/8/32/128. One command has one original offer and one
reply timestamp, including an entire MGET array. `response_items` records actual
reply shape (array length or one scalar), not an operation-count multiplier.
Flat bulk/null arrays are borrowed and bounded to 128 items within the unchanged
1,048,708-byte reply buffer; nested/unknown reply forms fail. Aggregate requests
above 1,052,672 bytes or unsupported counts fail before connecting/copying payloads.
The product default 128-entry limit is unchanged: 256 is rejection evidence, not
a successful workload obtained by changing defaults. Scripted live DEL/duplicate
and concurrent MSET/MGET fixtures are separate from scheduled missing-key DEL.

Independent native batch controls use `BatchGet`/same-fixed-value `BatchPut` on
direct ClientSurfaceState, HC1 HTTP and HC2 gRPC/mTLS. The direct control creates
no listener; it calls the production verified dispatcher with its own state.
All use the same binary dataset, batch sizes 1/8/32/128 and client slots 1/8/32/128.
One batch invocation remains one offer/sample, never latency divided by key count.
Aggregate logical value plus canonical-key bytes are capped at 1 MiB before setup
or repeated payload construction; this is a tool bound, not a new product limit.

HC1/direct use one atomic surface BatchPut dispatch. HC2's existing batch iterates
single-key requests: one dispatch per item, with no batch-wide atomicity claim.
Its SDK default accepts 256 items, unlike the surface's 128-entry batch limit;
the scheduled controls deliberately remain capped at 128. Negative HC1/direct
256-entry mutation checks and HC2 256-success/1025-SDK-rejection checks keep that
difference visible. A mixed HC2 PUT/unapplied-CAS fixture retains the first write;
it is not an atomic MSET oracle. No product path is changed to hide these costs
or semantic differences. Numerical native floors, matched secure RESP controls,
live DELETE/EXISTS and expiration/quota/event/fault matrices remain open.

Plaintext RESP3 is now an explicit `Dialect::Resp3` control, not automatic
fallback. `start_connections_dialect` validates HELLO 3 on every socket before
preload and scheduled offers. Its setup-only metadata reader is capped at 4 KiB,
seven required unique fields, a 64-byte printable version and a five-second
negotiation safety timeout. Map field order is irrelevant; unknown/duplicate
fields or wrong types/values fail. The version is bounded metadata, not a binary
identity or release receipt. HELLO/setup is not counted as measured goodput.

RESP3 scalar/array null uses `_\r\n`, while RESP2 retains `$-1\r\n`. Each parser
refuses the other dialect's null, and records carry a validated dialect plus
negotiated connection count/version. `GetMissing` adds a separate all-miss cell.
The normal reply grammar still rejects maps, pushes, attributes and nested arrays;
HELLO metadata is a separate shallow reader. Fifty-six RESP3 scheduled cells cover
seven operations and socket/depth boundaries; separate tests cover 1 MiB GET/SET,
mixed HELLO transitions, fragmentation, null/empty/binary/duplicate MGET and
cancelled array owners. This is not RESP3-wide conformance, TLS, numeric native
nonregression or release qualification. Secure adapter coverage is described below.

Historical capability audit at `2f6db5a4`: the Redis acceptor built server-auth
TLS with `with_no_client_auth()`; Redis AUTH supplies the listener-bound identity.
The approved opt-in product extension at `8565dbfd` now requires a client
certificate when an explicit Redis client CA is configured; legacy server-auth
TLS is unchanged. `RespControl::start_mtls` uses that production runtime factory
and accept loop, then AUTH on every socket before HELLO/preload. HC2 and RESP
can share ephemeral fixture PKI; certificate fingerprint equality does not imply
identical application authorization, batch atomicity, negotiated cipher or
numeric admission. Explicit shutdown joins owners and checks active connections.

Allocation diagnostics have fixed preload/GET/SET/idle/delete/refill/shutdown
phases and exact bounded arguments. The System counter implementation is source
referenced from the earlier scratch tool, never linked into the timing library.
RSS is whole-process working set/VmRSS; allocator active/resident/retained stays
unavailable, not zero or green. See the separate secure-observer-memory-checks
contract/design; commit and seal source/lock/binary before fresh-process execution.

`EmbeddedControl` separately calls public HydraCache encoded GET/PUT. It creates
no listener or ClientSurfaceState, has deterministic 1/8/32/128 logical client
slots, the same raw binary corpus digest and an injective fixture-only hex key
mapping. It checks byte oracles, public delete/refill and flush/shutdown; the
cache's approximate entry count is not allocator retention. Batch operations are
refused rather than relabeled atomic. This closes a semantic adapter gap only;
there is still no unprofiled timing CLI or full-D3 numerical series.
