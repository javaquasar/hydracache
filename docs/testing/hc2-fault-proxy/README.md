# H19 retained fault replay

This directory retains privacy-safe deterministic evidence for the HC/2 H19
fault proxy. It is separate from compatibility, performance, and release
evidence.

| Artifact | Seed | Direction | SHA-256 | Size |
| --- | ---: | --- | --- | ---: |
| `h19-seed-1592590353.json` | `1592590353` | server to client | `4b4aedc065d738683fa9609ef4cd1c9fce28101ec46fd2a5cb4e9f179f90ff88` | 5,784 bytes |

The case applies seeded fragmentation, bounded coalescing, delay, half-open,
late delivery, and bandwidth/window pressure to four deterministic synthetic
chunks. The artifact retains only input shape, action trace, delivery sizes,
logical ticks, terminal state, and SHA-256 hashes. It contains no raw payload.

Verify the checked-in artifact:

```text
cargo xtask client-plane-fault-check
cargo xtask client-plane-fault-check --replay docs/testing/hc2-fault-proxy/h19-seed-1592590353.json
```

Generate the exact case outside the tracked evidence directory:

```text
cargo xtask client-plane-fault-check --seed 1592590353 --output target/hc2-fault-traces/replay.json
```

Do not overwrite a retained artifact merely because a scheduler change creates
a different trace. Treat that as a reviewable contract change: explain it,
retain the old artifact when a supported line still needs it, and update this
manifest only in the same reviewed commit.
