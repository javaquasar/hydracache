# H19/H20 retained fault replay

This directory retains privacy-safe deterministic evidence for the HC/2 H19
fault proxy. It is separate from compatibility, performance, and release
evidence.

| Artifact | Seed | Direction | SHA-256 | Size |
| --- | ---: | --- | --- | ---: |
| `h19-seed-1592590353.json` | `1592590353` | server to client | `4b4aedc065d738683fa9609ef4cd1c9fce28101ec46fd2a5cb4e9f179f90ff88` | 5,784 bytes |
| `h20-client-half-close-seed-1592590354.json` | `1592590354` | client to server | `aa9ece55669d8d1eb8a4f783ad04c3711d8c04d548f5f9365e90e374f5e89893` | 9,232 bytes |
| `h20-uncooperative-timeout-seed-1592590355.json` | `1592590355` | server to client | `7fa7448f22c60818b1d224ec64adffe2c814574cfcb2c5bf8929cd1eac4d36a7` | 1,563 bytes |
| `h20-peer-reset-seed-1592590356.json` | `1592590356` | client to server | `100d9ffdb68c66c0ca4fcc0ac7878c1c5dbd777e21b189085a34f0ab26c30ac7` | 1,542 bytes |

The H19 case applies seeded fragmentation, bounded coalescing, delay, half-open,
late delivery, and bandwidth/window pressure to four deterministic synthetic
chunks. The artifact retains only input shape, action trace, delivery sizes,
logical ticks, terminal state, and SHA-256 hashes. It contains no raw payload.
The three H20 cases bind client half-close, uncooperative blocked-direction
timeout, and peer reset to the bounded HTTP/2 drain controller.

Verify the checked-in artifact:

```text
cargo xtask client-plane-fault-check
cargo xtask client-plane-fault-check --replay docs/testing/hc2-fault-proxy/h19-seed-1592590353.json
```

Generate the exact case outside the tracked evidence directory:

```text
cargo xtask client-plane-fault-check --seed 1592590353 --output target/hc2-fault-traces/replay.json
cargo xtask client-plane-fault-check --case h20-peer-reset --seed 1592590356 --output target/hc2-fault-traces/h20-reset.json
```

Do not overwrite a retained artifact merely because a scheduler change creates
a different trace. Treat that as a reviewable contract change: explain it,
retain the old artifact when a supported line still needs it, and update this
manifest only in the same reviewed commit.
