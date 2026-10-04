# W3 adaptive coalescing baseline

Status: preliminary local smoke, non-promotable. These are one independently started sample per
cell, captured before the adaptive candidate. They establish the mechanical baseline only; no
numeric improvement claim is allowed until the preregistered five counterbalanced pairs pass.

- Product source: `461c6c298419251dd208bc2949ae92fc594a9cf1`
- Measurement source: `7c5e0d09598c6ede551c3dc00f1a6bbced63c5f1`
- Transport: loopback TCP
- Per cell: 160,000 measured operations after 16,000 warmup operations
- Payload/key-space/seed: 64 bytes / 262,144 / 740074
- Instrumentation: enabled
- Raw receipts: `local-runs/w3-adaptive-baseline/`

Every cell recorded exactly 160,000 response frames, 160,000 server write calls, 160,000 explicit
flush calls and 160,000 TCP `poll_write` attempts. The adaptive candidate therefore has a direct
mechanical owner to remove without inferring kernel syscall counts.

| Operation | Pipeline | Concurrency | Goodput op/s | CPU ns/op | p99 us |
| --- | ---: | ---: | ---: | ---: | ---: |
| GET | 1 | 1 | 11,930.2 | 72,558.6 | 405 |
| GET | 1 | 8 | 28,903.5 | 69,238.3 | 1,826 |
| GET | 1 | 32 | 30,574.4 | 76,367.2 | 3,299 |
| GET | 10 | 1 | 45,931.8 | 20,214.8 | 773 |
| GET | 10 | 8 | 124,479.1 | 47,363.3 | 1,063 |
| GET | 10 | 32 | 120,305.7 | 50,683.6 | 4,503 |
| GET | 50 | 1 | 63,490.2 | 15,039.1 | 2,529 |
| GET | 50 | 8 | 127,816.6 | 44,335.9 | 4,491 |
| GET | 50 | 32 | 125,212.2 | 43,945.3 | 21,983 |
| SET | 1 | 1 | 21,721.1 | 40,722.7 | 239 |
| SET | 1 | 8 | 37,403.1 | 77,929.7 | 885 |
| SET | 1 | 32 | 59,063.5 | 72,949.2 | 1,000 |
| SET | 10 | 1 | 85,781.0 | 11,523.4 | 251 |
| SET | 10 | 8 | 122,981.7 | 50,488.3 | 1,644 |
| SET | 10 | 32 | 120,792.1 | 52,636.7 | 4,635 |
| SET | 50 | 1 | 93,173.5 | 10,449.2 | 1,016 |
| SET | 50 | 8 | 122,478.4 | 52,636.7 | 7,243 |
| SET | 50 | 32 | 128,542.3 | 49,316.4 | 21,615 |

The high-concurrency pipeline-50 p99 values are retained rather than filtered. They reinforce why
the candidate must preserve the frozen tail and favorable-pair guards instead of selecting only
the highest-throughput cells.
