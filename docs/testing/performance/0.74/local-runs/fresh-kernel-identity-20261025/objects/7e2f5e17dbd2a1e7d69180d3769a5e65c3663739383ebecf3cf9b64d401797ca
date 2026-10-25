# Fresh kernel identity CPU screens

These bounded nonproduct screens compare fresh device/inode observations and
the supervisor's actual `Origin::revalidate`. They do not benchmark cache API
throughput, grant original-start authority or replace the full preparation gate.
No result authorizes fewer observations, reordered conditional reads or new
time origins. See the [result and preserved failures](../../docs/testing/performance/0.74/fresh-kernel-identity-result-20261025.md).

`protocol.json` fixes the standalone metadata screen before runtime integration.
`screen.rs` uses the exact existing Rust 1.94 GNU compiler and pinned libc rlib;
the evidence retains its complete build command, hashes and eight first process
transcripts. It is intentionally not another Cargo workspace package or a new
dependency. Its three arms are original `File::metadata`, the identical original
control, and a fresh `libc::fstat` prototype. Each process uses twelve balanced
samples of 8192 iterations over seven retained files.

`installed-origin-protocol.json` separately fixes eight processes with twelve
balanced samples of 1024 complete origin validations per arm. Each arm performs
the original named/held procfs, Boot and Namespace checks in the same order.
The installed arm calls the actual runtime method, not another copy of the
prototype. The manual registered gate selects one cohort; the retained one-shot
server coordinator owns the finite eight-process schedule. Do not silently
restart a failed cohort or convert one successful cohort into the whole verdict.

On a reviewed Linux x86_64 environment, an explicitly selected cohort uses:

```sh
HC074_FD_IDENTITY_COHORT=0 cargo +1.94.0 test --release --offline --locked \
  -p hydracache-long-run-supervisor-074 --lib \
  fresh_fd_identity_original_origin_cpu_screen -- --ignored --nocapture --test-threads=1
```

Do not run these timing screens beside compilation, signer fixtures or another
measurement. The frozen server protocol uses UID/GID 1000 and process-local
CPU affinity 5; it does not alter host CPU isolation, services or clocks.
Preserve raw stdout/stderr, selector, source/image hashes, child PID/start ticks
and endpoint host frames before reviewing results. `review.py` is pure offline
code; it rejects incomplete, reordered, unequal-work or invalid-clock input and
keeps leaf, installed-origin, complete preparation and release verdicts separate.
Its seven model controls run with `python -m unittest discover -s tools/kernel-fresh-identity-074`.
