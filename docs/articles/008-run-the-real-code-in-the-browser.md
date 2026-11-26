# Run the Real Code in the Browser: Compiling Production Rules to WebAssembly for Honest Simulations

<!-- article-series:start hydracache-runtime -->
## HydraCache Runtime Series

This article is part of a practical series about building a Rust-native local-first cache runtime.

You are reading: Draft.

- [Part 1: Why Rust Needs Cache Semantics, Not Just Another Cache Map](https://medium.com/@artur.buzov/why-rust-needs-cache-semantics-not-just-another-cache-map-ecf3c4e01191)
- [Part 2: Single-flight Is Not an Optimization](https://medium.com/@artur.buzov/single-flight-is-not-an-optimization-85917bdbe77d)
- [Part 3: TTL Is Not Enough](https://medium.com/@artur.buzov/ttl-is-not-enough-ec4e96d89546)
- [Part 4: Local-first Distributed Invalidation](https://medium.com/@artur.buzov/local-first-distributed-invalidation-87bf0249e935)
- [Part 5: Typed Query Caching in Rust](https://medium.com/@artur.buzov/typed-query-caching-in-rust-aac4352599f0)
- [Part 6: How to Measure Cache Performance Without Measuring Noise](https://medium.com/@artur.buzov/how-to-measure-cache-performance-without-measuring-noise-926c10d713f5)
- Draft: Optimization Starts With Measurement: Reading Allocations, RSS, and Runtime Together
- Draft: Run the Real Code in the Browser: Compiling Production Rules to WebAssembly for Honest Simulations

GitHub:

https://github.com/javaquasar/hydracache

crates.io:

https://crates.io/crates/hydracache
<!-- article-series:end -->

Our cluster demo had a nice animation. A client wrote a value, a dot travelled to the node the
client was attached to, hopped to the Raft leader, and fanned out to every replica. Each node
flashed when its copy arrived.

It looked right, and it was wrong.

In HydraCache, Raft is the metadata control plane only. The leader commits who is in the cluster;
it never sees a cache write. And by default, values are not copied to every node at all: the
writing member stores the value locally and the others receive an invalidation. The animation was
a faithful picture of our *simulator*, and the simulator had drifted from the product.

A visualization is a claim. People who watch it learn a model of the system, and they will act on
that model when they design around it. This article describes the approach we use to keep that
claim honest: the browser runs the same Rust code as the product, compiled to WebAssembly. Where it
cannot run the same code yet, the difference is written down, given an id, and owned by a
technical-debt record.

## The setup: one deterministic engine, two targets

The demo is a static page. Everything it shows comes from one crate, `hydracache-sim`, a
deterministic simulator:

- every node is a sans-IO state machine, so time, network, and storage are supplied from outside;
- the network is simulated, with seeded delays, partitions, and drops;
- elections run on real `raft-rs` on native targets, or on a labelled model in the browser;
- the world produces a versioned JSON **snapshot** after every step.

The same crate builds twice:

```text
hydracache-sim  ──native──▶  sandbox server, tests, replay tooling
               └─wasm32───▶  hydracache-sim-wasm ──▶ demo/pkg ──▶ the page
```

Three rules hold the setup together:

1. **The page never invents state.** It renders snapshots. It does not compute a verdict or
   decide who the leader is. If the engine does not report something, the page cannot draw it.
2. **Native and wasm must agree byte for byte.** A test runs the same seed in both builds and
   compares the snapshot JSON. If a rule behaves differently in the browser, this test fails.
3. **The snapshot is a versioned contract.** Every new field bumps the schema, gets an entry in the
   compatibility register, and strict readers reject versions they do not know. A shared demo
   link with a seed can be replayed exactly or refused loudly. It is never "roughly" replayed.

These rules make the demo reproducible. They do not make it truthful. A deterministic simulator can
reproduce a wrong model perfectly.

## Why mirrors drift

The usual shortcut is a **mirror**: a small copy of the product rules, written for the demo,
because the real code does not compile for the browser.

We had one. The product decides whether a write can be acknowledged at a given consistency level
(`One`, `Quorum`, `All`, and so on) in `hydracache::ConsistencyLevel`. That code lived in a module
compiled only for native targets, because the same module also holds the networked grid. So the
simulator carried 69 lines that reimplemented the rule for its own topology, plus a 65-line parity
test that compared the copy with the product for every cluster size from 1 to 7 and every set of
live nodes, including the error text.

The parity test was good engineering, and it still left three problems:

- **Two copies of one rule.** A product change fails the parity test, which is good, but then
  someone has to fix the copy by hand.
- **The parity surface only covers what you enumerated.** The test listed the levels explicitly. A
  new level would not fail it; it would simply be missing from the demo.
- **The mirror models the simulator's topology, not the product's.** It assumed every node is a
  replica in one region. As soon as the demo wants replication factor below cluster size, or
  regions, the mirror has to grow its own placement logic, and the parity test grows with it.

A mirror is a promise to keep two things equal forever. The cheaper fix is to stop having two
things.

## Step 1: make the pure rules build for every target (done)

The rule itself never needed the network. `ConsistencyLevel`, the replica-set types
(`Replicas`, `EffectiveReplicationMap`), `ReplicationConfig`, and the `RegionId` newtype depend
only on a node id type, `serde`, and `std`. They were native-only because of where they lived, not
because of what they do.

So we moved them:

```text
crates/hydracache/src/
  grid/                   # networked grid, native only
    mod.rs                # re-exports the moved types internally
  replication_model/      # NEW: built for every target, including wasm32
    mod.rs                # Replicas, EffectiveReplicationMap, ReplicationConfig, RegionId
    consistency_level.rs  # ConsistencyLevel, AckRequirement, ConsistencyUnsatisfiable
```

Four details made the move safe.

**The public names did not change.** The crate root re-exports the same names it always did, so
native users see no difference. `grid` re-exports the moved types with `pub(crate) use`, which
meant none of the other dozen grid modules needed an edit.

**A golden test went in first.** The next step will move the rendezvous ranking that decides
which member owns a key. Key ownership is a cross-version contract: if two releases in one cluster
disagree on the owner, ownership reshuffles during a rolling upgrade. Before anything moved, we
recorded the current owners and replica sets for a fixed set of keys and members:

```rust
let expected = [
    "user:42|node-1|node-1,node-0,node-3",
    "profiles:p|node-2|node-2,node-3,node-0",
    // ...
];
assert_eq!(placement(), expected);
```

The golden test also caught something we had not planned to look at. The codebase carries two
copies of the same FNV-based rendezvous hash: one in the single-owner resolver and one in the
replication strategy. The golden test pins both, and a second test asserts that their first choice
agrees.

**The simulator calls the product, and an exhaustive `match` replaces the enumerated list.**

```rust
fn consistency_level_name(level: ConsistencyLevel) -> &'static str {
    match level {
        ConsistencyLevel::One => "one",
        ConsistencyLevel::LocalQuorum => "local_quorum",
        ConsistencyLevel::Quorum => "quorum",
        ConsistencyLevel::EachQuorum => "each_quorum",
        ConsistencyLevel::All => "all",
    }
}
```

A new product level now stops the simulator from compiling until someone decides how the demo
shows it. The parity test is no longer needed, because nothing is left to compare.

**The output did not move.** The mirror had been proven equal to the product, so the snapshot JSON
is identical: no schema bump and no compatibility entry. The browser suite (168 Playwright,
visual, and seeded-replay checks) passed against the rebuilt wasm package. The snapshot test now
pins the product's own error text, for example
`requires 3 live acknowledgements from 3 replicas, only 2 are live`.

Net effect: 134 lines of mirror and parity test deleted, one module boundary moved, and the
browser runs the rule that production runs.

### What blocks wasm, and what does not

When a crate "does not build for wasm32", it rarely means the logic is unportable. In our case the
blockers were:

- whole modules gated with `#[cfg(not(target_arch = "wasm32"))]` because *some* of their contents
  use runtime dependencies (Tokio, sockets, the filesystem);
- types that pull in a runtime type through a field. `ClusterMember` is native-only, so a
  function that takes `&[ClusterMember]` is native-only too, even if it only reads the node ids;
- crates with platform features. `getrandom` needs its `js` feature on `wasm32-unknown-unknown`,
  and our Raft crate imports product runtime types, which is why browser elections stay on a model
  (more on that below).

The fix is the same each time. Find the pure core, give it a home with no runtime dependencies,
and leave the runtime parts where they are. Gate modules, not the whole crate. The work is
mechanical; the expensive part is noticing that the boundary was in the wrong place.

## Step 2: enter through the same door as the product (done)

The leader-shaped animation came from one line in the simulator. A manual write went to the
"preferred write node", which was the Raft leader, or, silently, the first live node when there
was no leader. The background workload, meanwhile, wrote to a random live node. Neither matches
the product, where a write is handled by the member the client is connected to, whether or not
there is a leader.

The silent fallback was the worse half. It hid "there is no leader" behind a working write, which
is exactly the kind of quiet degradation our project rules forbid.

Every write now goes through the client's **coordinator**, the member it is connected to:

- the connection is sticky and stored in the world. It moves only when its member crashes, is
  disabled, or is isolated, and the move is traced;
- client admission is a simulated `ClientUpsert` metadata command, which, like the real one, fails
  loud while there is no leader. The page explains this before the user hits the refusal;
- a client that is already admitted keeps writing during an election, because the leader is not
  on the write path.

Two side effects are worth noting. Every client-facing test now starts from a formed cluster,
which is what the product requires anyway. And the push animation lost a hop: the request travels
only along the writer's own link, because that member is where the engine applies the write.

## Step 3: model the protocol with product primitives, behind an explicit mode (done)

The simulator's data plane is a product type, `ClusterNode`, that broadcasts every write to every
peer. It was built to exercise faults and linearizability, not placement. The product has the
placement and acknowledgement rules as tested primitives; what it does not yet have is a networked
path that wires them into a write. That gap matters for honesty, so it shapes the design:

- a replication model, chosen by a replayable control action, selects `full_broadcast` (the old
  model), `local_first` (write on the client's member and invalidate the other copies), or
  `replicated { rf, level }`. In `replicated`, the coordinator forwards to the key's primary, which
  replicates to its backups, and the product `Ack` messages are counted against
  `ConsistencyLevel::validate`;
- the nodes and their messages stay product code (`ClusterNode`, `ReplicatePut`, `Ack`). The
  simulator only decides where each message goes, so no second data plane was written;
- placement uses the product's rendezvous ranking. It had to become wasm-buildable first, without
  changing a single owner; the golden test from step 1 passed untouched;
- a write the live owners cannot meet is refused loud with the product's own reason, for example
  `write refused at quorum: requires 2 live acknowledgements from 3 replicas, only 1 are live`. A
  pending write fails loud as soon as that becomes true;
- a down primary is replaced by the next live owner. This is a simplified form of the product's
  backup promotion, and the gap register says so;
- the old broadcast model stays the default, and the built-in workload refuses to run under the
  new ones. Existing deterministic-simulation tests and invariants were written against it, and
  replacing it silently would change what those tests mean;
- `replicated` is labelled in the demo as **the model of the opt-in replication primitives**, not
  as shipped runtime behaviour.

Step 3b then added what a lost copy needs:

- a primary that cannot reach a backup keeps a **hint** and replays it when the backup returns.
  Hints are lost with a crashed holder, as with the product's in-memory store;
- copies apply **by version**: a late hint or an older invalidation is acknowledged but never
  overwrites a newer value. The first test of this caught a real race, a hint and a newer write
  in flight together;
- an **anti-entropy pass** repairs copies lost in flight.

These three rules are modeled in the simulator, not compiled from the product. The product's
hint store and version merge depend on value-record types that do not build for the browser yet.
That is recorded as a gap of its own, so the next step is obvious: move those types the way step 1
moved the consistency levels.

That last point is the general lesson. When a simulator is ahead of the product, it is a design
tool, which is fine, but the page has to say so.

## Step 4: emit events where they actually happen (planned)

Today a subscriber in the simulator receives an event only after *every* live node holds the new
version. That is an all-replicas convergence rule, and no real subscriber works that way. In the
product, a subscriber hears from the cache event or invalidation stream of the node it is attached
to. The fix is small: deliver an event when the subscriber's own node applies the write or
receives the invalidation.

This step shows why the demo cannot be fixed alone. The demo already times a subscriber's flash to
the engine's recorded delivery step, so it is honest about *when*. The engine was wrong about
*why*.

## Step 5: draw only what the engine recorded, on separate lanes (partly done)

The rendering rule is simple: **every visual effect is tied to a fact in the snapshot.** Over the
last schema versions we added the facts the animation needed:

- `applied_writes` per node, so a lagging node's progress bar measures its own applied writes and
  not a cluster-wide counter;
- per-link drop counters by cause (`cut_at_send`, `injected`, `cut_at_delivery`, `target_down`),
  so a cut link visibly refuses traffic instead of looking idle;
- `applied_at_step` per replica, so a node flashes at the step the engine applied the value. A copy
  still in flight stops short, and a copy that will never arrive fades with a loss marker.

The remaining work is to separate the planes visually. The client, the coordinator, the owners, and
the acknowledgements go on the data lane. The Raft leader goes on its own metadata lane and
lights up only for admission, leave, topology, and voter changes, because those are the only things
it does.

## Honesty is a mechanism, not a tone

Wording on a page is not enough. These are the mechanisms that keep the demo from overclaiming:

**A source label with a disclosure.** Browser elections run on a deterministic model, because the
Raft crate cannot build for wasm today. The snapshot carries `election_source: "sim-model"` or
`"raft"` with a disclosure string, and a unit guard asserts that the wasm build reports the model
and never claims product consensus.

**One source of truth for the real behaviour.** We keep a single architecture document for the
Raft control plane and the write path. It maps each concern to the code symbol that implements it,
states what stops and what keeps working without a leader, and says plainly which primitives are
not yet wired into a runtime path. If the document and the code disagree, the document is the bug.

**A gap register.** Every place where the simulator or the page differs from the product gets a
row with an id (G1 to G8), the current behaviour, the product behaviour, and the debt record that
owns it. Closing a gap means changing the engine first, then the demo, then the table. The mirror
from step 1 was G8, and it is now closed.

**Debt records with a definition of done.** Each gap belongs to a technical-debt entry that says
why the debt exists, what risk it carries, what triggers a revisit, and the boolean conditions for
removing it: tests that must exist, snapshots that must still match, gates that must pass.

## Pitfalls we hit

- **Hash stability is a hidden compatibility contract.** Moving placement code is a refactor until
  a byte changes. Then it becomes an ownership reshuffle across versions. Golden-test placement
  before touching it.
- **"Unused" warnings appear only in wasm.** Native-only callers disappear in the browser build,
  so items look dead there. Read them as a map of what the browser cannot reach yet, not as noise
  to silence.
- **Visual tests need deterministic input, not just a fixed seed.** One flaky check fired an effect
  only if a frame happened to land inside a narrow slice of an animation cycle. The fix was in the
  renderer: an arrival missed in one cycle fires on the next frame. The test was not loosened.
- **Mirrors pass their own tests.** Ours was correct and fully parity-tested, and it would still
  have grown into a second implementation as soon as the demo needed replica sets. The trigger to
  remove a mirror is not a bug in it. It is the first feature that would make it grow.

## A checklist for honest simulations

1. Does the page compute anything the engine did not report? It should not.
2. Do the native and browser builds produce identical snapshots for the same seed?
3. Is every snapshot change versioned, registered, and rejected by strict readers when unknown?
4. For every rule the page shows, is it the product's code or a copy? If it is a copy, which debt
   record owns it, and what feature would make it grow?
5. Does the simulator enter through the same door as the product, and does it fail where the
   product fails?
6. When the simulator models something the product does not ship yet, does the page say so?
7. Is there one document that states the real behaviour, with a gap register that lists every
   difference?

## The lesson

A simulator earns trust the same way production code does: by running the real rules, failing
where they fail, and saying what it does not cover.

WebAssembly makes the first part cheap, often cheaper than maintaining a mirror. Most of the work
is moving the boundary between pure logic and runtime to where it should have been. The rest is
discipline: record every place where the picture and the system still differ, and close those gaps
in the engine before you redraw them on the page.
