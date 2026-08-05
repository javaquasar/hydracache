# ADR-0018: Hazelcast Compatibility Surface Boundaries

Status: accepted. Supersedes no prior ADR; constrains all future
Hazelcast-migration planning (`0.52`, the `0.68` client-plane foundation, `0.69` conformance,
and any later migration release).

HydraCache's core positioning is "safe migration target for Hazelcast
applications." "Hazelcast application" is not one thing, so "support Hazelcast"
is not one decision. This ADR fixes **which Hazelcast integration surfaces
HydraCache will and will not implement**, why, and the honest constraints on
each. It exists so that no future plan re-opens a settled boundary or promises a
surface that does not exist (the false-surface failure mode `0.66`/`0.67` had to
reconcile after the fact).

## Context: an app uses Hazelcast in one of four ways

The official Hazelcast connector is used in fundamentally different modes that
differ in migration difficulty by roughly two orders of magnitude. Any migration
plan must classify the target app by mode first.

1. **Embedded, non-interop.** The app calls `Hazelcast.newHazelcastInstance()`
   for convenience (single node or a small self-contained cluster it owns
   entirely) and uses `IMap`, `FencedLock`, `JCache`, near-cache, or Hibernate
   L2. It does **not** need to co-cluster with foreign Hazelcast members.
2. **Client.** The app calls `HazelcastClient.newHazelcastClient()` and talks
   over the network to a cluster using the Hazelcast **Open Binary Client
   Protocol**.
3. **Member co-clustering.** The app is a full cluster **member** and must
   participate in a **mixed** cluster alongside real Hazelcast Java members via
   the internal member-to-member protocol (join/discovery, partition table +
   migration, CP Raft, gossip, split-brain merge).
4. **Gradual migration bridge.** The app (or the org) needs to run both systems
   during a cutover window and move data/traffic incrementally.

## Decision

### D1. Embedded, non-interop (mode 1): **supported** — the primary path

Replace `Hazelcast.newHazelcastInstance()` with an embedded HydraCache instance
behind a Hazelcast-shaped `IMap`/`FencedLock`/`JCache` Java facade (the `0.52`
contract, once shipped as a real Java artifact — see the "honest gap" below).
This is a **drop-in facade**, not protocol interop: the app changes a dependency
and config, not code. It is the tractable path for the majority of embedded
Hazelcast apps, which use embedded mode for convenience rather than for
co-membership.

Boundary: HydraCache's authority stays raft + epoch (`R-1`, ADR-0001); the facade
maps supported operations and **fails loud** on unsupported ones.

### D2. Hazelcast client protocol facade (mode 2): **eligible as a bounded edge track**

HydraCache **may** implement a server-side listener that speaks the Hazelcast
Open Binary Client Protocol so an **unmodified** Hazelcast Java client connects
as if to a real cluster. This is the same architectural pattern as the `0.63`
Redis RESP facade: an optional, off-by-default, **single-endpoint, node-local**
edge listener that translates a supported command subset into the existing
client-surface, reuses tenancy/limits/audit, and never touches the core.

It is eligible, not committed: it is a large, multi-release surface and must
carry the same honesty discipline as the RESP facade (node-local flip-sentinels,
loud-unsupported, pinned supported protocol versions, executable conformance).

Feasibility rationale: Hazelcast's **client** protocol is an open, versioned,
generated specification — implementable without reverse-engineering. Cost
drivers that make it larger than RESP: authentication/cluster-version handshake;
a partition table the smart client routes against (trivial for a single member,
but must be served or the client hangs); a **server-to-client event/listener
push channel** (IMap entry listeners, near-cache invalidation) that RESP never
required; hundreds of message types (implement a subset: `IMap`
get/put/CAS/remove/listeners and lock ops); and protocol version drift across
Hazelcast releases (pin supported versions, `0.63` oracle-pinning discipline).

### D3. Hazelcast member co-clustering (mode 3): **will NOT be supported**

HydraCache will **not** implement the Hazelcast internal member protocol and will
**not** join or form a mixed cluster with foreign Hazelcast members. This is a
firm, standing anti-goal, not a deferral.

Rationale:

- The member protocol is Hazelcast's **internal**, version-coupled machinery
  (join/discovery, partition table + migration, CP subsystem Raft on their wire
  format, anti-entropy, heartbeats, split-brain protection + merge). It is not a
  stable public API the way the client protocol is.
- Implementing it would be a **reimplementation of Hazelcast's clustering
  internals in Rust**, bug-for-bug and version-for-version — not an integration
  of two systems, but one system rebuilt inside the other.
- It directly contradicts HydraCache's architecture and positioning: authority is
  **raft + epoch** (`R-1`, ADR-0001, ADR-0003); Hazelcast internals and
  async-replication are explicit **anti-references**. HydraCache has its own
  consensus, ownership, and membership model; a HydraCache "member" would have to
  abandon all of it to speak Hazelcast's.
- The effort is multi-year and permanently version-chasing, against the grain of
  the whole project, for a use case a bridge (D4) serves without the interop.

Any future request to "run HydraCache as a Hazelcast member," "join an existing
Hazelcast cluster," or "mixed Hazelcast/HydraCache cluster" is **out of scope by
this ADR** and must be declined or redirected to D4, unless a future ADR
explicitly supersedes this decision with a recorded rationale.

### D4. Gradual migration bridge (mode 4): **supported approach**

Because HydraCache cannot be a member (D3), incremental migration is served by a
**client-side bridge**: a component connects to the live Hazelcast cluster **as a
client** (preferably by embedding the real Hazelcast Java client library in a JVM
sidecar rather than reimplementing the client protocol), subscribes to map
events, and mirrors data into HydraCache; reads are then cut over
incrementally. This reuses the shipped `0.54` invalidation transports and `0.48`
backup/restore. Client-side **consume** is materially easier than server-side
**serve** (D2), so the bridge does not depend on the client-protocol facade.

## Supported vs rewrite (applies to D1 and D2 surfaces)

The facade covers the Hazelcast data/coordination subset that maps to shipped
HydraCache primitives, and is loud about the rest:

- **Supported / supported-with-caveat:** `IMap` get/put/CAS
  (`replace(k,old,new)`, `remove(k,val)`), entry listeners over the invalidation
  bus, TTL/expiry, single-key **fenced** lock (a genuine HydraCache strength —
  the linearizable fenced-lock engine, `0.46`/`0.52`), near-cache and JCache /
  Hibernate L2 region semantics.
- **Rewrite / unsupported-loud:** full **CP Subsystem** (beyond the single-key
  fenced lock), `EntryProcessor` and distributed compute/executor, SQL / Jet,
  topics / queues / ringbuffer, and any API on `unsupported_hazelcast_apis.txt`.
  These fail loud with a documented divergence; they are never silently emulated.

Node-local honesty (D2): a client-protocol facade, like the RESP facade, is
**single-endpoint and node-local** — no real partitioning, listeners are
listener-local. This must be stated and guarded with flip-sentinels, never
implied to be a distributed Hazelcast cluster.

## The honest gap this ADR records

As of this ADR, the outward Java-facing surface for D1/D2 is largely a
**contract, not a shipped artifact**: `0.52` ships the migration *contract* +
facade *surface* as a **Rust-side mapping**; there is **no Maven/Gradle/Java
module** in the repo, and the Hibernate L2 Java `RegionFactory` is **planned**
(TD-0005). Therefore:

- D1 ("drop-in embedded facade") and any borrowed-Hazelcast-suite conformance
  (e.g. `0.69` W1) are **blocked on shipping the real `0.68` Java artifact** and must not
  claim a runnable Java facade until it exists. Until then, migration-conformance
  work targets the Rust-side contract, or is recorded as blocked.
- The highest-leverage migration enabler is therefore **shipping the Java client +
  facade artifact** (close TD-0005 / the `0.52` Java module), followed by a
  migration-assessment analyzer (scan an app's `com.hazelcast.*` usage against the
  supported/unsupported manifest — reusing the sibling `hazelcast-toolkit`
  `ClassScanner`) and a Spring Boot starter.

## Difficulty summary

| Mode | Surface | Difficulty | Decision |
| --- | --- | --- | --- |
| 1 Embedded non-interop | Drop-in `IMap`/`FencedLock`/`JCache` Java facade | Medium (gated on the Java artifact) | Supported — primary path (D1) |
| 2 Client | Hazelcast Open Binary Client Protocol facade | High but bounded; RESP-facade-shaped, node-local | Eligible bounded edge track (D2) |
| 3 Member | Internal member protocol / mixed co-clustering | Extreme; reimplements Hazelcast internals, against `R-1` | **Not supported (D3)** |
| 4 Bridge | Client-side consume + mirror (sidecar) | Medium | Supported migration approach (D4) |

## Consequence

Migration strategy is segmented and honest: most embedded apps get a drop-in
facade once the Java artifact ships; client apps can be served by a bounded,
node-local, RESP-shaped protocol facade if that track is funded; gradual cutovers
use a client-side bridge; and mixed-cluster co-membership is permanently
declined. The cost of D3's firmness is that HydraCache cannot advertise "drop a
HydraCache node into your Hazelcast cluster" — which is the correct trade, since
attempting it would mean rebuilding Hazelcast inside HydraCache and abandoning the
raft + epoch authority model the rest of the system is built on. Future plans
that touch Hazelcast compatibility must cite this ADR and stay within its
boundaries, or supersede it explicitly.
