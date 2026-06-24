# HydraCache Cluster Simulator Demo

This directory contains the static browser demo for the real deterministic
`hydracache-sim` engine. The UI does not script fake green animations: every
visible state is serialized from a real `SimWorld` snapshot, and every verdict is
produced by the actual invariant checker.

The published DevRel build is intended for explanation and reproduction, not as
the release correctness gate. Correctness remains covered by the simulator test
suite and `cargo xtask verify`.

## Local Build

```powershell
rustup target add wasm32-unknown-unknown
cargo build -p hydracache-sim-wasm --target wasm32-unknown-unknown --locked
wasm-pack build crates/hydracache-sim-wasm --target web --out-dir ../../demo/pkg --release -- --locked
python -m http.server 5173
```

Then open:

```text
http://127.0.0.1:5173/demo/
```

The default engine is the local WASM package. To drive the same UI through the
sandbox HTTP API instead, run the sandbox and add `engine=server` plus the API
base:

```powershell
cargo run -p hydracache-sandbox -- --backend memory
```

```text
http://127.0.0.1:5173/demo/?engine=server&api=http://127.0.0.1:3000
```

The page URL carries `seed`, `steps`, and `scenario`, so a shared URL can be
replayed locally. The copy-reproducer button emits the matching `hydracache-sim`
CLI command.

## DevRel CI

`.github/workflows/demo.yml` runs outside the fast PR gate. It builds the WASM
package, runs the headless UI and seed-share smoke tests, and publishes the
static `demo/` artifact to GitHub Pages on nightly/manual/tag runs.
