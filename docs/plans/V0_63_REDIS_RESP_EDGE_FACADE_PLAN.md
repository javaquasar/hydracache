# HydraCache 0.63.0 Redis RESP Edge Facade — Codex Execution Plan

> **At a glance**
> - **What:** an **optional, off-by-default edge server mode** (`hydracache-redis-compat`) that speaks
>   the **Redis RESP wire protocol** for the subset of commands that maps cleanly to HydraCache cache
>   semantics, so polyglot stacks can point **existing mainstream Redis clients** at HydraCache by
>   changing a connection string — **not** by rewriting cache code. It **translates** RESP into the
>   `ClientRequest`/`ClientResponse` family and **reuses** the client-surface execution layer
>   (tenancy, limits, consistency); the expanded 0.63 scope registers a targeted
>   `hydracache-client-protocol` v3 extension for TTL metadata, but it does **not** re-implement
>   cache access and does **not** make HydraCache a Redis clone.
> - **Why:** the honest #1 remaining weakness is **adoption reach for non-Rust stacks** (POSITIONING:
>   SDK breadth is Rust + Python; no drop-in wire). A RESP facade is the single highest-leverage
>   *outward* step: "change the connection string, not the code." It is an **edge crate** — the core
>   API and embedded fast path are untouched, so it is compatible with a later `1.0` API freeze.
> - **After (depends on):** `0.49` (`hydracache-client-protocol` `ClientRequest`/`ClientResponse`,
>   `ClientRequestEnvelope`), `0.48`/`0.49` client surface (`hydracache-client-transport-axum`
>   `ClientSurfaceState`, tenancy/limits/auth), `0.62` (correctness-hardened grid underneath).
> - **Blueprint:** `CROSS_PROJECT_REREAD_IMPROVEMENT_PLAN.md` §"Redis-Compatible Protocol Facade" +
>   Redis `src/tracking.c`/`notify.c` as *reference*, not as a feature list; Redis Cluster / async
>   replication are explicit **anti-references**.
> - **Sequencing note:** this is **outward adoption before/around `1.0`**. Because it is an edge crate
>   that never touches the frozen core, a subsequent `1.0` stabilization can proceed independently.
> - **Status:** in-progress; scope expanded on 2026-07-10 to include the six remaining
>   compatibility-proof items plus mandatory `MSET` atomicity, Redis TTL support through a
>   registered client-protocol v3 extension, Redis `AUTH`/`HELLO AUTH`, and native `rediss://`
>   support before the release can close. A semantic accuracy audit pass was added on 2026-07-11
>   (see "Semantic Accuracy Audit Pass") after a post-implementation review of command return
>   semantics; it does not widen the supported surface, it closes accuracy gaps in the already-claimed
>   surface before ship.
>
> Roadmap: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> positioning: [`../POSITIONING.md`](../POSITIONING.md) ·
> blueprint: [`CROSS_PROJECT_REREAD_IMPROVEMENT_PLAN.md`](CROSS_PROJECT_REREAD_IMPROVEMENT_PLAN.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its Definition of
Done **and** `cargo xtask verify`; never push red. Per-step: targeted `build`/`clippy`/`test` of the
changed crate + downstream; full `verify` at merge/tag.

## Preflight (verified against the repo at `0.62.0`)

- **The translation target already exists.** `crates/hydracache-client-protocol/src/lib.rs`:
  `ClientRequest` (lib.rs:612) = `Get { ns, key }`, `Put { ns, key, value, ttl_ms?, dimensions }` (:616),
  `Invalidate { ns, key }` (:624), `BatchGet` (:626), `BatchPut { entries }` (:631),
  `CompareAndSet` (:678), plus the lock family; `ClientResponse` (:795); `ClientRequestEnvelope`
  (:552); `Namespace` / `StructuredKey`. Protocol is postcard, currently `PROTOCOL_VERSION = 2`.
  **The expanded release adds protocol v3 only for TTL metadata and explicit expiry operations; the
  facade still maps RESP verbs onto native client-surface operations instead of inventing an
  alternate cache path.**
- **The execution layer to reuse (do NOT bypass) exists.**
  `crates/hydracache-client-transport-axum/src/lib.rs`: `ClientSurfaceState` (lib.rs:283) turns a
  request into a `ClientResponseEnvelope` (:491/:704/…), applying **tenancy** (`TenantStatus`,
  quotas), **limits** (`ClientSurfaceLimits`, :75, `max_value_bytes`/`max_batch_*`), auth identity
  (`HYDRACACHE_CLIENT_ID_HEADER`/`TENANT_HEADER`), and the lock/CAS semantics. The RESP facade drives
  this in-process — it does **not** re-serialize over the wire and does **not** call the cache
  directly, so tenancy/limits/consistency are preserved for free.
- **The TTL/tag claims require explicit protocol and execution work.**
  `ClientRequest::Put` already carries `ttl_ms`, but the current client-surface dispatch drops that
  field (`ttl_ms: _`) before `handle_put`, and `handle_put` stores only value bytes. `ClientResponse`
  returns `Value { value }` without remaining-TTL metadata. The 2026-07-10 expansion makes
  `SET EX/PX`, `SETEX`/`PSETEX`, `EXPIRE`/`PEXPIRE`, `PERSIST`, `TTL`, and `PTTL` mandatory release scope, so W0/W2 must
  register protocol v3, make the client surface apply TTL, and expose enough metadata to answer
  remaining TTL before these commands can be marked done. Similarly, `Put` carries
  `dimensions: Vec<String>` rather than a proven RESP tag contract, and `BatchPutEntry` carries only
  key/value bytes. W0 turns tag assumptions into release blockers before W3 implementation.
- **There is no server-side RESP codec in the workspace.** No `redis-protocol` / `redcon` /
  server-`redis` in any `Cargo.toml` (the `redis` crate is used only as a *client* by
  `hydracache-transport-redis` for the invalidation transport). W1 adds a RESP codec as a new,
  contained dependency of the new edge crate.
- **The server already runs role-scoped listeners on separate ports.**
  `crates/hydracache-server/src/config.rs`: `ClientApiConfig` (client surface),
  `AdminApiConfig { listen_addr = 127.0.0.1:9091 }` (config.rs:74-83), with `validate()` rejecting
  overlapping listen addresses (`AdminAddressConflicts`, config.rs:247-249). **The RESP facade is a
  third, off-by-default listener** (default `127.0.0.1:6379`) with the same distinct-address rule.
- **Lifecycle/security to reuse:** `0.48` mTLS + graceful upgrade, `0.56` `graceful_shutdown`
  (bootstrap.rs) — the RESP listener drains through the same path, not a bespoke one.

## Release Theme

Let existing Redis clients talk to HydraCache for the **cache subset**, honestly: RESP in, native
`ClientRequest` execution (tenancy/limits/consistency) underneath, HydraCache-native concepts
(tags/invalidation/diagnostics) exposed as explicit `HC.*` extension commands, everything outside the
subset **failing loud**. An edge accelerator — **not** a Redis clone, **not** Redis Cluster, and only a
targeted client-protocol v3 expansion for TTL metadata rather than a new cache API family.

## Non-Goals

- **Not "become Redis."** RESP is a wire-compat layer, not a product identity. Unsupported data
  structures (`HSET`/`ZADD`/lists/streams/Lua/`MULTI`/modules) fail loud with a stable
  `ERR unsupported command` (R-3) — never silently wrong.
- **No Redis Cluster.** No `MOVED`/`ASK` redirections, no hash slots, no gossip authority — authority
  stays **raft + epoch** (R-1). Redis Cluster and async replication are **anti-references**, not to be
  copied. `CLUSTER SLOTS`, `CLUSTER NODES`, `CLUSTER INFO`, and related cluster topology commands
  must return the stable unsupported-command error; cluster-aware clients must connect in standalone
  Redis mode.
- **Does not replace `hydracache-client-protocol`.** The stable frame contract (namespace, structured
  keys, idempotency, consistency labels, locks/CAS, residency, versioned compat) stays authoritative;
  the facade *translates into it*. The only protocol change in the expanded 0.63 scope is a registered
  v3 additive extension for TTL metadata and expiry operations.
- **No raw prefix-invalidation over internal binary keys.** Tag/structured invalidation is expressed
  only through explicit `HC.*` extension commands or configured namespace/tag conventions.
- **Pub/Sub is not a general message bus.** If offered at all, scoped to **invalidation notifications
  only**; otherwise unsupported-loud (not an event log — R-9).
- **Off by default; core untouched (R-10).** The listener is opt-in; embedded caching stays unchanged.
  The client-surface path changes only to honor protocol v3 TTL semantics that are already part of
  HydraCache's cache model. Edge crate → compatible with a later `1.0` freeze.
- **No Hazelcast facade here.** That is a separate, heavier future plan (member illusion).

## Release Strengthening Pass

This pass is mandatory scope, not reviewer advice. A wire-compat release is only useful if mainstream
clients can connect **and** every accepted command means what Redis clients expect it to mean. The
implementation must therefore prove semantic compatibility before claiming command support. If a
command cannot be implemented exactly through the current HydraCache execution layer, the command is
documented as `unsupported` or `candidate`, and the facade returns a stable loud RESP error.

1. **Add W0 before the crate work: a semantic capability audit.** W0 owns a command-by-command contract
   table with the Redis behavior, HydraCache target operation, exact RESP reply shape, caveats, auth
   requirements, and covering tests. This prevents W2 from accidentally treating "parsable" as
   "compatible."
2. **Treat TTL as a release-blocking protocol expansion, not a free mapping.** Current preflight shows
   that `ttl_ms` is present in the protocol but not enforced by the client surface, and the response
   shape lacks remaining-TTL metadata. The expanded release requires W0/W2 to land a registered
   protocol v3 TTL metadata path, real expiry enforcement, and Redis-compatible `SET EX/PX`, `SETEX`/`PSETEX`,
   `EXPIRE`/`PEXPIRE`, `PERSIST`, `TTL`, and `PTTL` behavior.
3. **Separate command support levels.** The docs matrix must distinguish `supported`, `supported with
   caveat`, `HydraCache extension`, `admin-disabled`, `candidate`, and `unsupported`. A command cannot
   be `supported` if it returns the wrong count, has weaker atomicity, mutates through a different
   tenant scope, or hides a partial failure.
4. **Make mainstream-client compatibility a matrix.** `redis-rs` is the fast proof. Docker/nightly
   smoke should also exercise at least one Python client, one Node client, one Go client, and one JVM
   client, because their startup handshakes differ (`HELLO`, `AUTH`, `CLIENT SETINFO`, `COMMAND`,
   pipelining, connection naming, and protocol fallback).
5. **Add pipelining, partial-frame, and backpressure gates.** RESP clients commonly send multiple
   commands before reading responses. The listener must preserve response order, handle partial frames,
   reject oversized bulk/array frames before allocation spikes, time out slowloris connections, and
   bound in-flight work per connection.
6. **Expand W6 into a full release-ledger work item.** W6 is not only "wire the listener and write
   docs." It must update `COMPAT.md`, `GATES.md`, `TESTING.md`, CI/nightly naming, release manifests,
   docs matrix, observability labels, security redaction notes, and backlog closure in one coherent
   release evidence pack.
7. **Make `HC.*` extensions stricter than the plain Redis subset.** `HC.INVALIDATE_TAG` and tag
   mutation commands ship only if they map to a real HydraCache tag/dimension operation with
   falsifiable tests. A scan-and-loop over keys is not tag invalidation, and a fake implementation is
   worse than leaving the command unsupported.
8. **Add performance/resource smoke without making this a benchmark release.** The gate should prove
   bounded memory and file descriptors under many idle and pipelined connections, bounded metric label
   cardinality, no key/value leakage in logs or metrics, and no unbounded allocation on hostile RESP
   frames.
9. **Make authentication and native Redis TLS release-blocking.** `AUTH` and `HELLO AUTH` must work for an
   auth-required listener with Redis-shaped `NOAUTH`/`WRONGPASS`/`OK` behavior before `0.63.0` can
   claim mainstream Redis-client compatibility. Native Redis TLS is mandatory in the same release:
   `rediss://` must be backed by the existing server TLS certificate/key material, must reject
   plaintext on TLS listeners, must fail loud for wrong CA or wrong AUTH, and must prove a successful
   `rediss://` Redis-client path before production exposure is documented.

## Scope Expansion: Remaining 0.63.0 Proof Items

This section records the explicit scope expansion for the current implementation branch. The release
still does **not** claim Redis Cluster or global/persisted tag-scoped invalidation; the 0.63 tag work is
limited to the documented RESP-listener-local `HC.*` extension path. The expansion closes the
compatibility proof around the supported RESP2/RESP3 cache subset and adds five mandatory 0.63 work
streams: atomic `MSET`, TTL/expiry commands backed by a registered client-protocol v3 extension, Redis
startup security (`AUTH`/`HELLO AUTH`), native Redis TLS (`rediss://`) on the RESP listener, and RESP3
negotiation for the same cache subset.

The following five items are now mandatory release scope in addition to the six proof items below:

1. **Atomic `MSET`.** `MSET key value [key value ...]` is mandatory supported scope and closes only
   when it is executed through `ClientSurfaceState` as an atomic batch. The batch path must validate arity,
   total batch limits, per-item value limits, tenant quota, and duplicate-key ordering before any
   mutation. Redis duplicate-key semantics are required: later values in the same command win. A
   rejected command must leave all touched keys unchanged, including keys written earlier in the same
   command. The facade returns Redis `OK` only after the whole batch is applied.
2. **Protocol v3 TTL and expiry support.** `SET key value EX seconds`, `SET key value PX milliseconds`,
   `EXPIRE`, `PEXPIRE`, `PERSIST`, `TTL`, and `PTTL` are mandatory supported scope and close only after
   `hydracache-client-protocol` registers v3 additive request/response shapes for expiry mutation and
   remaining-TTL metadata. `ClientSurfaceState` must store value bytes together with optional expiry,
   apply TTL on write, remove expired keys before reads/counts/batch reads, expose remaining TTL using
   Redis semantics (`-2` missing, `-1` no expiry, positive seconds/milliseconds when expiring), and
   preserve existing v2 behavior for clients that do not negotiate v3.
3. **Redis `AUTH`/`HELLO AUTH`.** `AUTH password`, `AUTH username password`, and
   `HELLO 2 AUTH username password` are mandatory release scope for auth-required RESP listeners.
   The listener keeps unauthenticated loopback development possible by default, but when auth is
   configured every data or mutating command fails with Redis-shaped `NOAUTH` until successful
   authentication. Invalid credentials return `WRONGPASS` without leaking usernames, passwords,
   connection strings, tenant names beyond the configured public identity, or raw client metadata.
   Successful auth binds the connection-local Redis identity to a HydraCache client-surface
   `ClientIdentity`/tenant before any cache request is dispatched.
4. **Native Redis TLS / `rediss://`.** `rediss://` is now mandatory supported scope for the optional
   RESP listener. The server config must enable Redis TLS explicitly, reuse the existing server TLS
   certificate/key material by default, and keep Redis `AUTH` as the application-layer protection
   above TLS. Implementation must add a TLS acceptor around the raw RESP TCP listener without changing
   RESP2 command semantics. Tests must cover a successful TLS handshake followed by `AUTH`, `PING`,
   `SET`, `GET`, `MSET`, and TTL commands; plaintext sent to a TLS listener must fail before mutation;
   a client using the wrong CA must fail the handshake; invalid Redis credentials over TLS must still
   return `WRONGPASS`; and logs/metrics must not expose connection-string credentials or certificate
   material. The Docker/client matrix must include at least one `rediss://` Redis-client path and docs
   may show `rediss://` examples only after that gate exists.
5. **RESP3 negotiation and cache-subset codec.** `HELLO 3` is now mandatory supported scope for the
   optional RESP listener. A successful `HELLO 3` switches only that connection into RESP3 response
   encoding while preserving the same cache subset, auth, rediss, MSET, TTL, unsupported-command, and
   tenant/limit semantics as RESP2. `HELLO 3 AUTH username password` must authenticate before sending
   the handshake response. RESP3 command arrays with blob/simple string arguments decode to the same
   parser-neutral `RedisCommand` model, and representative cache commands (`PING`, `SET`, `GET`,
   `MSET`, `MGET`, `TTL`/`PTTL`, `QUIT`) must round-trip after negotiation. RESP3 `Map`, `Set`, `Push`,
   attributes-as-command, and nested non-string argument shapes remain unsupported and must fail loud
   before mutation; Redis Cluster and non-cache data structures remain out of scope.

The following six items are now mandatory release scope:

1. **Real Redis oracle.** The `redis_clients` gate must run the supported subset against HydraCache and
   pinned real Redis oracle images (`redis:6.2.14`, `redis:7.2.5`) and compare normalized replies for
   `PING`, `ECHO`, `GET`, `SET`, `MGET`, `DEL`, `EXISTS`, `MSET`, and the supported TTL commands.
2. **Mainstream client matrix.** The `redis_clients` gate must keep the Rust `redis-rs` smoke and add
   executable, gated Python, Node, Go, and JVM rows. Each client row must exercise plain `MSET`,
   `SET EX` or `SET PX`, at least one TTL read (`TTL` or `PTTL`), and an auth-required connection
   scenario in addition to the existing cache subset. The matrix must also include a `rediss://`
   startup scenario using pinned test TLS material. Missing runtimes skip loud only when the nightly
   gate is not explicitly enabled for that runtime.
3. **Executable heavy gates.** The `redis_clients` and `resp_resource_smoke` targets must compile in
   the fast tier and run their ignored scenarios only when the documented env vars are set.
4. **Reconnect and failure semantics.** RESP listener tests must cover close mid-command, close
   mid-pipeline, reconnect-and-retry, and drain during pipeline without response corruption or
   connection-local state leakage.
5. **Multi-node RESP e2e.** A network-gated server test must drive the RESP listener against a real
   daemon/grid path and exercise at least one restart or drain boundary. The fast tier may compile the
   harness and keep the scenario ignored/env-gated.
6. **Executable docs/examples.** Examples in `docs/integrations/redis-compat.md` must either be
   executable in a docs-smoke gate or explicitly labeled with the Docker/nightly gate that proves them.

Each item remains one closed task with its own commit. Before each commit, run the targeted tests for
the changed crate or docs gate. The final release still requires the global gates after this branch is
merged.

## Additional Strengthening Pass: executable compatibility contract

The first strengthening pass prevents silent semantic drift. This second pass makes the compatibility
contract executable across code, docs, real Redis, real clients, multi-node HydraCache, and operations.
Each item below is tied to an existing work item so it becomes testable release scope rather than a
loose reminder.

1. **Conformance manifest (W0/W5/W6).** Add a versioned
   `redis_compat_conformance.{json,yaml}` manifest that is the single source of truth for supported,
   candidate, admin-disabled, HydraCache-only, and unsupported commands. The manifest feeds the docs
   matrix, golden fixtures, translator contract tests, real Redis oracle scenarios, mainstream-client
   smoke, and release-note command table. No command can be implemented, documented, or tested through
   an ad hoc list that diverges from this manifest.
2. **Pinned real Redis oracle versions (W5/W6).** The real Redis oracle must run against pinned Docker
   image tags, never `latest`. The plan should test at least one baseline Redis 6.x image and one
   Redis 7.x image unless the release explicitly narrows the compatibility claim. Updating oracle
   versions is a reviewed compatibility change with a changelog note, because real Redis behavior,
   `COMMAND` metadata, `HELLO`, and error text can evolve.
3. **RESP2/RESP3 negotiation (W0/W1/W2/W5/W6).** RESP2 and RESP3 are the `0.63.0` supported wire
   dialects for the same cache subset. `HELLO 2` must produce an honest RESP2-compatible handshake.
   `HELLO 3` must switch the connection to RESP3 replies, and `HELLO 3 AUTH username password` must
   authenticate before the handshake response. RESP3 command arrays using blob/simple string arguments
   decode to the same parser-neutral `RedisCommand` model. Unsupported RESP3 aggregate command frames
   (`Map`, `Set`, `Push`, attributes as commands, or nested non-string arguments) are rejected before
   mutation. The docs and oracle normalization must say exactly what is compared under RESP2 and RESP3.
4. **Multi-node HydraCache e2e (W5/W6).** Add a gated test that drives the RESP facade against a real
   multi-daemon HydraCache grid, not only an in-process state. The test writes through RESP, reads
   through RESP, exercises at least one leader restart/drain or node restart path, and proves the RESP
   edge still goes through tenancy, limits, and consistency rather than bypassing the cluster surface.
5. **Executable docs (W6).** Every copy-paste example in `docs/integrations/redis-compat.md` must be
   executable as a docs-smoke test. That includes `redis-cli`, Rust, Python, Node, Go, and JVM examples
   when the corresponding client matrix row is claimed. The expanded release requires executable
   examples for `MSET`, TTL, `SELECT 0`, `rediss://`, and RESP3 negotiation. Docs cannot show
   `HC.*` examples or non-zero `SELECT` examples unless the matching gates are green.
6. **Reconnect and connection-failure semantics (W5).** Add tests for close mid-command,
   close mid-pipeline, reconnect-and-retry, server drain during pipeline, and malformed response
   boundaries. A failed connection must not corrupt the next response, leak connection-local namespace
   state across connections, or apply an ambiguous partial write without idempotency evidence.
7. **Client health-check commands (W0/W2/W5).** W0 must explicitly classify common framework probes:
   `INFO`, `ROLE`, `DBSIZE`, `TYPE`, `SCAN`, `CONFIG`, `CLIENT LIST`, `CLIENT ID`, and related
   health/readiness commands. `INFO` and `TYPE` are supported-with-caveat probe commands only:
   `INFO` returns minimal honest RESP-facade facts, and `TYPE` returns `string`/`none` for the cache
   subset. `ROLE`, `DBSIZE`, and `SCAN` stay unsupported-loud because replication role, exact
   keyspace size, and iterable keyspace state would fabricate or overexpose Redis server behavior.
   A framework should never pass readiness because HydraCache returned a fabricated Redis server
   state.
8. **Config/operator packaging (W6).** The server config, sample configs, Helm/operator docs, and any
   production guide must prove the RESP listener is disabled by default, not exposed by default, and
   enabled only by explicit config. If Kubernetes/operator packaging exists, the plan must cover port
   exposure, service annotations, TLS/auth secrets, NetworkPolicy guidance, and rollback defaults.
9. **Oracle normalization rules (W0/W5/W6).** The real Redis oracle must define what is compared
   exactly and what is normalized. Integer counts, nil/bulk shape, array order, atomic `MSET` outcome,
   and success/failure class match exactly for supported commands. Error text may be normalized by
   class/code. TTL values compare with bounded tolerance because TTL support is now required scope.
   Unsupported divergence is allowed only when the conformance manifest says HydraCache is
   intentionally not Redis-compatible for that row.
10. **Rollout/rollback playbook (W6).** Add an operator-facing playbook for canary enablement,
    production monitoring, rollback triggers, and disable procedure. It must name the metrics and
    audit events to watch, how to turn off the listener safely, whether restart is required, what to do
    with existing connections during drain, and which failures should immediately revert the facade.

## Dependency Graph

```
W1 crate scaffold + RESP codec + listener config ─► W2 command translator (cache subset) ─────┐
                                                     W3 HC.* extension commands ──────────────┼─► W6 server wiring + docs + gates
                                                     W4 unsupported matrix + guardrails ──────┤
                                                     W5 golden fixtures + client smoke + fuzz ┘
```

Strengthened execution order:

```text
W0 semantic capability audit + command contract
  -> W1 RESP crate/listener/codec
  -> W2 Redis cache-subset translator, only for commands W0 marks supportable
  -> W3 HC.* extensions, only for native HydraCache operations W0 proves real
  -> W4 unsupported/admin-disabled matrix generated from the same contract
  -> W5 golden/client/fuzz/pipeline/resource proof
  -> W6 expanded release ledger: docs + COMPAT + GATES + TESTING + CI + backlog
```

W0 is a hard predecessor for W2/W3/W4/W5/W6. W1 can scaffold the crate and listener in parallel, but
no command is advertised or documented as supported until W0 assigns it a support level and a test row.

## W0. Semantic capability audit + command contract

**Goal.** Build the release contract before implementing command behavior. The output is a reviewed
matrix that says exactly what each Redis command means at the HydraCache edge, which commands are
accepted, which are rejected, and which accepted commands require extra native support before they can
graduate from candidate to supported.

**Files.** `docs/integrations/redis-compat.md` (initial supported/unsupported matrix),
`docs/integrations/redis_compat_conformance.json` (or `.yaml`, the versioned executable contract),
this plan, `docs/COMPAT.md` draft row, `docs/GATES.md` draft rows, oracle-normalization notes, and
optional fixture data under `crates/hydracache-redis-compat/tests/fixtures/commands/`.

**Command contract columns.** Each command row must include:
- **Command / arity / protocol form:** for example `SET key value [EX seconds|PX milliseconds]`.
- **Redis expectation:** return type, nil/missing behavior, count semantics, atomicity, error shape,
  auth state, and any startup-handshake behavior mainstream clients depend on.
- **HydraCache target:** `ClientRequest`, client-surface helper, diagnostics read model, or
  unsupported-loud. If the target does not exist, the row is `candidate`, not `supported`.
- **Semantic status:** `supported`, `supported-with-caveat`, `candidate`, `HydraCache-extension`,
  `admin-disabled`, or `unsupported`.
- **Exact RESP response:** including integer counts (`DEL`, `EXISTS`), bulk nils (`GET`, `MGET`),
  simple strings (`OK`, `PONG`), and stable errors.
- **Tenant/auth behavior:** whether the command needs authenticated identity, tenant scope, admin
  scope, or is safe before auth (`HELLO`, selected `CLIENT` metadata commands).
- **Limits/backpressure behavior:** max bulk size, max array length, per-connection in-flight cap,
  timeout, and rate/quota failure mapping.
- **Oracle normalization:** whether real Redis comparison is exact, normalized by error class, skipped
  because HydraCache intentionally diverges, or compared with bounded tolerance (TTL only).
- **Covering tests:** at least one named test for every supported/candidate row.

**Release-blocking semantic decisions.**
1. **TTL:** `SET EX/PX`, `SETEX`/`PSETEX`, `EXPIRE`/`PEXPIRE`, `PERSIST`, `TTL`, and `PTTL` are mandatory release scope.
   W0 registers `hydracache-client-protocol` v3 as an additive extension with explicit expiry
   mutation and remaining-TTL metadata. W2 cannot close until the client surface applies TTL on write,
   expires keys before reads/counts, exposes Redis remaining-TTL semantics, and keeps v2 clients
   backward-compatible.
2. **`DEL` / `EXISTS` counts:** accepted only if the translator returns Redis-style integer counts.
   `DEL` must count keys actually removed. `EXISTS` must count keys currently present. Returning only
   `OK` or boolean is not compatible.
3. **`MSET` atomicity:** mandatory release scope. The batch path admits the whole batch before
   mutation and cannot partially store entries. If any entry violates limits or shape, all touched
   keys remain unchanged. Duplicate keys follow Redis order, with the last value winning.
4. **`MGET` ordering:** supported only if the response preserves request order and represents misses
   as nil bulk entries.
5. **`SELECT` / Redis multi-db:** `SELECT 0` is supported as a connection-local no-op for mainstream
   Redis client URL compatibility (`redis://host:6379/0`). HydraCache `0.63.0` exposes exactly one
   logical Redis database backed by the configured HydraCache namespace. `SELECT 1`, every other
   non-zero DB index, negative indexes, and malformed indexes fail loud before mutation; a failed
   `SELECT` must not change the connection keyspace or dispatch through `ClientSurfaceState`.
6. **`COMMAND`:** mainstream clients often issue `COMMAND` during bootstrap. It can return a minimal
   honest command table for the supported subset, but it must not advertise unsupported commands.
7. **`AUTH` / `HELLO AUTH`:** mandatory release scope. W0 defines the exact identity mapping for
   `AUTH password`, `AUTH username password`, and `HELLO 2 AUTH username password`, including
   connection-local authenticated state, tenant/client-id derivation, `NOAUTH`, `WRONGPASS`, `OK`,
   and credential redaction in logs. The conformance manifest may remain `candidate` only while the
   implementation is absent; before release close, the row must graduate to supported with the same
   tests that prove the listener rejects unauthenticated data commands.
8. **Dangerous/admin commands:** `CONFIG`, `FLUSHDB`, and `FLUSHALL` are recognized but
   `admin-disabled` by default. `CONFIG` is a Redis server-administration surface for reading or
   changing runtime configuration; the facade must not fabricate Redis memory, persistence, TLS, ACL,
   replication, or module configuration, and it must not accept `CONFIG SET` as if those settings
   existed. `FLUSHDB` deletes the selected Redis database, and `FLUSHALL` deletes every Redis database;
   under HydraCache's `SELECT 0`-only compatibility contract these would become broad destructive
   tenant/namespace operations rather than normal cache-subset commands. The default behavior is a
   stable `NOPERM ... is disabled by the HydraCache Redis facade` error before dispatch or mutation.
   Any future wipe/config capability must be HydraCache-native admin API scope with explicit
   authorization, audit, and rollout gates, not a Redis-compatible default.
9. **RESP negotiation:** `HELLO 2` and `HELLO 3` are supported dialect negotiations for `0.63.0`.
   `HELLO 3` must enter an explicit RESP3 connection mode with RESP3 reply encoding for the same cache
   subset. Unsupported RESP3 aggregate command frames must fail loud before mutation; the listener must
   not accidentally enter a half-RESP3 mode where negotiation succeeds but replies stay RESP2.
10. **Health/readiness commands:** classify `INFO`, `ROLE`, `DBSIZE`, `TYPE`, `SCAN`, `CONFIG`,
   `CLIENT LIST`, `CLIENT ID`, and similar framework probes. `INFO` is supported only as a minimal
   bulk-string facade snapshot: standalone mode, role, HydraCache package version, supported RESP
   dialects, accepted connection count, processed command count, and RESP error count. It must not
   include Redis memory, DB keyspace, replication offsets, cluster topology, or other fabricated
   internals. `TYPE key` is supported only for the cache subset and returns `string` for a present
   value or `none` for a miss. `ROLE`, `DBSIZE`, and `SCAN` stay unsupported because Redis replication
   role, exact keyspace cardinality, and iterable keyspace semantics are not safe or honest claims for
   this edge.
11. **Conformance manifest ownership:** the command matrix in `redis-compat.md`, the test fixtures,
   the real Redis oracle, and the release-note supported-command table must be generated from or
   checked against the same versioned manifest. Hand-maintained duplicate command lists are release
   blockers.

**Tests & requirements.**
- `redis_command_contract_has_no_supported_row_without_test`.
- `client_protocol_v3_registers_ttl_metadata_without_breaking_v2`.
- `ttl_commands_require_protocol_v3_metadata_and_surface_expiry`.
- `del_and_exists_return_redis_integer_counts`.
- `mset_is_atomic_and_duplicate_keys_use_last_value`.
- `command_reply_advertises_only_supported_subset`.
- `select_zero_is_supported_as_noop_for_single_database_contract`.
- `select_nonzero_and_invalid_db_fail_loud`.
- `resp_listener_select_zero_ok_and_nonzero_keeps_default_database`.
- `info_returns_minimal_honest_facade_state`.
- `info_section_argument_does_not_fabricate_redis_keyspace_state`.
- `resp_listener_info_probe_does_not_fabricate_keyspace_or_cluster_state`.
- `type_reports_string_or_none_through_client_surface`.
- `resp_listener_type_reports_string_and_none`.
- `mainstream_redis_client_can_talk_to_the_facade` proves `INFO`/`TYPE` through redis-rs.
- `nightly_python_node_go_jvm_clients_bootstrap_and_run_supported_subset` proves `INFO`/`TYPE`
  through Python, Node, Go, and JVM clients.
- `admin_commands_are_disabled_by_default_without_config_or_flush_mutation`.
- `resp_listener_admin_commands_are_disabled_before_mutation`.
- `auth_hello_auth_and_noauth_errors_match_contract`.
- `redis_auth_required_listener_rejects_data_commands_before_auth`.
- `redis_auth_redacts_credentials_from_errors_logs_and_metrics`.
- `redis_api_rediss_env_reuses_server_tls_material`.
- `redis_resp_listener_accepts_rediss_auth_and_cache_commands`.
- `redis_resp_tls_listener_rejects_plaintext_before_mutation`.
- `redis_resp_tls_client_rejects_wrong_ca`.
- `redis_resp_tls_keeps_wrong_auth_as_wrongpass`.
- `redis_compat_conformance_manifest_is_the_single_source_of_truth`.
- `resp2_hello_is_supported_and_resp3_is_rejected_or_downgraded_as_documented`.
- `health_check_commands_are_classified_before_translation`.
- `oracle_normalization_rules_are_declared_for_every_supported_command`.
- Run: `cargo xtask doc-check`, plus the initial `hydracache-redis-compat` contract tests once W1
  creates the crate.

**Risk & rollback.** W0 is documentation and tests first. If it reveals a command is too expensive or
semantically unsafe, the rollback is to keep that command unsupported-loud and ship a smaller, honest
facade.

## W1. `hydracache-redis-compat` crate + RESP codec + listener config

**Goal.** A new edge crate + an off-by-default RESP listener, with a **parser-independent** command
pipeline so the codec choice is swappable.

**Files.** new `crates/hydracache-redis-compat/` (`publish` decision: it is a real server-mode library
— either publishable + listed in the publish scripts, or `publish = false` with a reason; doc-check
`publishable-crate` gate enforces the choice), `crates/hydracache-server/src/config.rs`
(`RedisApiConfig { enabled: false, listen_addr: 127.0.0.1:6379 }` + distinct-address validation),
root `Cargo.toml` `[workspace].members`.

**Codec decision (blueprint §"Existing Rust Building Blocks").** Production codec = **`redis-protocol`**
(RESP2 and RESP3 — `Bytes`-based, streaming, Tokio codec, fuzzable) under a **HydraCache-owned
Tokio listener**, so lifecycle/TLS/auth/metrics/backpressure/drain stay consistent with
`hydracache-server`. `redcon` may be used only for a throwaway PoC; do **not** hand-write RESP parsing.
Vet license/maintenance/RESP2+3/zero-copy before pinning; record the ADR (`docs/adr/…-resp-codec.md`).

**Pipeline (keep translator independent of the parser, blueprint):**
```text
RESP frame ──(redis-protocol decode)──► RedisCommand enum ──► ClientRequest / HC-op
                                                                     │
RESP response ◄──(encode)── RespValue ◄── translate(ClientResponse) ◄┘
```

**Steps.**
1. Scaffold the crate; add `redis-protocol` + `tokio` codec; define `RedisCommand` enum (parser-neutral).
2. Add `RedisApiConfig` (off by default, own port); `validate()` rejects RESP addr == client/admin/
   cluster addr (mirror `AdminAddressConflicts`, config.rs:247).
3. A HydraCache-owned listener accepting RESP connections, holding the same in-process handle the
   client surface uses (drives `ClientSurfaceState`, not the cache directly).

**Tests & requirements.** `crates/hydracache-redis-compat/tests/`
- `resp_frame_roundtrip_matches_redis_protocol` (decode/encode a known corpus).
- `redis_api_addr_conflicting_with_client_or_admin_is_rejected_loud` (config validation).
- `resp2_frames_are_accepted_and_resp3_only_frames_fail_loud_before_mutation`.
- Run: `cargo test -p hydracache-redis-compat --locked`, `cargo test -p hydracache-server --locked config`.

**Risk & rollback.** Additive, opt-in. Revert removes the crate + config field; the daemon is unchanged.

## W2. Command translator — the cache subset

**Goal.** Translate the honest MVP subset into `ClientRequest` and back, executing through
`ClientSurfaceState` so tenancy/limits/consistency hold.

**MVP subset (blueprint §"MVP Command Subset").**
- **Connection/compat (many clients issue these before user code):** `PING`, `ECHO`, `QUIT`, `HELLO`,
  `AUTH`, `CLIENT SETNAME`/`SETINFO`, `COMMAND` — minimal honest replies / no-ops where safe.
- **Values:** `GET`→`Get`, `SET`→`Put`, `MGET`→`BatchGet`, `MSET`→`BatchPut`, `DEL`→`Invalidate`,
  `EXISTS`→`Get`-probe. Binary bulk strings ↔ HydraCache value bytes (opaque).
- **TTL:** `SET EX/PX` plus `SETEX`/`PSETEX` aliases, `EXPIRE`/`PEXPIRE`→`Put` ttl; `TTL`/`PTTL`→read metadata; `PERSIST`.
- **Namespace / logical DB:** the listener uses one configured HydraCache namespace as Redis DB 0.
  `SELECT 0` is a supported no-op for Redis URL/client compatibility. Non-zero, negative, or
  malformed DB indexes return a stable loud error and never switch namespace or keyspace.

**Compatibility correction from W0.** The list above is the required support subset for the expanded
release, not an automatic support claim. W2 may close a command only after W0 proves the target
operation exists and the RESP reply matches Redis semantics:
- `SET EX/PX`, `EXPIRE`, `PEXPIRE`, `PERSIST`, `TTL`, and `PTTL` require protocol v3 expiry
  operations, real TTL application, and remaining-TTL metadata. The execution layer must stop dropping
  `ttl_ms`, store expiry metadata, remove expired keys before reads/counts, and return Redis TTL
  semantics with bounded oracle tolerance.
- `DEL` returns an integer count of keys actually removed, not just `OK`.
- `EXISTS` returns an integer count of currently present keys, including multi-key input if accepted.
- `MGET` preserves request order and emits nil bulk entries for misses.
- `MSET` is supported only when all entries are admitted before any mutation and the batch cannot
  partially succeed. Duplicate keys are applied in request order with last value winning. Do not map
  Redis `MSET` to a partial HydraCache batch result.
- `COMMAND` advertises only the commands this matrix marks supported; client bootstrap no-ops must
  never imply broader Redis compatibility.
- `SELECT 0` returns `OK` without dispatching to the client surface. `SELECT 1` and invalid DB
  indexes return stable RESP errors, and subsequent commands on the same connection still operate in
  DB 0.
- Startup no-ops (`CLIENT SETNAME`, `CLIENT SETINFO`, selected `HELLO` metadata) are accepted only
  when they are side-effect-free, bounded, and explicitly documented.
- Health/readiness probes (`INFO`, `ROLE`, `DBSIZE`, `TYPE`, `SCAN`, `CONFIG`, `CLIENT LIST`,
  `CLIENT ID`) follow the W0 classification. `INFO` returns only true RESP-facade fields; `TYPE`
  returns only `string`/`none` through the same `Get` path as cache reads; `ROLE`, `DBSIZE`, and
  `SCAN` remain unsupported-loud.

**Steps.**
1. `RedisCommand → ClientRequest` (+ the reverse `ClientResponse → RespValue`), through
   `ClientSurfaceState` (tenancy/limits enforced; a value over `max_value_bytes` returns a loud RESP
   error, not a truncation).
2. Auth: `AUTH`/`HELLO AUTH` maps to the client-surface identity (client-id/tenant); an unauthenticated
   command on an auth-required listener returns `NOAUTH`, invalid credentials return `WRONGPASS`, and
   successful authentication is connection-local and completed before any cache command reaches
   `ClientSurfaceState`.
3. Honest replies for startup no-ops so real clients connect.
4. Protocol v3 TTL operations: expiry metadata in request/response types, v3 negotiation/gating,
   v2 compatibility tests, and client-surface storage of value plus optional expiry timestamp.
5. Atomic batch apply for `MSET`: pre-validate batch size, per-value limits, tenant quota, and command
   arity before mutating the store; return a Redis error without partial writes on any rejection.

**Tests & requirements.** `crates/hydracache-redis-compat/tests/commands.rs`
- `get_set_del_mget_mset_roundtrip_through_client_surface`.
- `client_protocol_v3_registers_ttl_metadata_without_breaking_v2`.
- `set_ex_and_px_apply_expiry_through_client_surface`.
- `setex_psetex_expire_pexpire_persist_and_ttl_pttl_match_redis_semantics`.
- `expired_keys_are_absent_for_get_mget_exists_and_del`.
- `ttl_pttl_use_bounded_tolerance_against_real_redis`.
- `del_and_exists_return_redis_integer_counts`.
- `mget_preserves_order_and_represents_misses_as_nil_bulk`.
- `mset_is_atomic_and_duplicate_keys_use_last_value`.
- `mset_oversized_value_rejects_without_partial_mutation`.
- `command_reply_advertises_only_supported_subset`.
- `auth_hello_auth_and_noauth_errors_match_contract`.
- `redis_auth_required_listener_rejects_data_commands_before_auth`.
- `redis_auth_success_binds_connection_local_client_identity`.
- `info_role_dbsize_type_scan_and_config_follow_contract_classification`.
- `oversized_value_is_rejected_loud_not_truncated` (limits reuse, falsifiable).
- `unauthenticated_command_returns_noauth_when_auth_required`.
- `select_zero_is_supported_as_noop_for_single_database_contract`.
- `select_nonzero_and_invalid_db_fail_loud`.
- `resp_listener_select_zero_ok_and_nonzero_keeps_default_database`.
- Run: `cargo test -p hydracache-redis-compat --locked commands`.

**Risk & rollback.** The load-bearing property is *no silent semantic drift* — reuse the client-surface
execution rather than re-implementing. Revert removes the translator.

## W3. HydraCache extension commands (`HC.*`)

**Goal.** Expose HydraCache-native concepts RESP cannot express, as explicit opt-in commands — so
plain Redis clients get basic cache behavior and HydraCache-aware clients get tags/invalidation without
leaving RESP.

**Commands (blueprint §"HydraCache Extensions").** `HC.TAG key tag...` / `HC.SETTAGS`,
`HC.INVALIDATE key`, `HC.INVALIDATE_TAG tag`, `HC.NAMESPACE [name]`, `HC.STATS`, `HC.DIAGNOSTICS` —
mapped to existing diagnostics, per-key `Invalidate`, or the RESP listener's edge-local tag metadata
path. In 0.63 the tag path is deliberately **listener-local and in-memory**: `HC.TAG`/`HC.SETTAGS`
attach non-empty UTF-8 tags only to existing live keys, and `HC.INVALIDATE_TAG` invalidates live tagged
keys through `ClientSurfaceState` using `ClientRequest::Invalidate`. It is not a scan over the visible
keyspace, not Redis Cluster, and not a persisted/global HydraCache tag index.

**Tests & requirements.**
- `hc_tag_settags_and_invalidate_tag_use_edge_local_index_and_client_surface`
  (falsifiable: untagged key survives).
- `hc_tag_missing_key_does_not_create_metadata_or_mutate`.
- `hc_invalidate_tag_prunes_expired_keys_without_counting_them`.
- `hc_stats_and_diagnostics_are_read_only`.
- Run: `cargo test -p hydracache-redis-compat --locked`.

**Risk & rollback.** Additive commands; revert removes them (plain RESP subset still works).

### W3 Expansion: extension commands are native-or-unsupported

`HC.*` commands are allowed to be more HydraCache-specific than Redis, but they have a higher honesty
bar than the Redis subset because clients will treat the `HC.` prefix as a HydraCache contract. A
command under this namespace must either map to a real native operation with scoped auth and tests, or
return a stable unsupported error. It must not approximate internal behavior by scanning keys,
looping over per-key operations, or ignoring partial failures.

**Staging.**
1. **W3a read-only diagnostics, safest first.** `HC.STATS` and `HC.DIAGNOSTICS` may ship first because
   they are read-only and can use existing diagnostics/read-model surfaces. They must be tenant-scoped,
   redacted, bounded in size, and safe during drain. They must not expose other tenants, raw values,
   credentials, full keys unless already allowed by the existing diagnostics policy, or unbounded
   internal debug structures.
2. **W3b per-key native commands.** `HC.INVALIDATE key` may ship if it maps exactly to
   `ClientRequest::Invalidate` through `ClientSurfaceState` and returns a RESP result whose count or
   status is documented. It is not allowed to bypass tenancy, rate limits, residency checks, or audit.
3. **W3c namespace reporting and same-namespace confirmation.** `HC.NAMESPACE` returns the listener
   namespace, and `HC.NAMESPACE <same-name>` returns `OK`. Any other namespace fails loud. This is
   intentionally separate from Redis `SELECT`; Redis `SELECT` stays limited to `SELECT 0` as a no-op.
4. **W3d edge-local tag mutation.** `HC.TAG` and `HC.SETTAGS` mutate only the RESP listener's local
   in-memory tag index. They first prove the key is live through `ClientSurfaceState`, preserve value
   bytes and TTL by not rewriting the value, reject empty/non-UTF-8 tags, return `0` for missing keys,
   and clean local metadata after successful `DEL`/`HC.INVALIDATE`.
5. **W3e edge-local tag invalidation.** `HC.INVALIDATE_TAG tag` reads only keys previously tagged
   through this listener, verifies live values with `BatchGet`, invalidates live matches through
   `ClientRequest::Invalidate`, returns the live invalidation count, prunes stale expired/deleted
   entries, and leaves untagged keys untouched. It is not a scan-and-loop over visible Redis keys.

**Implemented value, complexity, and deferred global scope.** These rows record what 0.63 includes and
what remains outside the release claim. The implementation is useful for HydraCache-aware RESP clients
that route through one listener, while a persisted/core-wide tag index remains future work.

| Command group | What it gives clients in 0.63 | Implementation complexity now | Deferred / not claimed |
| --- | --- | --- | --- |
| `HC.NAMESPACE [name]` | HydraCache-aware clients can inspect the configured listener namespace and assert they are connected to the expected namespace without abusing Redis `SELECT`. | Low for reporting/same-namespace confirmation because it does not change connection state. | Switching to arbitrary namespaces, namespace allowlists, and auth remapping remain out of scope. It must not become Redis multi-db by another name. |
| `HC.TAG key tag...` | Adds one or more tags to an existing live key so a later extension command can invalidate a logical group. Duplicate tags are idempotent and return only newly added count. | Medium. Needs live-key lookup, tag normalization, local index maintenance, and cleanup after explicit key invalidation. | Tags are listener-local/in-memory, not persisted, not shared across listeners, and not visible to the core outside this RESP facade. |
| `HC.SETTAGS key tag...` | Replaces the complete local tag set for an existing live key while preserving the value and TTL. Duplicate input tags collapse to one stored tag. | Medium-high. Requires replace semantics, stale reverse-index cleanup, missing-key handling, and value/TTL preservation. | No atomic cross-listener metadata transaction and no protocol-level `dimensions` mutation claim. |
| `HC.INVALIDATE_TAG tag` | Invalidates live keys associated with a local tag without `SCAN pattern -> DEL`; untagged keys survive and stale expired/deleted entries are pruned. | High. Requires bounded local index lookup, live `BatchGet` verification, per-key invalidation through `ClientSurfaceState`, and partial-failure tests. | No Redis Cluster hash slots/topology, no global tag invalidation, and no persisted tag state after listener restart. |

**Future native path.** A later core-wide tag release can replace the edge-local path only with an
explicit compatibility entry. It must cover persisted/global tag metadata, tag index maintenance on
all write/delete/expiry paths, per-tenant namespace scoping, bounded tag count and tag byte limits,
atomic update semantics, redacted observability, and canary tests proving scan-and-loop behavior fails.

**Compatibility rule.** Extending `hydracache-client-protocol` for W3 is not allowed as an incidental
side effect of this edge release. If W3 requires a new public `ClientRequest` variant, the release must
be explicitly re-scoped as a client-protocol compatibility release, `docs/COMPAT.md` must register the
new protocol version/operation, and the "stable protocol untouched" claim in this plan must be
removed. The preferred `0.63.0` path is: use existing native surfaces, add edge-local read-model
helpers where they do not become public protocol, or keep the `HC.*` command unsupported-loud.

**Syntax and errors.**
- Commands are case-insensitive on input but documented uppercase.
- Arity errors return a stable RESP error distinct from unsupported-command errors.
- Keys remain binary bulk strings; namespace names and tags are UTF-8 strings unless the matrix
  explicitly allows binary-safe tag bytes.
- All `HC.*` errors include a stable machine-readable prefix such as `ERR hydracache unsupported`,
  `ERR hydracache auth`, `ERR hydracache limit`, or `ERR hydracache config`.
- Diagnostics errors must be redacted; values and credentials are never included in RESP errors.

**Additional tests & requirements.**
- `hc_stats_and_diagnostics_are_tenant_scoped_and_redacted`.
- `hc_diagnostics_are_read_only_during_drain`.
- `hc_namespace_is_listener_scoped_not_redis_multidb`.
- `hc_invalidate_key_goes_through_client_surface_limits_and_audit`.
- `hc_tag_settags_and_invalidate_tag_use_edge_local_index_and_client_surface`.
- `hc_tag_missing_key_does_not_create_metadata_or_mutate`.
- `hc_invalidate_tag_prunes_expired_keys_without_counting_them`.
- `client_matrix_runs_hydracache_tag_extension_scenario`.
- `redis_oracle_hc_extensions_are_hydracache_only`.

**W3 release decision.** `0.63.0` ships W3a/W3b plus the scoped W3c-W3e edge-local tag path. The
claim is intentionally narrower than a native global tag system: no scan-and-loop, no cross-listener
metadata, no Redis Cluster behavior, and no persisted tag index.

## W4. Unsupported-command matrix + guardrails

**Goal.** Everything outside the subset fails **loud and stable**, never wrong-but-green.

**Deferred Redis data-structure scope assessment.** The table below is part of the 0.63 contract,
not a backlog hint. These commands are useful in Redis ecosystems, but adding them would move the
release from a cache-subset RESP facade into Redis-compatible data-structure server territory. They
therefore stay unsupported in 0.63 unless a later release adds native HydraCache protocol/client-surface
operations, atomicity rules, limits, observability, and real Redis oracle coverage for the exact subset.

| Area | Complexity | Potential upside | Why not included in 0.63 |
| --- | --- | --- | --- |
| Hashes: `HSET`, `HGET`, `HDEL`, `HMGET`, `HEXISTS`, `HLEN` | Medium-high | Helps session fields, small object maps, metadata blobs, and frameworks that store partial object state in Redis hashes. This is the most plausible future data-structure subset if implemented as native single-key map operations. | Requires atomic read-modify-write or native map updates through the HydraCache client surface, TTL behavior for the whole structure, Redis count/nil semantics, size limits, and concurrency tests. A facade-only blob/JSON implementation would be wrong under concurrent field updates. |
| Sorted sets: `ZADD`, `ZRANGE`, `ZREM`, score/rank queries | High | Enables leaderboards, rate-limit buckets, delayed scheduling patterns, and priority/rank lookups used by Redis-heavy applications. | Needs ordered indexes by score/member, floating-point score compatibility, deterministic tie-breaking, range queries, atomic updates, memory accounting, and real Redis oracle tests. HydraCache currently exposes key/value cache operations, not per-key ordered index semantics. |
| Lists: `LPUSH`, `RPUSH`, `LPOP`, `RPOP`, blocking list ops | High | Covers queue-like workloads and simple producer/consumer patterns many Redis clients use. | List order, head/tail mutation, blocking operations, fairness, reconnect behavior, and consumer backpressure are outside the current cache-subset model. Shipping non-blocking-only or approximate queues would create wrong expectations for Redis clients. |
| Streams: `XADD`, `XREAD`, consumer groups, pending entries | Very high | Would support event-log and stream-processing migration stories, including consumer-group based fanout. | Streams are an append log with IDs, consumer groups, pending entries, ack/reclaim behavior, trimming, and replay semantics. That conflicts with the 0.63 non-goal that HydraCache is not an event log; implementing it honestly would be a separate product-sized release. |
| Lua / `EVAL` / script cache | Very high and security-sensitive | Some Redis applications rely on scripts for atomic multi-command workflows and server-side business logic. | Requires sandboxing, CPU/memory/time limits, deterministic execution, script cache semantics, command allowlists, auth isolation, and atomic execution across every command the script can call. It would bypass the simple command-by-command client-surface mapping that makes 0.63 safe. |
| Transactions: `MULTI`, `EXEC`, `DISCARD`, `WATCH` | High | Helps clients that group Redis operations and expect Redis transaction error/queue semantics. | Requires connection-local command queues, Redis error ordering, atomic commit semantics, `WATCH` conflict detection, and cross-command reducer behavior. Current 0.63 commands are executed directly through `ClientSurfaceState`, one command at a time. |
| Modules and Redis module commands | Product-sized / not planned | Would allow custom Redis commands/types from existing Redis module ecosystems. | Redis modules depend on a server-side ABI/API and custom data types. Supporting that would make HydraCache a Redis server clone, not a RESP cache-subset facade. This should remain a permanent non-goal unless the product direction changes explicitly. |
| Pub/Sub: `PUBLISH`, `SUBSCRIBE`, pattern subscriptions | Medium-high | Could improve migration for cache-notification style users and simple fanout listeners. | General Redis pub/sub is broader than HydraCache invalidation. It needs subscription lifecycle, pattern matching, delivery/backpressure rules, connection-mode behavior, and reconnect semantics. 0.63 keeps any pub/sub-like behavior limited to native HydraCache invalidation surfaces, not a general message bus. |

**Future path.** If this scope is ever expanded, hashes should be the first candidate because they can
be bounded to one key and mapped to native single-key map operations. That future release must add the
protocol operations first and then graduate a narrow hash subset from unsupported to supported. Sorted
sets, lists, streams, Lua, transactions, modules, and general pub/sub stay out of 0.63 because they are
either new data models, execution engines, or messaging systems rather than cache-subset RESP commands.

**Steps.**
1. A stable `ERR unsupported command '<CMD>'` for `HSET`/`ZADD`/lists/streams/`EVAL`/`MULTI`/`EXEC`/
   modules/`SUBSCRIBE` (unless invalidation-scoped)/`CLUSTER`/etc. A committed **matrix** doc lists
   supported vs unsupported.
2. No `MOVED`/`ASK`; `CLUSTER *` → unsupported (authority stays raft+epoch, R-1).
   `CLUSTER SLOTS`, `CLUSTER NODES`, `CLUSTER INFO`, `CLUSTER KEYSLOT`, and
   `CLUSTER GETKEYSINSLOT` are intentionally not implemented, because returning fake slots or a fake
   topology would make cluster-aware clients cache an invalid routing model.
3. `CONFIG`, `FLUSHDB`, and `FLUSHALL` → loud admin-disabled error before dispatch. `CONFIG` must not
   return fake Redis server configuration, and `FLUSHDB`/`FLUSHALL` must not remove keys in the
   default release configuration. They remain off by default even though the parser recognizes them,
   because destructive/admin operations need HydraCache-native authorization, scope, and audit.

**Tests & requirements.**
- `unsupported_commands_fail_loud_with_stable_error` (table-driven over the matrix).
- `cluster_commands_decode_as_unsupported_standalone_contract`.
- `cluster_and_moved_ask_are_never_emitted`.
- `cluster_mode_commands_fail_loud_over_resp_without_topology_or_redirects`.
- `admin_commands_are_disabled_by_default_without_config_or_flush_mutation`.
- `resp_listener_admin_commands_are_disabled_before_mutation`.
- Run: `cargo test -p hydracache-redis-compat --locked`.

**Risk & rollback.** Pure rejection surface; the matrix doc is the contract.

## W5. Golden RESP fixtures + mainstream-client smoke + decode fuzz

**Goal.** Prove wire-compat against real bytes and a real client, and that the decoder never panics.

**Steps.**
1. Committed **golden RESP request/response fixtures** (blueprint + the in-repo golden pattern,
   `0.62` `golden_vectors`): `.resp` byte corpus decoded to expected `RedisCommand`/`RespValue`.
2. **Mainstream-client smoke (Docker/gated, skip-graceful):** run the `redis`/`redis-rs` client (as a
   *dev-dependency*, client role) against a live `hydracache-redis-compat` listener for
   the W0-supported subset — proves an off-the-shelf client interoperates. The expanded smoke includes
   `MSET`, `SET EX` or `SET PX`, expiry observation through `TTL`/`PTTL`, post-expiry absence, and an
   auth-required connection path.
3. **Fuzz/property:** `proptest` over arbitrary bytes → decoder returns a value or a loud `Err`, never
   panics/`unwrap` (R-3); truncated/oversized/garbage frames.

**Tests & requirements.**
- `golden_resp_fixtures_decode_to_expected`.
- `mainstream_redis_client_can_talk_to_the_facade` (Docker-gated, skip-graceful).
- `resp_decoder_never_panics_on_arbitrary_bytes`.
- Run: `cargo test -p hydracache-redis-compat --locked` (+ Docker-gated smoke).

**Expanded client and wire proof.**
- **Conformance-driven scenarios:** W5 does not invent its own scenario list. It reads the versioned
  conformance manifest from W0 and executes the same scenario ids against HydraCache, real Redis
  oracle, and mainstream clients. Golden fixtures are keyed by manifest scenario id so a docs or
  translator change cannot leave stale fixtures behind.
- **Targeted tests are not the final compatibility claim.** Fast code tests prove parser, translator,
  listener, auth/TLS, TTL, `MSET`, `INFO`/`TYPE`, unsupported, and admin-disabled behavior in the
  changed Rust modules. They do not by themselves prove that mainstream clients and real Redis oracle
  behavior match across languages and Redis versions. The final `0.63.0` compatibility claim requires
  the Docker/nightly matrix below before release notes can say the Redis client ecosystem is covered.
  If the heavy matrix is not green, the release may still carry implemented code, but the release
  status must say "targeted tests passed; ecosystem/oracle proof pending" rather than claiming broad
  Redis compatibility.
- **Heavy proof tiers and cost.**

  | Gate | What it proves | Implementation or run complexity | Release meaning |
  | --- | --- | --- | --- |
  | Fast `hydracache-redis-compat` tests | RESP codec, command mapping, reducers, listener I/O, targeted auth/TLS/TTL/MSET/INFO/TYPE/admin-disabled behavior. | Low to run; PR-tier. | Required for every code change, but not enough for final external compatibility claims. |
  | Docker/client matrix | Python, Node, Go, JVM, and Rust clients can bootstrap and execute the supported subset through normal Redis client APIs, including auth, `rediss://`, RESP3 where supported, TTL, `MSET`, `SELECT 0`, `INFO`, and `TYPE`. | Medium-high to run because it depends on Docker, language images/toolchains, host networking, TLS material, and skip/require env flags. | Required before saying "mainstream Redis clients work" in the release ledger. Missing optional local runtimes may skip only when Docker fallback is unavailable and the row is not marked mandatory. |
  | Pinned real Redis oracle | Supported Redis-subset replies match pinned `redis-server` images after documented normalization; unsupported/admin-disabled/HC rows diverge only in the documented way. | Medium-high because it starts Docker Redis images, normalizes Redis version differences, and must avoid `latest`. | Required before saying HydraCache matches Redis behavior for the claimed subset. |
  | Resource/reconnect/multi-node gates | Slowloris/oversized input, connection-close/retry, resource bounds, metric redaction, and real multi-daemon routing do not break the RESP facade. | High and environment-sensitive; nightly/pre-release rather than PR-tier. | Required before production release confidence; failures narrow or block operational claims, not parser support. |
- **Fast client proof:** `redis-rs` remains the PR-tier smoke because it is already natural in the Rust
  workspace and can be a dev-dependency without adding a new language runtime to the fast gate.
- **Nightly client matrix:** Docker-gated jobs should run at least one client from each ecosystem that
  matters for the adoption story: Python (`redis-py`), Node (`node-redis` or `ioredis`), Go
  (`go-redis`), and JVM (`Lettuce` or `Jedis`). Each client must connect without custom protocol
  shims and run the same contract subset W0 marks supported. Local client installations are allowed
  for fast developer loops, but the release gate must also have a Docker fallback with pinned
  language/client images so Python/Node/JVM rows remain reproducible on clean machines while the Go
  row stays covered through the local Go toolchain.
- **Real Redis oracle:** the same mainstream-client scenario suite must run against a real
  Docker-managed `redis-server` and against the HydraCache RESP facade. Redis Docker tags are pinned
  (no `latest`) and the tested versions are recorded in `docs/GATES.md`; the default target is one
  Redis 6.x baseline and one Redis 7.x baseline. For W0-supported Redis subset commands, normalized
  replies must match real Redis. For unsupported Redis commands, divergence is expected and must match
  the docs matrix: Redis may succeed, while HydraCache returns the documented loud error. For `HC.*`
  commands, real Redis should return unknown-command behavior while HydraCache either executes the
  documented native command or returns the documented `ERR hydracache unsupported`. This oracle is a
  compatibility guard, not a requirement to become Redis.
- **RESP negotiation proof:** the live suite must test `HELLO 2`, `HELLO 3`, RESP2 requests after
  handshake, and representative RESP3-only inputs. `HELLO 3` behavior must match W0 exactly: downgrade,
  unsupported-loud, or candidate. A connection must not silently switch into an untested mixed dialect.
- **Startup handshake fixtures:** commit request/response fixtures for `PING`, `HELLO`, `AUTH`,
  `HELLO 2 AUTH`, `CLIENT SETNAME`, `CLIENT SETINFO`, `COMMAND`, and `QUIT`. These are not "nice to
  have"; they are what lets mainstream clients reach user commands. Auth fixtures include `NOAUTH`
  before auth, `WRONGPASS` for invalid credentials, `OK` for valid credentials, and no credential
  material in errors/log probes.
- **Pipelining:** add fixtures and live tests where a client sends multiple commands before reading
  any response. Responses must be emitted in request order, even when some commands fail.
- **Partial frames:** split a RESP frame across multiple TCP reads and coalesce multiple frames into
  one read. Both paths must decode identically to the golden corpus.
- **Backpressure and hostile input:** cap max bulk length, max array length, max inline command length,
  max in-flight requests per connection, and idle/slowloris timeouts. Rejections must happen before
  unbounded allocation and must increment bounded metrics.
- **Resource smoke:** a gated test opens many idle and pipelined connections, then asserts memory/fd
  growth plateaus and no metric label includes keys, values, request ids, or client-provided raw names.
- **Reconnect/failure smoke:** a gated test closes the client connection mid-command, mid-pipeline, and
  during daemon drain; reconnects with the same client library; and verifies no response corruption,
  cross-connection namespace leak, or ambiguous partial mutation. Idempotency-key behavior must be
  documented for retryable writes.
- **Multi-node HydraCache e2e:** a Docker/network-gated test runs the RESP listener against a real
  multi-daemon HydraCache grid. The scenario writes through RESP, reads through RESP, restarts or
  drains one grid node, and verifies the facade still goes through tenant/limit/consistency paths. This
  proves the facade is an edge to the shipped client surface, not an in-process-only cache shortcut.

**Additional tests & requirements.**
- `pipelined_requests_preserve_response_order`.
- `pipelined_mixed_success_and_error_responses_stay_ordered`.
- `partial_resp_frames_decode_like_complete_frames`.
- `multiple_resp_frames_in_one_read_are_all_processed`.
- `oversized_bulk_and_array_frames_are_rejected_before_allocation_spike`.
- `slowloris_connection_is_timed_out_without_leaking_inflight_work`.
- `nightly_python_node_go_jvm_clients_bootstrap_and_run_supported_subset`.
- `client_matrix_runs_auth_required_connection_scenario`.
- `redis_compat_conformance_manifest_drives_client_and_oracle_scenarios`.
- `redis_oracle_supported_subset_matches_real_redis`.
- `redis_oracle_uses_pinned_redis_versions`.
- `redis_oracle_del_exists_counts_match_real_redis`.
- `redis_oracle_mget_nil_and_order_match_real_redis`.
- `redis_oracle_mset_atomicity_matches_real_redis`.
- `redis_oracle_ttl_matches_real_redis_with_bounded_tolerance`.
- `redis_oracle_unsupported_divergence_is_documented`.
- `redis_oracle_hc_extensions_are_hydracache_only`.
- `hello2_and_hello3_are_supported_and_switch_dialect`.
- `resp3_only_inputs_are_rejected_before_mutation`.
- `auth_hello_auth_and_noauth_errors_match_contract`.
- `redis_auth_redacts_credentials_from_errors_logs_and_metrics`.
- `resp_surface_metrics_have_bounded_labels_and_no_key_or_value_leak`.
- `connection_close_mid_command_does_not_corrupt_next_response`.
- `connection_close_mid_pipeline_preserves_committed_response_boundaries`.
- `reconnect_and_retry_does_not_leak_connection_local_namespace`.
- `server_drain_during_pipeline_has_documented_completion_or_close_behavior`.
- `multinode_resp_facade_roundtrip_survives_node_restart_or_drain`.

**Risk & rollback.** Fixtures are a reviewed contract; smoke is gated. Additive.

## W6. Server wiring, docs, gates, ledger

**Goal.** Wire the listener into the daemon lifecycle, document the honest boundary, gate it, and fold
the reread doc into the tracked backlog.

**Steps.**
1. Start/stop the RESP listener with the daemon; drain through `graceful_shutdown` (`0.56`);
   native `rediss://` via the existing server TLS certificate/key material when Redis TLS is enabled;
   bounded-label metrics for the RESP surface (R-6).
2. Docs: `docs/integrations/redis-compat.md` (supported matrix, `HC.*`, config, positioning) with the
   headline **"Redis protocol compatible for the cache subset, not Redis feature compatible."**
3. COMPAT: register both the RESP surface (RESP2 subset v1) and the additive
   `hydracache-client-protocol` v3 TTL metadata/expiry extension in `docs/COMPAT.md` (R-4); any
   `HC.*` that extends `ClientRequest` is registered separately.
4. Backlog hygiene: **fold `CROSS_PROJECT_REREAD_IMPROVEMENT_PLAN.md` Redis-facade item into the
   tracked `CROSS_PROJECT_IDEA_BACKLOG.md`** (it is currently an untracked doc); mark this plan its
   home.
5. GATES.md rows: `hydracache-redis-compat` fast tests + the Docker-gated client smoke.

**Tests & requirements.**
- `daemon_serves_resp_listener_only_when_enabled_and_drains_gracefully` (server test).
- `cargo xtask verify` green; `doc-check` green (publishable-crate gate covers the new crate).

### W6 Expansion: release ledger, docs, gates, and evidence pack

W6 is the release-integrity work item. It must gather the implementation proof from W0-W5 into the
public documents and automated gates that future releases will preserve. A RESP facade is a long-lived
compatibility surface even if it is "only" an edge listener, so W6 must make the boundary explicit
enough that a later contributor cannot accidentally widen or weaken it.

**W6a daemon lifecycle and listener ownership.**
1. The RESP listener starts only when `RedisApiConfig.enabled = true`; the default daemon has no open
   RESP port and no extra background tasks.
2. The listener has a distinct address validation path covering client, admin, cluster, metrics, and
   any future role-scoped listeners. A conflict fails config validation before binding.
3. Shutdown uses the existing daemon drain/graceful-shutdown path. In-flight commands either finish
   inside the configured drain window or receive a stable connection-close/error behavior documented
   in `docs/integrations/redis-compat.md`.
4. TLS behavior is explicit: the RESP listener reuses the existing server TLS certificate/key material
   for `rediss://`, rejects plaintext on a TLS listener before mutation, and documents which CA/client
   settings are required. The release cannot leave TLS ambiguous.
5. Auth-required listeners reject all mutating/data commands before successful `AUTH` or `HELLO AUTH`,
   while allowing only the pre-auth handshake commands W0 marks safe.

**W6b public documentation deliverables.**
1. `docs/integrations/redis-compat.md` is the canonical user-facing page. It includes the headline:
   **"Redis protocol compatible for the cache subset, not Redis feature compatible."**
2. The doc contains the W0 command matrix with columns for status, Redis expectation, HydraCache
   behavior, exact response shape, caveats, auth scope, and test name.
3. The doc has copy-pasteable examples for at least `redis-cli`, `redis-rs`, Python, Node, Go, and one
   JVM client, but examples must not claim commands outside the supported matrix. Every example is
   executable through a docs-smoke gate; examples that require Docker, a language runtime, TLS, auth,
   or `HC.*` extensions carry the exact gate label that proves them.
4. The doc explains namespace behavior: one configured HydraCache namespace exposed as Redis DB 0,
   `SELECT 0` as a no-op, non-zero/invalid DB indexes as loud errors, and `HC.NAMESPACE` only if a
   future candidate gate ships it.
5. The doc explains TTL honestly as supported through protocol v3 metadata: `SET EX/PX` applies
   expiry, `EXPIRE`/`PEXPIRE` mutate expiry only when the key exists, `PERSIST` clears expiry, and
   `TTL`/`PTTL` return Redis `-2`/`-1`/positive remaining-time semantics with bounded oracle
   tolerance.
6. The doc explains `HC.*` status command by command, especially whether tag commands are supported,
   candidate, or unsupported-loud.
7. The doc has a migration-warning section for Redis features HydraCache intentionally does not
   provide: hashes, sorted sets, lists, streams, Lua, transactions, modules, Redis Cluster, async
   replication, and general Pub/Sub.
8. The doc includes the oracle-normalization rules from W0: exact matches, normalized errors,
   bounded TTL tolerances, and documented divergence for unsupported commands.
9. The doc includes the supported `redis-server` oracle versions used by the release gate and the
   policy for updating those pinned versions.

**W6c compatibility register.**
1. `docs/COMPAT.md` gets a new artifact row for the RESP edge surface, for example
   `HydraCache Redis RESP edge surface | RESP2 subset v1`.
2. The row records that RESP2 and RESP3 are the supported wire dialects for `0.63.0`.
3. The register updates the existing `hydracache-client-protocol` artifact from version `2` to
   version `3` with additive TTL metadata/expiry request and response shapes. W6 also records that v2
   clients remain accepted and do not receive v3-only responses unless they negotiate protocol v3.
4. The row names the failure mode: unsupported commands, unsupported RESP3 aggregate command frames, oversized frames,
   unauthenticated commands, wrong tenant scope, and malformed/truncated frames fail loud before
   mutation.
5. If `HC.*` commands ship, the row names their compatibility version and says whether they are edge
   commands only or backed by public protocol operations.

**W6d GATES and TESTING.**
1. `docs/GATES.md` gets a fast gate row for `cargo test -p hydracache-redis-compat --locked` and the
   targeted server config/lifecycle tests.
2. `docs/GATES.md` gets a semantic contract gate: every supported command row in
   `redis-compat.md` has a named test and every translator-supported command appears in the matrix.
3. `docs/GATES.md` gets a Docker/nightly gate for the multi-language client matrix and resource smoke.
4. `docs/GATES.md` names the env vars for gated tests, for example
   `HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1` and `HYDRACACHE_RUN_REDIS_COMPAT_RESOURCE_SMOKE=1`.
5. `docs/TESTING.md` gets a "Redis RESP compatibility" section explaining how to add a command:
   update W0 matrix, add golden fixture, add translator test, add unsupported-matrix row if rejected,
   update client smoke only after the command is supportable.
6. `docs/TESTING.md` explains how to update the conformance manifest, regenerate fixtures, run the
   real Redis oracle locally, pin/update Redis Docker image versions, and decide whether a divergence
   is a bug or an intentional unsupported row.
7. CI has named steps for the fast crate tests and config tests. Nightly/client-matrix steps are
   explicitly named even if env-gated or scheduled, so the release gate is not docs-only.
8. Docs-smoke gates execute the examples in `docs/integrations/redis-compat.md` for each claimed
   client ecosystem. If a runtime is unavailable in fast CI, the example is Docker/nightly-gated and
   labeled as such in the doc.

**W6e observability and security.**
1. RESP metrics use bounded labels only: command family/status/protocol version/auth state, never key,
   value, raw client name, request id, tenant-provided tag, or arbitrary error text.
2. Errors and logs redact credentials from `AUTH`, `HELLO AUTH`, connection strings, and client
   library debug metadata.
3. Audit events exist for auth failures, admin-disabled commands, `HC.*` mutating commands, and
   dangerous command attempts (`FLUSHDB`, `FLUSHALL`, `CONFIG`, `MODULE`, `EVAL`).
4. Diagnostics commands are tenant-scoped and read-only. Cross-tenant data leakage is a release
   blocker.
5. The listener has a documented default timeout and max-frame policy. The policy is referenced from
   both config docs and `GATES.md` hostile-input tests.

**W6f backlog and release manifest hygiene.**
1. `CROSS_PROJECT_REREAD_IMPROVEMENT_PLAN.md` Redis-facade item is folded into
   `CROSS_PROJECT_IDEA_BACKLOG.md` with this plan as the tracked home.
2. `docs/plans/INDEX.md`, `docs/plans/releases.toml`, this plan header, and the eventual release note
   `docs/releases/0.63.0.md` are flipped together. No manifest points at a claim that docs/gates do
   not prove.
3. The release note describes `HC.INVALIDATE_TAG` as an edge-local RESP listener extension and lists
   global/persisted/core-wide tag metadata under "Not shipped in 0.63.0". TTL must not be listed there
   because it is mandatory scope for the expanded release.
4. The release note includes the exact supported command list and the exact unsupported classes.
5. Any follow-up work discovered by W0 becomes either a technical-debt entry or a future plan row with
   owner, gate, and reason, not an orphan TODO inside code comments.

**W6g config/operator packaging and rollout playbook.**
1. Server config docs and sample config files show the RESP listener disabled by default. Enabling the
   listener requires an explicit `redis_api.enabled = true`-style setting and an explicit listen
   address.
2. Production examples do not expose port `6379` by default. If Helm/operator packaging exists, the
   plan updates values, service templates, NetworkPolicy guidance, TLS/auth secret wiring, and upgrade
   notes so the facade is opt-in at the platform boundary too.
3. Operator documentation names safe defaults: bind to localhost for local dev, require auth/TLS or
   private network controls for production, and avoid exposing the RESP port on public load balancers.
4. The rollout playbook describes canary enablement: enable on one edge/daemon, run oracle/client
   smoke, watch metrics and audit events, then expand.
5. The rollback playbook names concrete triggers: auth failures spike, unsupported command rate exceeds
   the expected migration baseline, memory/fd growth does not plateau, p99 command latency violates
   the stated edge SLO, response-order tests fail in canary, or audit detects cross-tenant access.
6. The rollback procedure says whether disabling the listener requires restart, how existing
   connections are drained or closed, what metrics confirm shutdown, and how to preserve logs/fixtures
   for debugging.
7. Config and operator docs cross-link the W6 compatibility register and W5 resource/backpressure
   limits so operators know what is protected by tests and what is intentionally not Redis-compatible.

**Additional W6 tests & requirements.**
- `redis_compat_docs_matrix_has_test_for_every_supported_command`.
- `redis_compat_translator_has_no_command_missing_from_docs_matrix`.
- `compat_register_mentions_resp2_subset_and_failure_modes`.
- `gates_include_fast_contract_and_docker_client_matrix_rows`.
- `testing_docs_explain_how_to_add_a_resp_command`.
- `redis_compat_conformance_manifest_is_referenced_by_docs_tests_and_oracle`.
- `redis_compat_docs_examples_are_executable_or_gated_with_labels`.
- `redis_oracle_versions_are_pinned_and_documented`.
- `oracle_normalization_rules_are_documented_and_checked`.
- `resp_metrics_do_not_include_unbounded_labels`.
- `auth_and_connection_logs_redact_credentials`.
- `redis_api_tls_mode_is_explicitly_documented`.
- `redis_api_config_examples_keep_listener_disabled_by_default`.
- `redis_api_operator_packaging_does_not_expose_port_by_default`.
- `redis_api_rollout_and_rollback_playbook_names_metrics_and_triggers`.
- `release_note_lists_supported_and_not_shipped_commands`.

**W6 release decision.** `0.63.0` cannot be marked shipped because the listener works locally. It ships
only when the compatibility surface is registered, the docs matrix matches the translator, fast and
gated commands are named in `GATES.md`, and the release note tells users exactly which Redis behaviors
HydraCache does and does not implement.

## Test coverage matrix (every new artifact has a named test)

| New code | Source | Covering test(s) | Tier |
| --- | --- | --- | --- |
| semantic command contract (W0) | `docs/integrations/redis-compat.md` + contract fixtures | `redis_command_contract_has_no_supported_row_without_test`, `command_reply_advertises_only_supported_subset`, `oracle_normalization_rules_are_declared_for_every_supported_command` | PR |
| conformance manifest (W0/W5/W6) | `docs/integrations/redis_compat_conformance.json` or `.yaml` | `redis_compat_conformance_manifest_is_the_single_source_of_truth`, `redis_compat_conformance_manifest_drives_client_and_oracle_scenarios`, `redis_compat_conformance_manifest_is_referenced_by_docs_tests_and_oracle` | PR |
| client protocol v3 TTL metadata (W0/W2/W6) | `hydracache-client-protocol` + `docs/COMPAT.md` | `client_protocol_v3_registers_ttl_metadata_without_breaking_v2`, `protocol_v2_clients_do_not_receive_v3_ttl_shapes`, `compat_register_mentions_client_protocol_v3_ttl_extension` | PR |
| client-surface expiry semantics (W2) | `hydracache-client-transport-axum` | `set_ex_and_px_apply_expiry_through_client_surface`, `expire_pexpire_persist_and_ttl_pttl_match_redis_semantics`, `expired_keys_are_absent_for_get_mget_exists_and_del` | PR |
| RESP2/RESP3 negotiation (W0/W1/W2/W5) | `hydracache-redis-compat` + conformance manifest | `hello2_and_hello3_are_supported_and_switch_dialect`, `resp3_commands_roundtrip_supported_cache_subset`, `resp3_unsupported_aggregate_inputs_fail_before_mutation`, `client_matrix_runs_resp3_negotiation_scenario` | PR |
| `RedisCommand` + RESP codec (W1) | `hydracache-redis-compat` | `resp_frame_roundtrip_matches_redis_protocol` | PR |
| `RedisApiConfig` + validation (W1) | `hydracache-server/src/config.rs` | `redis_api_addr_conflicting_with_client_or_admin_is_rejected_loud` | PR |
| Redis auth and native TLS (W0/W1/W2/W5/W6) | `hydracache-redis-compat` + `hydracache-server` config/docs + raw TLS listener | `auth_hello_auth_and_noauth_errors_match_contract`, `redis_auth_required_listener_rejects_data_commands_before_auth`, `redis_auth_success_binds_connection_local_client_identity`, `redis_auth_redacts_credentials_from_errors_logs_and_metrics`, `redis_api_rediss_env_reuses_server_tls_material`, `redis_resp_listener_accepts_rediss_auth_and_cache_commands`, `redis_resp_tls_listener_rejects_plaintext_before_mutation`, `redis_resp_tls_client_rejects_wrong_ca`, `redis_resp_tls_keeps_wrong_auth_as_wrongpass` | PR |
| subset translator (W2) | `hydracache-redis-compat` | `get_set_del_mget_mset_roundtrip_through_client_surface`, `set_ex_and_ttl_map_to_protocol_v3_metadata`, `del_and_exists_return_redis_integer_counts`, `mget_preserves_order_and_represents_misses_as_nil_bulk`, `mset_is_atomic_and_duplicate_keys_use_last_value`, `mset_oversized_value_rejects_without_partial_mutation`, `oversized_value_is_rejected_loud_not_truncated`, `unauthenticated_command_returns_noauth_when_auth_required`, `select_zero_is_supported_as_noop_for_single_database_contract`, `select_nonzero_and_invalid_db_fail_loud`, `resp_listener_select_zero_ok_and_nonzero_keeps_default_database` | PR |
| health/readiness command classification (W0/W2) | conformance manifest + translator/unsupported matrix | `health_check_commands_are_classified_before_translation`, `info_role_dbsize_type_scan_and_config_follow_contract_classification`, `info_returns_minimal_honest_facade_state`, `info_section_argument_does_not_fabricate_redis_keyspace_state`, `resp_listener_info_probe_does_not_fabricate_keyspace_or_cluster_state`, `type_reports_string_or_none_through_client_surface`, `resp_listener_type_reports_string_and_none`, `mainstream_redis_client_can_talk_to_the_facade`, `nightly_python_node_go_jvm_clients_bootstrap_and_run_supported_subset`, `redis_oracle_supported_subset_matches_real_redis` for `TYPE` | PR + Docker-gated / nightly |
| `HC.*` read-only/per-key extensions (W3a/W3b) | `hydracache-redis-compat` | `hc_stats_and_diagnostics_are_tenant_scoped_and_redacted`, `hc_diagnostics_are_read_only_during_drain`, `hc_invalidate_key_goes_through_client_surface_limits_and_audit` | PR |
| `HC.*` tag/dimension commands (W3c/W3d/W3e) | RESP-listener-local tag index + `ClientSurfaceState` invalidation path | `hc_namespace_is_listener_scoped_not_redis_multidb`, `hc_tag_settags_and_invalidate_tag_use_edge_local_index_and_client_surface`, `hc_tag_missing_key_does_not_create_metadata_or_mutate`, `hc_invalidate_tag_prunes_expired_keys_without_counting_them`, `client_matrix_runs_hydracache_tag_extension_scenario`, `redis_oracle_hc_extensions_are_hydracache_only` | PR + Docker-gated / nightly |
| unsupported/admin-disabled matrix (W4) | `hydracache-redis-compat` | `unsupported_commands_fail_loud_with_stable_error`, `cluster_commands_decode_as_unsupported_standalone_contract`, `cluster_and_moved_ask_are_never_emitted`, `cluster_mode_commands_fail_loud_over_resp_without_topology_or_redirects`, `admin_commands_are_disabled_by_default_without_config_or_flush_mutation`, `resp_listener_admin_commands_are_disabled_before_mutation` | PR |
| golden + fuzz + frame boundaries (W5) | committed corpus + proptest | `golden_resp_fixtures_decode_to_expected`, `resp_decoder_never_panics_on_arbitrary_bytes`, `partial_resp_frames_decode_like_complete_frames`, `multiple_resp_frames_in_one_read_are_all_processed` | PR |
| pipelining/backpressure/resource behavior (W5) | RESP listener | `pipelined_requests_preserve_response_order`, `pipelined_mixed_success_and_error_responses_stay_ordered`, `oversized_bulk_and_array_frames_are_rejected_before_allocation_spike`, `slowloris_connection_is_timed_out_without_leaking_inflight_work`, `resp_surface_metrics_have_bounded_labels_and_no_key_or_value_leak` | PR + gated |
| mainstream client smoke (W5) | dev-dep `redis` client + Docker language clients | `mainstream_redis_client_can_talk_to_the_facade`, `nightly_python_node_go_jvm_clients_bootstrap_and_run_supported_subset`, `client_matrix_runs_mset_and_ttl_commands`, `client_matrix_runs_resp3_negotiation_scenario`, `client_matrix_runs_auth_required_connection_scenario`, `client_matrix_runs_rediss_required_connection_scenario` | PR + Docker-gated / nightly |
| real Redis oracle (W5) | pinned Docker `redis-server` versions + same client scenario suite against Redis and HydraCache | `redis_oracle_supported_subset_matches_real_redis`, `redis_oracle_uses_pinned_redis_versions`, `redis_oracle_del_exists_counts_match_real_redis`, `redis_oracle_mget_nil_and_order_match_real_redis`, `redis_oracle_mset_atomicity_matches_real_redis`, `redis_oracle_ttl_matches_real_redis_with_bounded_tolerance`, `redis_oracle_unsupported_divergence_is_documented`, `redis_oracle_hc_extensions_are_hydracache_only` | Docker-gated / nightly |
| reconnect/failure semantics (W5) | RESP listener live tests | `connection_close_mid_command_does_not_corrupt_next_response`, `connection_close_mid_pipeline_preserves_committed_response_boundaries`, `reconnect_and_retry_does_not_leak_connection_local_namespace`, `server_drain_during_pipeline_has_documented_completion_or_close_behavior` | Docker-gated / nightly |
| multi-node RESP e2e (W5) | real multi-daemon HydraCache grid + RESP listener | `multinode_resp_facade_roundtrip_survives_node_restart_or_drain` | network-gated / nightly |
| daemon RESP lifecycle (W6) | `hydracache-server` | `daemon_serves_resp_listener_only_when_enabled_and_drains_gracefully` | PR |
| executable docs (W6) | `docs/integrations/redis-compat.md` examples | `redis_compat_docs_examples_are_executable_or_gated_with_labels` | PR + Docker-gated |
| release ledger/docs/gates (W6) | `COMPAT.md` / `GATES.md` / `TESTING.md` / release notes | `redis_compat_docs_matrix_has_test_for_every_supported_command`, `redis_compat_translator_has_no_command_missing_from_docs_matrix`, `compat_register_mentions_resp2_subset_and_failure_modes`, `gates_include_fast_contract_and_docker_client_matrix_rows`, `testing_docs_explain_how_to_add_a_resp_command`, `redis_oracle_versions_are_pinned_and_documented`, `oracle_normalization_rules_are_documented_and_checked`, `release_note_lists_supported_and_not_shipped_commands` | PR |
| config/operator packaging and rollback docs (W6) | config examples / operator docs / production guide | `redis_api_config_examples_keep_listener_disabled_by_default`, `redis_api_operator_packaging_does_not_expose_port_by_default`, `redis_api_rollout_and_rollback_playbook_names_metrics_and_triggers` | PR |
| security/observability docs and behavior (W6) | listener logs/metrics/docs | `resp_metrics_do_not_include_unbounded_labels`, `auth_and_connection_logs_redact_credentials`, `redis_api_tls_mode_is_explicitly_documented` | PR |

**Coverage rule (DoD):** no new command/type lands without a row; PR-tier deterministic and in
`cargo xtask verify`; Docker-gated client smoke is env-gated skip-graceful.

## Gates (Definition of Done for the release)

- `cargo xtask verify` green; RESP client smoke Docker-gated + skip-graceful; W0 command-contract
  tests prove every supported command row has a named test.
- The versioned Redis compatibility conformance manifest is the single source of truth for docs,
  translator tests, golden fixtures, real Redis oracle scenarios, client smoke, and release notes.
- A **mainstream Redis client** performs the W0-supported subset against the facade unchanged. The
  minimum required subset is GET/SET/MGET/MSET/DEL plus startup handshake and the supported TTL
  commands. The release cannot close until real TTL application, remaining-TTL metadata,
  post-expiry absence, and auth-required startup are proven through the client surface.
- Docker/nightly client matrix proves the supported subset through Python, Node, Go, and JVM Redis
  clients, including `MSET`, `SET EX`/`SET PX`, `TTL`/`PTTL`, and post-expiry reads, or the release
  note explicitly narrows the client-support claim to the clients that passed.
- The same client scenario suite runs against Docker `redis-server` and HydraCache. W0-supported
  Redis-subset replies match real Redis after documented normalization; unsupported Redis-command
  divergence is documented; `HC.*` commands are documented as HydraCache-only and return unknown
  command on real Redis. Redis oracle images are pinned and documented; no `latest` oracle image is
  allowed in gates.
- RESP2/RESP3 negotiation is explicit and tested: `HELLO 2` works as documented, `HELLO 3` switches
  the connection to RESP3 replies for the supported cache subset, and unsupported RESP3 aggregate
  command frames fail before mutation with no silent mixed dialect mode.
- Redis `AUTH` and `HELLO 2 AUTH` are explicit and tested: auth-required listeners reject data and
  mutating commands with `NOAUTH` before authentication, invalid credentials return `WRONGPASS`,
  successful auth binds connection-local identity/tenant before dispatch, and credentials never appear
  in errors, logs, metrics, or diagnostics.
- Every non-subset command fails with a **stable loud error**; no `MOVED`/`ASK`/`CLUSTER`;
  `CONFIG`, `FLUSHDB`, and `FLUSHALL` are admin-disabled by default and proven not to dispatch or
  mutate keys (W4). RESP decoder never panics on arbitrary bytes (W5, R-3).
- Health/readiness probes (`INFO`, `ROLE`, `DBSIZE`, `TYPE`, `SCAN`, `CONFIG`, `CLIENT LIST`,
  `CLIENT ID`) are classified in the manifest. `INFO` returns minimal honest RESP-facade state,
  `TYPE` returns `string`/`none` for the cache subset, and `ROLE`/`DBSIZE`/`SCAN` fail unsupported.
  No fabricated Redis server state is exposed.
- Pipelined commands preserve response order; partial frames and coalesced frames decode like the
  golden corpus; oversized/hostile RESP frames are rejected before unbounded allocation; slowloris
  connections time out without leaking in-flight work (W5).
- Connection-close and reconnect behavior is tested: close mid-command, close mid-pipeline, drain
  during pipeline, and reconnect-and-retry cannot corrupt response boundaries, leak connection-local
  namespace state, or hide ambiguous partial writes.
- A network-gated multi-node HydraCache RESP e2e writes and reads through the facade across a real
  daemon/grid restart or drain, proving the edge listener does not bypass tenancy, limits, or
  consistency.
- Tenancy/limits/consistency are enforced because the facade drives `ClientSurfaceState`, not the cache
  directly; an oversized value is rejected loud, not truncated (W2).
- `DEL`/`EXISTS` return Redis-style integer counts; `MGET` preserves order and nil misses; `MSET` is
  atomic with duplicate-key last-write-wins semantics; `SET EX/PX`, `EXPIRE`/`PEXPIRE`, `PERSIST`,
  `TTL`, and `PTTL` match Redis semantics with bounded TTL tolerance; `COMMAND` advertises only
  supported commands (W0/W2).
- Tags/invalidation are only via explicit `HC.*` — no raw prefix-invalidation over binary keys, no
  scan-and-loop fake tag invalidation, no cross-tenant tag mutation (W3).
- `HC.NAMESPACE`/`HC.TAG`/`HC.SETTAGS`/`HC.INVALIDATE_TAG` ship only as the documented
  RESP-listener-local extension path; global/persisted/core-wide tag invalidation remains out of
  scope unless a future native tag release adds its own compatibility entry and tests (W3).
- Listener is **off by default**, on its own port, distinct-address-validated; embedded/core fast path
  byte-for-byte unchanged (R-10); `hydracache-client-protocol` v3 is registered as an additive TTL
  metadata/expiry extension and v2 clients remain accepted (R-4).
- TLS/rediss behavior is explicit and implemented: `rediss://` uses the existing server TLS
  certificate/key material, plaintext is rejected on TLS listeners before mutation, wrong CA fails the
  client handshake, wrong Redis credentials over TLS return `WRONGPASS`, and auth credentials or
  certificate material are redacted from logs and errors (W6).
- Metrics/logs/audit use bounded labels and never include key bytes, value bytes, raw client names,
  request ids, credentials, or unbounded tenant-provided strings (W5/W6).
- User-facing examples in `docs/integrations/redis-compat.md` are executable docs-smoke tests or are
  explicitly labeled with their Docker/nightly gate. The examples include supported `MSET`, TTL
  commands, `SELECT 0`, `rediss://` startup, and RESP3 negotiation. Docs cannot show untested `HC.*`
  commands or non-zero `SELECT` examples.
- Config/operator packaging keeps the RESP listener disabled and unexposed by default. Helm/operator
  or production examples require explicit enablement, explicit port exposure, Redis `AUTH`, and TLS
  material/CA configuration.
- The rollout/rollback playbook is complete: canary path, metrics and audit events to watch, rollback
  triggers, disable procedure, connection drain/close behavior, and evidence capture are all named.
- No new consensus/consistency level (R-1); positioning states "cache subset, not feature compatible";
  the reread Redis-facade item is folded into the tracked backlog.
- `docs/integrations/redis-compat.md`, `docs/COMPAT.md`, `docs/GATES.md`, `docs/TESTING.md`,
  `releases.toml`, `INDEX.md`, plan header, and `docs/releases/0.63.0.md` are reconciled together;
  `doc-check` green.

## Semantic Accuracy Audit Pass (post-implementation review, 2026-07-11)

A post-implementation code review of `hydracache-redis-compat` against real Redis semantics confirmed
the translator is broadly accurate: `EXISTS k k`/`DEL k k` duplicate counting, `MGET` order/nil, `TTL`
`-2`/`-1`/positive, `SET EX/PX`, `SETEX`/`PSETEX`, and `PERSIST` are correct and already covered in the
fast tier (`del_and_exists_return_redis_integer_counts`,
`setex_psetex_expire_pexpire_persist_and_ttl_pttl_match_redis_semantics`,
`mget_preserves_order_and_represents_misses_as_nil_bulk`). No correctness bug was found in the
translator. The review found **one impactful compatibility gap** and **two unverified return edges**
that must close — or be explicitly narrowed in the conformance manifest — before `0.63.0` ships. These
items are additive to W0–W6: they do not widen the supported surface, they make the already-claimed
surface honest. Each is one closed task with its own commit; run the targeted crate tests before each
commit.

### A1. `SET` write-conditional and retention options (NX/XX/GET/KEEPTTL/EXAT/PXAT)
**Finding.** `parse_set_ttl_ms` accepts only `[]` or `[EX|PX, value]`; every other option shape is
rejected as a translation error. `SET key value NX`, `SET key value XX`, `SET key value KEEPTTL`,
`SET key value GET`, and the canonical lock idiom `SET key value NX PX ttl` therefore fail. This is
loud, not silent, but it breaks the Redis distributed-lock primitive that mainstream lock libraries
(redis-py `Lock`, node `redlock`, Redisson locks) depend on — the highest-value real-world Redis idiom
after plain GET/SET.
**0.63 decision.** Choose path (a) for this release: declare these forms explicitly
`unsupported-loud`. `SET NX`/`XX` is not a parser nicety; it is the Redis single-key conditional write
primitive used by lock libraries. Shipping it safely would require an atomic client-surface
conditional write path and a real client/oracle matrix, not a read-then-write shim. Therefore 0.63 keeps
the supported write surface to bare `SET` plus `SET EX/PX`/`SETEX`/`PSETEX`, and every conditional,
return-old-value, retention, or absolute-expiry option (`NX`, `XX`, `GET`, `KEEPTTL`, `EXAT`, `PXAT`)
returns a Redis-shaped `ERR syntax error` before dispatch. This is intentional documented divergence:
no silent success, no dropped option, no lock primitive that appears to work while racing.
**DoD tests.** `set_write_conditional_options_follow_conformance_contract`,
`set_nx_px_lock_idiom_has_declared_behavior_and_redis_shaped_error`, plus the env-gated
`client_matrix_set_nx_px_lock_idiom_fails_loud_without_hanging` row.

### A2. `EXPIRE`/`PEXPIRE` with non-positive TTL
**Finding.** `parse_expire_ttl_ms` collapses `value <= 0` to `ttl_ms = 0`. The store's
`expires_at = now` with `is_expired: expires_at <= now` does make the key vanish immediately (correct
side effect), but **no fast-tier test asserts the Redis return contract**: `EXPIRE existing 0`/negative
must delete the key and return `1`, and the key must be absent for subsequent
`GET`/`MGET`/`EXISTS`/`TTL`. This is a silent zone in the executable compatibility contract the release
promises.
**DoD tests.** `expire_zero_or_negative_deletes_key_and_returns_one`,
`expired_by_nonpositive_expire_is_absent_for_get_mget_exists_ttl`.

### A3. `EXPIRE`/`PEXPIRE`/`PERSIST` on a missing key
**Finding.** `reduce_expiry` returns the client-surface `applied` flag, but no fast-tier test asserts
the missing-key contract: `EXPIRE missing …` → `0`, `PERSIST missing` → `0`.
**DoD test.** `expire_pexpire_and_persist_on_missing_key_return_zero`.

### A4. Redis-shaped error text for rejected option/TTL shapes (cosmetic, class-normalized)
**Finding.** Rejected SET-option and oversized-TTL shapes surface custom messages
("SET supports only EX seconds or PX milliseconds options in this release", "TTL value is too large")
instead of native Redis text (`ERR syntax error`, `ERR invalid expire time in 'set' command`). The
oracle normalizes by error class (`ERR`), so gates pass, but the human-facing text differs from real
Redis and can confuse operators reading logs side by side.
**0.63 decision.** Align the high-value human-facing `SET` rejection text where cheap: unsupported SET
option shapes return `ERR syntax error`, and invalid `SET EX/PX` TTL returns Redis-shaped invalid
expire-time text. Keep the broader oracle rule as class-normalized for other rejected TTL/resource
edges whose exact Redis wording is not part of the supported subset contract.
**DoD test.** `rejected_set_and_expire_shapes_use_redis_error_class_or_documented_normalization`.

### Coverage matrix additions

| New code | Source | Covering test(s) | Tier |
| --- | --- | --- | --- |
| `SET` write-conditional/retention contract (A1) | `hydracache-redis-compat` + conformance manifest + client matrix | `set_write_conditional_options_follow_conformance_contract`, `set_nx_px_lock_idiom_has_declared_behavior_and_redis_shaped_error`, `client_matrix_set_nx_px_lock_idiom_fails_loud_without_hanging` | PR + Docker-gated |
| non-positive `EXPIRE` return/side-effect (A2) | `hydracache-redis-compat` | `expire_zero_or_negative_deletes_key_and_returns_one`, `expired_by_nonpositive_expire_is_absent_for_get_mget_exists_ttl` | PR |
| missing-key expiry return (A3) | `hydracache-redis-compat` | `expire_pexpire_and_persist_on_missing_key_return_zero` | PR |
| rejected-shape error normalization (A4) | `hydracache-redis-compat` + oracle-normalization notes | `rejected_set_and_expire_shapes_use_redis_error_class_or_documented_normalization` | PR |

### Gate additions
- `SET` write-conditional/retention options (NX/XX/GET/KEEPTTL/EXAT/PXAT) have a single declared
  behavior in the conformance manifest — supported-atomic **or** unsupported-loud with a Redis-shaped
  error — proven by a client-matrix lock-idiom row. No silent success, no silent drop, no hang.
- `EXPIRE`/`PEXPIRE` with `0`/negative TTL delete the key and return `1` for existing keys, and the key
  is absent afterward. `EXPIRE`/`PEXPIRE`/`PERSIST` on a missing key return `0`. All asserted in the
  **fast tier**, not only the gated oracle.
- Rejected option/TTL shapes either match Redis error text or are explicitly documented as
  class-normalized in the oracle-normalization notes.

## Final Release Decision

`0.63.0` ships **only** if every gate is green. The honesty bar for a *compatibility* release is
inverted: a facade that *looks* Redis-compatible but silently drifts (a wrong TTL, a wrong integer
count, a partially applied `MSET`, a faked tag-invalidation, a swallowed unsupported command, an
unbounded RESP allocation) is worse than no facade. W0 decides what is supportable through the
versioned conformance manifest; W2/W3 implement only those commands; W4 rejects the rest loudly; W5
proves real clients, pinned real Redis oracle behavior, hostile bytes, reconnects, and multi-node
HydraCache behavior; W6 records the compatibility surface, executable docs, packaging defaults, and
rollout/rollback evidence so future releases cannot widen it by accident. Every command names its
behavior, unsupported is loud, and the executable contract is the proof it actually interoperates. The
core stays untouched, while the client protocol is explicitly re-scoped for a registered additive v3
TTL metadata/expiry extension with v2 compatibility tests.
