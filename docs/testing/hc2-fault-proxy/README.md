# H03/H11/H19/H20 retained fault replay

This directory retains privacy-safe deterministic evidence for the HC/2 H19
fault proxy. It is separate from compatibility, performance, and release
evidence.

| Artifact | Seed | Direction | SHA-256 | Size |
| --- | ---: | --- | --- | ---: |
| `h19-seed-1592590353.json` | `1592590353` | server to client | `4b4aedc065d738683fa9609ef4cd1c9fce28101ec46fd2a5cb4e9f179f90ff88` | 5,784 bytes |
| `h20-client-half-close-seed-1592590354.json` | `1592590354` | client to server | `aa9ece55669d8d1eb8a4f783ad04c3711d8c04d548f5f9365e90e374f5e89893` | 9,232 bytes |
| `h20-uncooperative-timeout-seed-1592590355.json` | `1592590355` | server to client | `7fa7448f22c60818b1d224ec64adffe2c814574cfcb2c5bf8929cd1eac4d36a7` | 1,563 bytes |
| `h20-peer-reset-seed-1592590356.json` | `1592590356` | client to server | `100d9ffdb68c66c0ca4fcc0ac7878c1c5dbd777e21b189085a34f0ab26c30ac7` | 1,542 bytes |
| `h11-handshake-reset-seed-1592590357.json` | `1592590357` | server to client | `e3936f68529de62aed36ca8a863dd931528657850f244a7c7439c0d7d1ded422` | 1,552 bytes |
| `h11-server-restart-seed-1592590358.json` | `1592590358` | server to client | `8ed83678a6da8992c34aa1149209d8849c9e166c24dd26a4a75a420ec6ad8c66` | 1,600 bytes |
| `h11-leader-hint-reset-seed-1592590359.json` | `1592590359` | server to client | `a140e7086281b18ac4c711c1457cae09c9335d8d7fe62e19f22b580d20d320ca` | 2,117 bytes |
| `h11-subscription-gap-seed-1592590360.json` | `1592590360` | server to client | `7e215b0f1763147e468e7585c210ea52d814720641b961ac2075ecc1556ee8b9` | 5,998 bytes |
| `h11-invocation-reset-seed-1592590361.json` | `1592590361` | server to client | `3a735ef655f46d09d19edc648d61c574aca31a34d27d3de85a1593d5691dd26f` | 1,526 bytes |
| `h11-session-loss-seed-1592590362.json` | `1592590362` | client to server | `951f16cf73b68419ac43457156109df23c3810ca7561a2a7ca9d989b555767fb` | 2,215 bytes |
| `h11-duplicate-event-seed-1592590363.json` | `1592590363` | server to client | `72e7bb74011370cc80f08838e1cce0dab8daf89f16264466802f1cdfdde2e43d` | 2,538 bytes |
| `h11-reconnect-exhausted-seed-1592590364.json` | `1592590364` | server to client | `879b791efbecd0e40e91b2515685c0bef9cfaf12c8ef46f2096f8c66442d9174` | 1,208 bytes |
| `h03-candidate-preservation-seed-1592590365.json` | `1592590365` | client to server | `d019e1a6a400b785f39e7defb7f442d0cef493f155ecd03df1cc27e124323d58` | 7,502 bytes |
| `h03-candidate-close-seed-1592590366.json` | `1592590366` | client to server | `c8a1920835018ab971b49762e062909921a4162b1699b879f9d4dde8bae8538f` | 1,434 bytes |
| `h03-candidate-duplicate-seed-1592590367.json` | `1592590367` | client to server | `751f4e2c1413e6cc939a86a92ca199e24a268b5f62d1532617789069ed54e10c` | 2,546 bytes |
| `h03-candidate-reset-seed-1592590368.json` | `1592590368` | client to server | `c39ba4810e2ab528075e083ef9312eab38878965181d3df598bace90e6d69608` | 1,176 bytes |

The H19 case applies seeded fragmentation, bounded coalescing, delay, half-open,
late delivery, and bandwidth/window pressure to four deterministic synthetic
chunks. The artifact retains only input shape, action trace, delivery sizes,
logical ticks, terminal state, and SHA-256 hashes. It contains no raw payload.
The three H20 cases bind client half-close, uncooperative blocked-direction
timeout, and peer reset to the bounded HTTP/2 drain controller.
The eight H11 cases bind initial-handshake fallback, server restart,
leader-hint endpoint selection, subscription gaps, safe invocation replay,
permanent fenced-session loss, duplicate-event suppression, and bounded
reconnect exhaustion to the recovery contract. The retained traces are
validated by both `xtask` and the recovering-client integration suite.
The four H03 cases are applied without modification to the encoded semantic
frame of every transport candidate. Fragment/coalesce/delay/window pressure
must preserve decoding, while close, duplicate, and reset must fail closed.

Verify the checked-in artifact:

```text
cargo xtask client-plane-fault-check
cargo xtask client-plane-fault-check --replay docs/testing/hc2-fault-proxy/h19-seed-1592590353.json
cargo test -p hydracache-client-hc2 --test reconnect_repair --locked
cargo test -p hydracache-client-plane-spike --test fault_proxy --locked
```

Generate the exact case outside the tracked evidence directory:

```text
cargo xtask client-plane-fault-check --seed 1592590353 --output target/hc2-fault-traces/replay.json
cargo xtask client-plane-fault-check --case h20-peer-reset --seed 1592590356 --output target/hc2-fault-traces/h20-reset.json
cargo xtask client-plane-fault-check --case h11-subscription-gap --seed 1592590360 --output target/hc2-fault-traces/h11-gap.json
cargo xtask client-plane-fault-check --case h03-candidate-reset --seed 1592590368 --output target/hc2-fault-traces/h03-reset.json
```

Do not overwrite a retained artifact merely because a scheduler change creates
a different trace. Treat that as a reviewable contract change: explain it,
retain the old artifact when a supported line still needs it, and update this
manifest only in the same reviewed commit.
