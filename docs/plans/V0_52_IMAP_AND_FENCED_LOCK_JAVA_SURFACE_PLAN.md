# HydraCache 0.52.0 IMap + Fenced Lock Java Surface — Codex Execution Plan

> **At a glance**
> - **What:** *surface what the engine already proves.* Promote the existing in-process
>   single-key fenced-lock primitive (`SingleKeyConditionalStore`, shipped 0.46) into a
>   **supported, wire-exposed, Java-facing distributed lock** with a real **lock lease +
>   auto-release on client-session loss** and **reentrancy** — the Hazelcast `FencedLock`
>   shape — and round out the **IMap data-plane ergonomics** (`replace(k,old,new)`,
>   `remove(k,val)`, entry listeners) on top of the CAS engine that already exists. Then
>   **reverse the Java migration manifest stance** for the lock-by-key subset: from
>   "rejected, use a database lock" to "supported via HydraCache fenced lock".
> - **Why:** the two most-requested Hazelcast migration features — `IMap` and distributed
>   locks — are the ones the current product **actively rejects** in
>   `manifests/unsupported_hazelcast_apis.txt` (`IMap.lock`, `IMap.tryLock`, `FencedLock`,
>   `getCPSubsystem`), even though the linearizable single-key fenced-lock **engine already
>   ships** (`crates/hydracache/src/grid/conditional.rs`). The gap is **surface, not
>   algorithm**: the lock is in-process only, has no lease/session liveness (only a
>   test-only `force_acquire_lock`), and is not in the wire protocol or the Java facade.
>   A fenced lock is *exactly* the documented ceiling in [`../RULES.md`](../RULES.md) R-2
>   ("single-key linearizable conditional writes") — so it ships **without crossing any
>   permanent non-goal**.
> - **After (depends on):** 0.46 (single-key conditional writes + fenced-lock primitive,
>   `SingleKeyConditionalStore`) and 0.49 (stable client wire protocol + Java/Spring
>   migration contract + `unsupported_hazelcast_apis.txt`). Both shipped.
> - **Unblocks:** a credible "Hazelcast CP FencedLock / IMap-lock replacement" migration
>   story; closes the largest gap between the stated migration goal and the shipped surface.
> - **Status:** shipped.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> positioning: [`../POSITIONING.md`](../POSITIONING.md) ·
> competitive analysis: [`../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md`](../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md)

This plan is written for an autonomous coding agent (Codex). Read [`CLAUDE.md`](../../CLAUDE.md),
[`docs/RULES.md`](../RULES.md), and [`docs/GATES.md`](../GATES.md) first. One work item =
one commit/PR; after each, run its Definition of Done **and** `cargo xtask verify`; never
push red. Where behavior is multi-node or crash/restart/partition, add coverage to the
`0.44` `hydracache-sim` deterministic harness.

## Justification (why this, why now)

The honest weakness today is a **positioning/surface contradiction**, not a missing core.

- The fenced-lock engine **exists and is tested**: `SingleKeyConditionalStore` in
  `crates/hydracache/src/grid/conditional.rs` already implements `try_acquire_lock`,
  `release_lock`, `validate_fence_token`, a monotonic `FenceToken`, and
  `compare_and_set` / `put_if_absent`, all gated to a linearizable-capable
  `ConsistencyLevel` (`require_linearizable_level`). This is precisely the
  Martin-Kleppmann / Chubby fencing-token model that Hazelcast's `FencedLock` documents
  (see reference `cashe/hazelcast/.../cp/lock/FencedLock.java` — "Lock holders are ordered
  by a monotonic fencing token … passed to external services to ensure sequential
  execution").
- But the engine is **walled off from users on three sides**:
  1. **In-process only.** It is a deterministic state machine; there are **no lock
     operations in the client wire protocol** (`crates/hydracache-client-protocol/src/lib.rs`
     has `Get/Put/Invalidate/BatchGet/BatchPut/SubscribeInvalidations` — no lock).
  2. **No liveness.** There is **no lease and no auto-release on owner death** — only a
     test-only `force_acquire_lock` whose own doc comment says it is "simulating lease
     expiry / failover in deterministic tests." Hazelcast's lock liveness comes from **CP
     sessions + heartbeats** (`cashe/hazelcast/.../cp/session/CPSession.java`); HydraCache
     has client sessions but never ties lock ownership to them.
  3. **Actively rejected in the Java facade.** `unsupported_hazelcast_apis.txt` tells
     migrants `IMap.lock` / `IMap.tryLock` / `FencedLock` / `getCPSubsystem` are
     unsupported — "use database locks". The migration contract test
     (`crates/hydracache-client-protocol/tests/java_migration_contract.rs`) **asserts**
     these are unsupported, locking the stance in.
- Meanwhile the **IMap data plane is already strong**: the Java facade maps
  `Get/Put/PutIfAbsent/Remove/GetAll/PutAll/BatchGet/BatchPut/ConditionalPutIfAbsent/
  EvictRegion` (`crates/hydracache-client-protocol/src/java_migration.rs`). The remaining
  IMap ergonomic gaps — `replace(k,old,new)`, `remove(k,val)`, entry listeners — all map
  onto primitives that **already exist** (`compare_and_set`, the invalidation bus, cache
  events).

So the cheapest, highest-leverage release is not new consensus work: it is **exposing the
proven primitive** through the wire protocol and the Java facade, adding the **one missing
algorithmic piece (lock lease bound to session liveness)**, and **flipping the manifest
stance** for the lock-by-key subset. This delivers the two features the migration goal
cares about most while staying inside the project's permanent ceiling.

## Release Theme

Turn the shipped single-key fenced-lock engine into a **supported distributed lock** —
leased, session-bound, reentrant, wire-exposed, and presented through a Hazelcast
`FencedLock` / `IMap`-lock-shaped Java facade — and finish the **IMap CAS ergonomics and
entry listeners**, without adding a new consistency level, without a general CP Subsystem,
and without remote code execution (R-1, R-2).

The work is seven items (W1–W6) plus a DST validation item (W7) and explicit deferrals.

## Non-Goals

- **Not a general CP Subsystem.** This release ships the **lock** primitive only. No
  `IAtomicLong`, `ISemaphore`, `ICountDownLatch`, `IAtomicReference` over the wire. The
  manifest keeps `getCPSubsystem` mapped only for the **lock subset**; the rest stays
  unsupported (R-2). A broader CP API is a separate, later decision.
- **No remote code execution.** `IMap.executeOnKey` / entry processors / `addInterceptor`
  remain rejected — moving logic into the grid violates R-2. Entry **listeners** are
  cache-signal subscriptions (existing invalidation bus), not server-side execution.
- **No cross-region linearizable lock.** The fenced lock is **single-partition / single-
  key linearizable** at Quorum+; it is owned in its home region. Cross-region remains
  bounded-staleness / causal+ (R-2). A lock acquired against a region that cannot form a
  quorum **fails loud**, never downgrades (R-3).
- **No new consistency level.** Lock and CAS reuse the existing
  `ConsistencyLevel::allows_single_key_linearizable` gate (0.46); requesting a weak level
  fails with `ConditionalError::WeakConsistency` exactly as today.
- **No Hazelcast binary wire compatibility.** We map Hazelcast **concepts and method
  shapes** (fence token, `tryLock`, lease, reentrancy) to the HydraCache protocol. We do
  **not** implement Hazelcast client protocol codecs; "drop-in" means *source-level
  migration ergonomics*, not binary compatibility. State this in the migration doc.
- **No silent lock loss.** A lock whose lease expires or whose session is lost is released
  and its fence **advances**; any later operation with the stale fence is rejected and
  counted (R-3) — never silently honored.

## Inherited Boundary (assumes 0.46 + 0.49 implemented)

- **0.46 `SingleKeyConditionalStore`** (`grid/conditional.rs`) is the engine: extend it
  with lease + owner identity + reentrancy. Do **not** fork a second lock implementation,
  and do **not** weaken `require_linearizable_level`.
- **0.46 `FenceToken` / `ConditionalError`**: the fence type and error enum are the
  durable/wire contract surface; new variants (e.g. `LeaseExpired`, `ReentrancyLimit`,
  `NotOwner`) extend the enum and are registered in `docs/COMPAT.md` (R-4).
- **0.49 client wire protocol** (`hydracache-client-protocol/src/lib.rs`,
  `ClientRequest`/`ClientResponse`): lock operations are **new request/response variants**
  with a bumped protocol minor version registered in `docs/COMPAT.md`; unknown future
  variants must refuse loud (R-4).
- **0.49 Java migration facade** (`hydracache-client-protocol/src/java_migration.rs`,
  `JavaMapOperation` / `JavaMapProtocolFamily` / `UnsupportedHazelcastApiManifest`): the
  lock + new IMap ops extend these enums; the manifest entries move from unsupported to
  supported-mapping and the contract test updates with them.
- **0.49 client session** (the authenticated client identity already negotiated by
  `hydracache-client`): lock leases bind to this session; **no new session/identity type
  is introduced** — the lock lease is layered on the existing client session + a
  heartbeat watermark.
- **0.44 DST harness** (`hydracache-sim`): all multi-node / partition / session-expiry /
  zombie-holder behavior is validated there (W7).

## Dependency Graph

```
0.46 single-key fenced-lock engine ── 0.49 client protocol + Java migration contract
        │
        ▼
W1 lock lease + session-bound ownership + auto-release (the one missing algorithm)
        │
        ├──────────────► W2 reentrancy + owner identity (Hazelcast default-reentrant FencedLock)
        ▼
W3 lock operations in the client wire protocol (TryLock/Lock/Unlock/GetFence/IsLocked)
        │
        ▼
W4 Java FencedLock + IMap-lock facade + REVERSE the unsupported manifest stance
        │
        ├──────────────► W5 IMap CAS ergonomics: replace(k,old,new), remove(k,val)
        ├──────────────► W6 IMap entry listeners over the invalidation bus
        ▼
W7 DST validation: mutual exclusion under partition, session-expiry fence advance,
   zombie-holder rejection, reentrancy limit, lock linearizability
```

W1 is the long pole: the lease + session liveness is the only genuinely new algorithm;
everything else exposes or maps existing primitives.

Conventions per work item: **Goal / Files / Steps / Definition of Done (tests + exact
`cargo`/CI) / Risk & rollback.**

---

## W1. Lock lease + session-bound ownership + auto-release (the missing algorithm)

**Goal.** Give the fenced lock real **liveness**: an acquisition carries a **lease** and is
owned by a **client session**; when the lease expires or the session stops heart-beating,
the lock is released and the **fence advances** so a stale holder can never win a later
race. This is the HydraCache equivalent of Hazelcast CP sessions + lock leases.

**Hazelcast reference.** `cashe/hazelcast/.../cp/lock/FencedLock.java` (the GC-pause /
session-expiry scenario where a paused client loses ownership and a later write with the
old fence is rejected), `cashe/hazelcast/.../cp/session/CPSession.java`
(`isExpired(timestamp)`, heartbeat interval), `IMap.lock(key, leaseTime, TimeUnit)`
(`cashe/hazelcast/.../map/IMap.java`).

**Files.** Extend `crates/hydracache/src/grid/conditional.rs`
(`SingleKeyConditionalStore` lock state: `LockHold { owner: SessionId, fence: FenceToken,
lease_deadline: LogicalDeadline, reentrancy: u32 }`; new `ConditionalError::LeaseExpired`
/ `NotOwner`). Add `crates/hydracache/src/grid/lock_session.rs` (new: `SessionId`,
`SessionHeartbeats`, logical-time deadline helpers). Register the new error/state in
`docs/COMPAT.md`.

**Steps.**
1. Replace `locks: BTreeMap<String, FenceToken>` with `locks: BTreeMap<String, LockHold>`
   carrying owner session, fence, and a **logical** lease deadline (epoch/version/logical
   clock — **never wall-clock**, R-5). `try_acquire_lock` takes `(key, level, owner,
   lease)`; succeeds only if the key is unheld **or** the current hold's lease is expired
   at the supplied logical "now". On a successful steal of an expired hold, **advance the
   fence** (new token) and count it.
2. Add `renew_lease(key, owner, token, new_deadline)` (heartbeat extends the lease) and
   `expire_due(now)` (releases all holds whose lease deadline ≤ now, advancing fence and
   bumping a `lock_lease_expired_total` counter). Make `release_lock` require the **current
   owner** (`NotOwner` otherwise) in addition to the fence check it already does.
3. Tie ownership to a session heartbeat watermark: a `SessionHeartbeats` map records the
   last logical heartbeat per session; a session is "lost" when its watermark falls more
   than the configured lease behind "now". Losing a session expires **all** its holds via
   the same fence-advancing path (R-3: fail loud, never silently honor a zombie).

**DoD.** `crates/hydracache/tests/lock_lease.rs`
- `expired_lease_can_be_stolen_and_fence_advances` (unit) — old fence < new fence.
- `stale_holder_release_after_expiry_is_rejected` (unit) — `StaleFenceToken`/`NotOwner`.
- `heartbeat_renew_keeps_ownership` (unit) — renew prevents the steal.
- `session_loss_releases_all_its_locks_and_advances_fence` (unit) — zombie-holder safety.
- `release_by_non_owner_is_rejected_and_counted` (unit) — `NotOwner`.
- Run: `cargo test -p hydracache --locked lock_lease` + `cargo xtask verify`.

**Risk & rollback.** Changes the lock state shape inside one struct; the public
acquire/release signatures gain parameters. Keep logical-clock deadlines (no wall-clock) so
DST stays deterministic. Revert restores the `FenceToken`-only map.

---

## W2. Reentrancy + owner identity (Hazelcast default-reentrant FencedLock)

**Goal.** Match Hazelcast's default-reentrant `FencedLock`: the **same owner** may acquire
the lock multiple times (incrementing a hold count) and must `unlock` the same number of
times; a configurable **reentrancy limit** fails loud when exceeded.

**Hazelcast reference.** `FencedLock.java` ("By default, `FencedLock` is reentrant … you
can configure the reentrancy behaviour via `FencedLockConfig` … When the reentrancy limit
is reached … fails with `LockAcquireLimitReachedException`"), `isLockedByCurrentThread()`,
`cashe/hazelcast/.../config/cp/FencedLockConfig.java` (`lockAcquireLimit`).

**Files.** Extend `grid/conditional.rs` (`LockHold.reentrancy`, `lock_acquire_limit` config
on the store), `ConditionalError::ReentrancyLimit { limit }`.

**Steps.**
1. On `try_acquire_lock` when the key is already held **by the same owner**: increment the
   hold count and return the **existing** fence (Hazelcast keeps the fence stable across
   reentrant acquisitions — only a fresh ownership assignment bumps the fence). Honor
   `lock_acquire_limit`: a count past the limit returns `ReentrancyLimit` (R-3), never
   blocks or silently caps.
2. `release_lock` decrements the hold count; the lock is only freed (and removed from the
   map) when the count reaches zero. A non-owner unlock is `NotOwner` (from W1).
3. Add read helpers: `is_locked(key)`, `is_locked_by(key, owner)`, `current_fence(key)` —
   the engine side of Hazelcast's `isLocked()` / `isLockedByCurrentThread()` / `getFence()`.

**DoD.** `crates/hydracache/tests/lock_reentrancy.rs`
- `reentrant_acquire_keeps_same_fence_and_counts` (unit).
- `unlock_frees_only_at_zero_holds` (unit).
- `reentrancy_limit_fails_loud` (unit) — `ReentrancyLimit`.
- `is_locked_by_owner_reflects_state` (unit).
- Run: `cargo test -p hydracache --locked lock_reentrancy`.

**Risk & rollback.** Pure state-machine extension; default limit chosen to match
Hazelcast's "unbounded reentrancy by default" (configurable). Revert removes the count
field and the limit config.

---

## W3. Lock operations in the client wire protocol

**Goal.** Expose the engine over the network: a client can acquire/release a fenced lock on
the partition leader that owns the key, with the fence returned to the caller so it can be
forwarded to an external system of record.

**Hazelcast reference.** `cashe/hazelcast/.../client/impl/protocol/codec/FencedLockLockCodec.java`,
`FencedLockTryLockCodec.java`, `FencedLockUnlockCodec.java`,
`FencedLockGetLockOwnershipCodec.java` — the request/response shapes (lock/tryLock with a
timeout, unlock, ownership query returning fence + owner).

**Files.** Extend `crates/hydracache-client-protocol/src/lib.rs` (`ClientRequest::{TryLock,
Unlock, RenewLockLease, GetLockOwnership}`, `ClientResponse::{LockAcquired{fence},
LockBusy, LockReleased, LockOwnership{fence, locked}}`), bump the protocol minor in
`docs/COMPAT.md`. Wire the server side in `crates/hydracache-server` to route to the
owning partition leader and call the W1/W2 store; extend `crates/hydracache-client/src/lib.rs`
with `try_lock` / `unlock` / `renew_lock_lease` / `lock_ownership` methods next to `get`/`put`.

**Steps.**
1. Add the lock request/response variants. `TryLock { ns, key, lease_ms, wait_ms }` returns
   `LockAcquired { fence }` or `LockBusy`. `lease_ms` maps to a **logical** lease at the
   server (translated from the client's wall-clock request, but stored logically). Unknown
   future variants **refuse loud** on decode (R-4).
2. Server-side: route a lock op to the **leader of the key's partition** (reuse existing
   single-key routing); reject at a non-linearizable level with the existing
   `WeakConsistency` error surfaced as a protocol error envelope. Bind the lease to the
   request's **authenticated client session**; the client transport renews via
   `RenewLockLease` on its existing heartbeat path.
3. Client-side convenience: `HydraCacheClient::try_lock(ns, key, lease)` returning a
   `LockGuard { fence }` whose `Drop`/explicit `unlock()` releases; expose `fence()` for
   forwarding. Surface `lock_acquired_total` / `lock_busy_total` / `lock_lease_renew_total`
   as bounded-label client metrics (R-6).

**DoD.** `crates/hydracache-client-protocol/tests/lock_wire.rs` +
`crates/hydracache-server/tests/lock_endpoint.rs`
- `lock_request_response_roundtrips` (unit) — encode/decode incl. fence.
- `unknown_future_lock_variant_refuses_loud` (unit) — R-4.
- `weak_level_lock_returns_weakconsistency_envelope` (unit).
- `two_clients_contend_one_wins_fence_monotonic` (integration, server) — second client
  sees `LockBusy`, the winner's fence is monotonic.
- `lease_renew_extends_then_expiry_frees` (integration, server).
- Run: `cargo test -p hydracache-client-protocol --locked lock_wire`,
  `cargo test -p hydracache-server --locked lock_endpoint`, + `cargo xtask verify`.

**Risk & rollback.** New wire surface ⇒ COMPAT entry + protocol minor bump are mandatory
(doc-check gate). Keep lock ops **off** the cache fast path (separate request family).
Revert removes the variants and the COMPAT row in the same commit.

---

## W4. Java FencedLock + IMap-lock facade — reverse the manifest stance

**Goal.** Present the lock through a Hazelcast-shaped Java facade and **flip the migration
manifest** for the lock-by-key subset from "rejected" to "supported mapping", so a Java
team migrating `IMap.lock` / `CPSubsystem.getLock` has a documented, near drop-in path.

**Hazelcast reference.** `FencedLock.java` method set — `lock()`, `lockAndGetFence()`,
`tryLock()`, `tryLockAndGetFence()`, `tryLock(time, unit)`, `unlock()`, `getFence()`,
`isLocked()`, `isLockedByCurrentThread()`; `IMap.java` — `lock(key)`,
`lock(key, leaseTime, unit)`, `tryLock(key[, time, unit])`, `unlock(key)`,
`forceUnlock(key)`.

**Files.** Extend `crates/hydracache-client-protocol/src/java_migration.rs`
(`JavaLockOperation` enum + `JavaLockProtocolFamily`, mapping to the W3 wire ops; manifest
helpers). **Edit** `crates/hydracache-client-protocol/manifests/unsupported_hazelcast_apis.txt`
(move `IMap.lock`, `IMap.tryLock`, `FencedLock`, and a `getCPSubsystem` **lock-only** note
to a new supported-mapping section / remove from unsupported). **Edit**
`crates/hydracache-client-protocol/tests/java_migration_contract.rs` (the
`unsupported_hazelcast_api_surface_is_a_checked_in_manifest` test currently asserts
`IMap.lock` and `FencedLock` are unsupported — update it to assert they are now **mapped**,
while `IMap.executeOnKey`, `getSql`, `getExecutorService`, `ReplicatedMap`, `ReliableTopic`
stay unsupported). Update `docs/integrations/java-migration.md` with the lock mapping table
and the GC-pause/fencing caveat.

**Steps.**
1. Add `JavaLockOperation { Lock, LockAndGetFence, TryLock, TryLockTimed, Unlock,
   GetFence, IsLocked, IsLockedByCurrentThread, ForceUnlock }` and map each to a
   `JavaLockProtocolFamily` over the W3 wire ops (e.g. `LockAndGetFence` → `TryLock` with
   blocking-wait semantics returning the fence). Document that `forceUnlock` maps to an
   **admin/fence-advancing** release and is privileged.
2. Rewrite the manifest entries: `IMap.lock|...` / `FencedLock|...` move to a **supported
   mapping** with the migration hint "use `HydraFencedLock` (single-key linearizable fenced
   lock); pass the returned fence to your system of record" and the explicit caveat that it
   is **not reentrant across processes by identity unless a session is supplied** and is
   **not cross-region linearizable**. Keep `getCPSubsystem` documented as **lock-only**
   today. Bump `JAVA_MIGRATION_CONTRACT_VERSION` and update `docs/COMPAT.md`.
3. Update the contract test to the new stance and add a positive assertion that the lock
   operations resolve to the correct protocol family. Add the "Hazelcast concept →
   HydraCache equivalent" lock table to the migration doc (state: source-level ergonomics,
   not binary wire compatibility).

**DoD.** `crates/hydracache-client-protocol/tests/java_migration_contract.rs` (updated) +
`crates/hydracache-client-protocol/tests/java_lock_mapping.rs` (new)
- `lock_apis_are_now_supported_mapping_not_rejected` (unit) — manifest no longer lists
  `IMap.lock` / `FencedLock` as unsupported; still lists `executeOnKey`/`getSql`/etc.
- `java_lock_operation_maps_to_wire_family` (unit) — every `JavaLockOperation` resolves.
- `migration_contract_version_bumped_and_documented` (unit) — version + COMPAT + docs.
- `force_unlock_is_marked_privileged` (unit).
- Run: `cargo test -p hydracache-client-protocol --locked java_migration_contract java_lock_mapping`.

**Risk & rollback.** This is a **stance reversal**: the change is mostly the manifest +
test + docs, and is the politically load-bearing edit. Keep the non-lock unsupported
entries intact so we don't over-claim (R-7). Revert restores the prior manifest and test.

---

## W5. IMap CAS ergonomics — `replace(k, old, new)` and `remove(k, val)`

**Goal.** Close the remaining IMap conditional-write gaps using the **existing**
`compare_and_set` engine — no new algorithm.

**Hazelcast reference.** `IMap.java` — `replace(K, V oldValue, V newValue)` (CAS),
`replace(K, V)` (replace-if-present), `remove(K, V)` (remove-if-equal),
`putIfAbsent` (already mapped).

**Files.** Extend `crates/hydracache-client-protocol/src/java_migration.rs`
(`JavaMapOperation::{Replace, ReplaceIfPresent, RemoveIfValue}` →
`JavaMapProtocolFamily::ConditionalReplace` / `ConditionalRemove`), add the matching
`ClientRequest::{CompareAndSet, RemoveIfValue}` wire variants
(`hydracache-client-protocol/src/lib.rs`) mapped to `SingleKeyConditionalStore::compare_and_set`.

**Steps.**
1. Add `CompareAndSet { ns, key, expected, new_value }` and `RemoveIfValue { ns, key,
   expected }` wire variants returning `CasApplied { new_version }` / `CasMismatch {
   current }` (mirror the existing `CasResult`). Route to the partition leader; require a
   linearizable level (reuse the W3 path).
2. Map `JavaMapOperation::Replace` → `CompareAndSet`, `RemoveIfValue` → `RemoveIfValue`,
   `ReplaceIfPresent` → `CompareAndSet` with an "any current value" form documented as a
   distinct call (do **not** silently treat absent as match — R-3).
3. Extend the client convenience methods (`replace`, `remove_if`) and bounded metrics.

**DoD.** `crates/hydracache-client-protocol/tests/imap_cas.rs`
- `replace_with_matching_old_applies_and_bumps_version` (unit).
- `replace_with_stale_old_returns_mismatch_current` (unit).
- `remove_if_value_matches_then_tombstones` (unit).
- `replace_if_present_on_absent_is_mismatch_not_insert` (unit) — R-3.
- `java_replace_maps_to_conditional_replace_family` (unit).
- Run: `cargo test -p hydracache-client-protocol --locked imap_cas`.

**Risk & rollback.** Thin mapping over a shipped primitive; revert removes the variants
and the `JavaMapOperation` members.

---

## W6. IMap entry listeners over the invalidation bus

**Goal.** Offer `IMap.addEntryListener`-style notifications by mapping them onto the
**existing** invalidation subscription + cache-event surface — a cache signal, not
server-side execution.

**Hazelcast reference.** `IMap.java` — `addEntryListener(MapListener, boolean includeValue)`
and the `EntryAddedListener` / `EntryUpdatedListener` / `EntryRemovedListener` /
`EntryEvictedListener` family. **Caveat to document:** Hazelcast can include the value;
HydraCache near-cache invalidation is a **freshness signal** (key + reason), so value
inclusion is best-effort via a follow-up `get`, not guaranteed in the event.

**Files.** Extend the `SubscribeInvalidations` wire surface
(`hydracache-client-protocol/src/lib.rs`) with an entry-event projection; map
`JavaMapOperation::AddEntryListener` → `JavaMapProtocolFamily::SubscribeInvalidations`;
client-side adapter in `hydracache-client` turning invalidation frames into
`EntryEvent { key, kind: Added|Updated|Removed|Evicted }`.

**Steps.**
1. Project existing invalidation/event reasons onto the Hazelcast entry-event kinds
   (key-write → Added/Updated, tombstone → Removed, capacity/TTL → Evicted). Do not invent
   events the bus does not carry; unmappable reasons surface as a generic `Invalidated`
   kind (R-3, no fabricated semantics).
2. Honor the existing **bounded subscriber buffer + lag diagnostics** (v0 events); a slow
   listener is dropped-with-a-counter, never an unbounded queue (R-3, R-6).
3. Document the **value-inclusion caveat** and the **at-least-once / coalesced** nature of
   the signal in the migration doc — explicitly *not* a business event log (consistent with
   the existing `Ringbuffer` / `ReliableTopic` unsupported entries, which stay unsupported).

**DoD.** `crates/hydracache-client-protocol/tests/imap_entry_listener.rs`
- `invalidation_reasons_project_to_entry_event_kinds` (unit).
- `unmappable_reason_falls_back_to_invalidated_kind` (unit).
- `slow_listener_is_dropped_with_counter_not_unbounded` (unit).
- `add_entry_listener_maps_to_subscribe_family` (unit).
- Run: `cargo test -p hydracache-client-protocol --locked imap_entry_listener`.

**Risk & rollback.** Reuses the shipped subscription path; the only new surface is the
event projection. Revert removes the projection and the `JavaMapOperation` member.

---

## W7. DST validation — lock safety under partition, expiry, and contention

**Goal.** Prove the lock's safety properties in the `0.44` deterministic simulator:
**mutual exclusion**, **fence monotonicity**, **zombie-holder rejection**, and
**reentrancy-limit** behavior under partitions, session loss, and leader change — the
Jepsen-class evidence that makes "distributed lock" a defensible claim (R-7: claims ride
explicit gates).

**Hazelcast reference.** The `FencedLock.java` GC-pause scenario (paused owner loses the
lock; its later fenced write is rejected) is the canonical invariant to encode.

**Files.** `crates/hydracache-sim/tests/lock_safety_sim.rs` (new), reusing the seeded
network + fault-injecting harness and the linearizability/invariant checkers.

**Steps.**
1. Model N clients contending for one key across a simulated cluster with injected
   partitions, leader changes, and session-heartbeat loss. Assert the **mutual-exclusion
   invariant**: at most one live owner per (key, fence) at any committed point.
2. Encode the **zombie-holder invariant**: after a session-loss-induced release, an
   operation carrying the **old fence** is rejected; the new owner's fence is strictly
   greater. Replay/shrink on failure with the recorded seed (existing harness capability).
3. Assert lock acquisition **never violates the consistency gate** (no lock acquired at a
   weak level), and reentrancy-limit overflow fails loud rather than deadlocking.

**DoD.** `crates/hydracache-sim/tests/lock_safety_sim.rs`
- `mutual_exclusion_holds_under_partition_and_leader_change` (sim, seeded).
- `session_loss_advances_fence_and_rejects_zombie_writer` (sim, seeded).
- `fence_is_strictly_monotonic_across_ownership_changes` (sim, seeded).
- `no_lock_acquired_at_weak_consistency_level` (sim, seeded).
- Each test logs and replays its seed (R-5).
- Run: `cargo test -p hydracache-sim --locked lock_safety_sim` + `cargo xtask verify`.

**Risk & rollback.** Test-only; if the harness lacks a session-loss fault, add it to the
shared fault injector (`crates/hydracache/tests/support/fault_injector.rs`) in the same PR.
No production surface to roll back.

---

## Deferred (explicitly not in 0.52)

- **General CP Subsystem** (`IAtomicLong`, `ISemaphore`, `ICountDownLatch`,
  `IAtomicReference`) — a later release; only the lock ships now.
- **Entry processors / `executeOnKey` / interceptors** — permanent non-goal (R-2 remote
  code execution).
- **Cross-region distributed lock** — out of scope (R-2 cross-region linearizability); the
  lock is single-partition, home-region.
- **Management Center-style lock UI** — admin visibility is via existing metrics/actuator;
  a UI is a separate operability decision.
- **Hazelcast binary wire/codec compatibility** — we ship source-level migration
  ergonomics, not protocol compatibility.

## Gates (Definition of Done for the release)

- `cargo xtask verify` green (fmt, clippy, tests, doc-check, COMPAT, deny).
- New wire/durable surfaces (lock ops, CAS ops, new `ConditionalError` variants, protocol
  minor, migration contract version) registered in `docs/COMPAT.md`; doc-check passes.
- `releases.toml` + `INDEX.md` updated to `0.52.0` (this plan) and kept consistent.
- `POSITIONING.md` / `COMPETITIVE_ANALYSIS_AND_EVOLUTION.md` updated: "distributed lock" and
  "IMap-lock migration" move from gap to shipped, with the fenced-lock caveats stated.
- The `FEATURE_MATRIX.md` lock/Java-facade rows added.
- No new numeric self-score (R-7); the release ships on the boolean gates above.

---

# Implementation Appendix — hard parts, reference blueprints, and review patches

This appendix answers the implementation-difficulty review. It (a) folds in the
sequencing/patch items raised, and (b) gives concrete, reference-grounded guidance for
the three items that are heavier than their one-line W-statements suggest: the
**lease/session lifecycle (W1/W2)**, the **wire+server service path (W3)**, and the
**conditional tombstone (W5)**. Reference checkouts live under `C:\Workspace\prj\jq\cashe`.

## A0. Revised sequencing (per review)

Build strictly **W1 → W2** first (pure engine, no I/O). **Before W3**, land a small
**"Pre-W3 patch"** (§A2.4–A2.6 below) covering async-`Drop`, mixed-version negotiation, and
the conditional-tombstone engine method, because W3 and W5 both depend on those decisions.
Then W3 → W4 → (W5, W6 in parallel) → W7.

## A1. The load-bearing correctness decision (affects W1 **and** W3)

**Assign the fence token and register the lock session on the raft *apply* path, never at
*propose* time.** This is the tigerbeetle `src/vsr/client_sessions.zig` lesson: sessions are
"register[ed] … explicitly through the state machine to ensure that session numbers always
increase", with a careful committed-vs-uncommitted distinction because "uncommitted requests
may not survive a view change." If HydraCache hands out a fence optimistically before the
lock command commits, a leader change can replay/reorder and resurrect a stale fence —
breaking the single monotonic-owner invariant the whole feature exists to provide.

Concretely: `SingleKeyConditionalStore` becomes a **deterministic raft state machine**. A
lock op is `propose(LockCommand)` → wait for commit → the **apply** function mutates the
store and returns the assigned fence. `next_fence` lives *inside* the applied state (it
already does), so it only advances on apply, deterministically, on every replica. This makes
W1's lease logic and W3's wiring the same state machine, not two code paths.

## A2. W1/W2 — lease + session + reentrancy lifecycle

### A2.1 Reference model (Hazelcast CP)

Hazelcast's `ProxySessionManager`
(`cashe/hazelcast/.../cp/internal/session/ProxySessionManager.java`) is the proven shape:
- `acquireSession(groupId, count)` — **reentrancy is an acquire *count* on a session**, not a
  separate lock field. Re-acquire by the same owner increments the count; `releaseSession(…,
  count)` decrements; the lock frees at zero.
- `invalidateSession(groupId, id)` — "no more heartbeats will be sent"; the server-side
  session is the **liveness token**.
- `getOrCreateUniqueThreadId(groupId)` — **owner identity = (session id, unique thread/endpoint
  id)**. This is exactly what `isLockedByCurrentThread()` checks.

Map to HydraCache: `LockOwner { session: SessionId, endpoint: u64 }`, reentrancy = a hold
count on the `LockHold`, lease = a logical deadline owned by the session.

### A2.2 State shapes (extend `grid/conditional.rs`)

```rust
pub struct SessionId(u64);                 // assigned on apply, monotonic (tigerbeetle lesson)

struct LockHold {
    owner: LockOwner,                      // (session, endpoint)
    fence: FenceToken,                     // stable across reentrant holds; only a *new* owner bumps it
    holds: u32,                            // reentrancy count (Hazelcast acquire count)
    lease_deadline: LogicalTime,           // epoch/version/logical clock — NEVER wall-clock (R-5)
}

// new errors (extend ConditionalError; register in docs/COMPAT.md, R-4)
LeaseExpired { key, current: Option<FenceToken> }
NotOwner     { key, current_owner: Option<LockOwner> }
ReentrancyLimit { key, limit: u32 }
```

`locks: BTreeMap<String, FenceToken>` becomes `BTreeMap<String, LockHold>`.

### A2.3 Operations (all are pure functions of state + a logical `now`)

- `try_acquire(key, owner, lease, now)`: if unheld **or** `held.lease_deadline <= now` →
  grant, **bump fence**, count it (steal of an expired hold is the failover path). If held by
  the **same owner** → increment `holds` (honor `lock_acquire_limit`, else `ReentrancyLimit`),
  return the **existing** fence (Hazelcast keeps fence stable across reentrancy). Else →
  `LockBusy`.
- `renew(key, owner, token, new_deadline)`: heartbeat extends `lease_deadline`; rejects on
  `NotOwner`/stale token.
- `release(key, owner, token)`: require current owner **and** current fence; decrement
  `holds`; remove at zero. Non-owner → `NotOwner` (counted).
- `expire_due(now)`: sweep holds with `lease_deadline <= now`; each release **advances the
  fence** and bumps `lock_lease_expired_total`. Session loss (heartbeat watermark too old)
  expires **all** that session's holds via this same path (R-3: never silently honor a zombie).

### A2.4 Pre-W3 patch — `LockGuard::Drop` cannot do async unlock

Rust `Drop` is synchronous; a client `LockGuard` **must not** attempt an `async` network
unlock in `Drop` (no runtime guarantee, can't await, can panic on a dropped runtime). The
contract is therefore: **explicit `guard.unlock().await`** is the release path; the
**server-side lease expiry is the safety net** if the client dies or forgets. `Drop` may only
do a *best-effort, non-blocking* "abandon" hint (e.g. enqueue a fire-and-forget release on the
existing client channel) and must be documented as **not** a guaranteed release. Add this to
W3's client-API DoD and to the migration doc (Hazelcast `Lock.unlock()` is also explicit; this
matches expectations).

### A2.5 W1/W2 tests (engine-level; extend the DoD already in W1/W2)

Already specified in W1/W2; add two grounded by the references:
- `fence_assigned_only_on_apply_not_propose` — propose a lock, simulate leader change before
  apply, assert no fence leaked (A1 invariant).
- `reentrancy_is_acquire_count_on_session` — same (session,endpoint) increments; different
  endpoint on same session is `LockBusy` (Hazelcast thread-id semantics).

## A3. W3 — wire + server service path (the real weight)

### A3.1 Current reality (sized honestly)

- The request dispatch is `handle_request` in
  `crates/hydracache-client-transport-axum/src/lib.rs` — a `match envelope.request` that
  today holds a plain `Mutex<store>` (an in-memory map), **not** a `SingleKeyConditionalStore`,
  and is **not** partition/leader-aware.
- The server rejects on **strict** version equality: `if envelope.protocol_version !=
  PROTOCOL_VERSION` (line ~359). There is a real negotiation seam already —
  `VersionHandshake::negotiate` returns the highest common version — but `Default` is
  `min=max=PROTOCOL_VERSION` (currently `1`) and the dispatch ignores it.
- `SingleKeyConditionalStore` is referenced **only** from `crates/hydracache/` (and its test);
  nothing in `hydracache-server`/transport can see it yet.

So W3 = (1) protocol variants, (2) a **lock/CAS service** added to server state, (3)
**leader routing + propose/wait/apply**, (4) negotiation. Items 2–3 are the weight.

### A3.2 Backbone already exists — reuse the raft propose/commit seam

`crates/hydracache-cluster-raft/src/log_store.rs` exposes `propose(data) -> index` and
`committed_payloads_on(node_id) -> Vec<payload>`. The lock service path is the standard
replicated-state-machine loop, mirroring how qdrant drives ops through `raft-rs` consensus
(`cashe/qdrant/.../consensus*`):

```
client → transport handle_request
       → LockService (new, in server state)
           → if not leader: redirect/forward to leader (NotLeader response carrying leader hint)
           → serialize LockCommand::{TryLock,Unlock,Renew,Cas,RemoveIfValue}
           → raft.propose(cmd)                       // returns log index
           → await commit (index applied)
           → apply(cmd) on SingleKeyConditionalStore // fence assigned HERE (A1)
           → reply { fence | LockBusy | Mismatch }
```

The apply step is shared by every replica, so the conditional store stays deterministic and
DST-checkable (W7). Keep lock/CAS ops on a **separate request family** from the cache
fast-path (they pay a raft round-trip; `get`/`put` must not).

### A3.3 Mixed-version protocol patch (Pre-W3)

1. Bump `PROTOCOL_VERSION` to `2`; set client/server `VersionHandshake` to `min=1, max=2`.
2. Replace the strict `protocol_version != PROTOCOL_VERSION` reject with: decode at the
   **negotiated** version; accept `[min, max]`. Register the v2 frames in `docs/COMPAT.md` with
   the reader window (R-4).
3. **Gate lock/CAS variants on negotiated ≥ 2.** A v1 client must **never** receive a v2-only
   response shape; a v1 client that somehow sends a v2 op gets a loud `IncompatibleVersion`
   (R-3), not a silently-different reply. Add `mixed_version_v1_client_never_sees_v2_response`
   and `lock_op_requires_negotiated_v2` tests.

### A3.4 W3 tests (extend the DoD already in W3)

Keep the W3 DoD list; add `not_leader_forwards_or_redirects` (leader routing) and the two
mixed-version tests from §A3.3.

## A4. W4 — `forceUnlock` authorization + Java-facade honesty

- **`forceUnlock` is privileged.** It maps to a **fence-advancing admin release** (steal
  ownership without holding the token), so it must run through the existing identity/authz
  path (`0.42`/`0.49` node identity + authz), be **rejected for non-admin callers** (R-3), and
  emit an **audit event** on the existing consumer-audit surface (`0.49`). Add
  `force_unlock_requires_admin_and_audits` and `force_unlock_advances_fence`.
- **No Java SDK artifact is promised.** The repo has **no Maven/Gradle/Java module**; `0.52`
  ships the **migration *contract* + facade *surface*** (Rust-side `JavaLockOperation` mapping,
  manifest, docs), not a published `.jar` client. State explicitly in the plan's Non-Goals and
  the migration doc: a buildable Java client artifact is a **separate, later work item**, not
  implied by `0.52`. (This keeps R-7 honesty: we do not claim a deliverable we are not building.)

## A5. W5 — conditional tombstone engine method (not just a protocol name)

`compare_and_set` today only writes a **value** record; `remove(k, val)` needs a **conditional
tombstone**. The primitive already exists — `SingleKeyConditionalStore::apply_tombstone` and
`ReplicatedValueRecord::tombstone` — so add a real engine method, don't fake it at the wire
layer:

```rust
pub fn remove_if_value(&mut self, key, expected: &[u8], level)
    -> Result<CasResult, ConditionalError>
{
    require_linearizable_level(level)?;
    let current = self.records.get(key).and_then(current_bytes);
    if current.as_deref() != Some(expected) {
        self.metrics.cas_mismatch_total += 1;
        return Ok(CasResult::Mismatch { current });
    }
    let version = self.next_version; self.next_version += 1;
    // tombstone at an explicit version — A5 delete semantics, never resurrect (R-3)
    self.apply_tombstone(key, version);
    self.metrics.cas_applied_total += 1;
    Ok(CasResult::Applied { new_version: version })
}
```

**Engine-level tests (in addition to the protocol mapping in W5):**
- `remove_if_value_writes_tombstone_at_new_version`.
- `remove_if_value_mismatch_leaves_record_untouched`.
- `tombstone_from_remove_if_value_is_not_resurrected_by_stale_put` (R-3).
- `removed_key_reads_as_absent_after_tombstone`.

## A6. W6 — entry-event projection honesty + listener-is-not-an-event-log

Do **not** invent `Added` vs `Updated` if the invalidation bus does not carry enough state to
distinguish them. Inspect the existing event/invalidation reasons first; if the bus only knows
"this key changed", expose **`Upserted`** (or `Invalidated`) plus a documented **follow-up
`get`** for the value, rather than fabricating a transition the stream cannot prove (R-3). Only
emit `Added`/`Updated`/`Removed`/`Evicted` for reasons the bus actually distinguishes.

**Add the contract test the review asks for:**
- `entry_listener_is_not_a_business_event_log` — assert the surface keeps **coalescing**, a
  **bounded buffer**, and **lag/drop counters** (a slow listener is dropped-with-a-counter, not
  buffered unboundedly), so callers cannot mistake it for `Ringbuffer`/`ReliableTopic` (which
  stay unsupported). This makes the "cache signal, not event log" boundary executable.

## A7. W7 — lock workload + invariants in `hydracache-sim`

`hydracache-sim` is seeded and exists, but has **no lock workload or lock invariants yet** —
this is mandatory, not optional, or the "distributed lock" claim is unbacked. Add a lock client
workload (N contending clients, one key) and encode the invariants as checkers in the existing
harness: mutual exclusion per `(key, fence)`, strict fence monotonicity across ownership
changes, zombie-writer rejection after session-loss, and "no lock at a weak consistency level".
Reference the tigerbeetle simulator discipline (`cashe/tigerbeetle/src/testing/`,
`src/vopr.zig`): seeded, replayable, shrinking on failure.

## Reference index (for the implementer)

| Hard part | Reference (under `cashe/`) | What to copy |
| --- | --- | --- |
| Session liveness, monotonic ids, committed-vs-uncommitted | `tigerbeetle/src/vsr/client_sessions.zig` | register sessions through the state machine; fence/session on apply (A1) |
| Reentrancy as acquire-count, owner = session+thread, invalidate | `hazelcast/.../cp/internal/session/ProxySessionManager.java` | lock state shape (A2.1) |
| Fenced-lock semantics + GC-pause/fence story | `hazelcast/.../cp/lock/FencedLock.java` | the user-facing contract + W7 zombie invariant |
| Drive ops through raft consensus to a leader | `qdrant/.../consensus*`, `hydracache-cluster-raft/src/log_store.rs` (`propose`/`committed_payloads_on`) | W3 propose/commit/apply loop (A3.2) |
| Seeded simulator, replay/shrink | `tigerbeetle/src/testing/`, `src/vopr.zig` | W7 lock workload + invariants |
