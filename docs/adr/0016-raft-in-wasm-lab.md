# ADR-0016: Raft Election in the Wasm Lab

## Status

Accepted for 0.53.1.

## Context

0.53.1 adds a native lab election backend that drives real `raft-rs` `RawNode`
instances synchronously over the deterministic simulator network. The browser demo
also has a `wasm32-unknown-unknown` engine, so the release must decide whether that
engine can use the same raft backend or must stay on the labelled simulator model.

## Spike Result

The first wasm attempt with the raft backend reachable failed:

```powershell
cargo build -p hydracache-sim --target wasm32-unknown-unknown --locked
```

The initial blocker was `getrandom` requiring its `js` feature for
`wasm32-unknown-unknown`. After testing that path, the material blocker was
`hydracache-cluster-raft`: it imports product-runtime types from `hydracache` that
are intentionally `cfg(not(target_arch = "wasm32"))`, including `CacheError`,
`CacheResult`, `ClusterControlPlane`, `ClusterEpoch`, and membership/runtime types.

The native raft harness remains valid because it uses only the low-level
`RawNode`, `InMemoryRaftLogStore`, and `RaftWireMessage` path on native targets. The
wasm package builds when that native-only harness is compiled out:

```powershell
cargo build -p hydracache-sim --target wasm32-unknown-unknown --locked
wasm-pack build crates/hydracache-sim-wasm --target web --out-dir ../../demo/pkg --release -- --locked
```

## Decision

The server/sandbox engine is the high-fidelity election mode and defaults to
`election_source: "raft"`.

The wasm engine remains on the validated simulator model and reports
`election_source: "sim-model"`. It must not silently claim raft fidelity.

## Guard

`hydracache-sim-wasm` has a unit guard,
`wasm_default_reports_validated_sim_model`, asserting the wasm handle reports
`sim-model` with the non-product-claim disclosure. The release gate also runs the
two wasm build commands above.

## Consequences

Users who need real raft election fidelity should run the demo with the server
engine. The default browser-only wasm path remains deterministic and validated
against raft by the model-vs-raft tests, but it is explicitly labelled as a model.
