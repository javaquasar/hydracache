# Redis Edge Corpus For 0.64 Test Expansion

This document records the W28 mined Redis edge rows that are replayed by
`crates/hydracache-redis-compat/tests/redis_mined_edge_corpus.rs`.

The corpus is intentionally small and committed. It mines Redis' own Tcl test suite for edge shapes
inside HydraCache's supported RESP subset, then encodes the expected RESP bytes as a fast deterministic
oracle. Rows that require a live Redis instance, unsupported data structures, scripting, transactions,
or Redis server state stay out of the fast claim and belong in the existing Docker-gated oracle lane.

| Label | Redis source | HydraCache contract |
| --- | --- | --- |
| `string-get-missing-is-null` | `redis/tests/unit/type/string.tcl` nil-shape string rows | `GET` on a missing key returns RESP Null Bulk (`$-1`). |
| `string-mget-mixes-value-and-null` | `redis/tests/unit/type/string.tcl`, `MGET against non existing key` | `MGET` preserves positional value/null shape. |
| `string-mset-duplicate-key-last-write-wins` | `redis/tests/unit/type/string.tcl`, duplicate-key `MSET` row | Duplicate keys inside one `MSET` batch are applied in command order, so the last value is visible. |
| `string-mset-wrong-arity-fails-loud` | `redis/tests/unit/type/string.tcl`, `MSET/MSETNX wrong number of args` | Odd key/value arity fails loud with a Redis-shaped wrong-arity error and no mutation. |
| `expire-set-invalid-px-zero-fails-loud` | `redis/tests/unit/expire.tcl`, invalid expire time rows | `SET ... PX 0` fails loud as an invalid expire time. |

The W28 scope is a mined compatibility corpus, not a claim that the full Redis suite runs against the
facade. When a row is promoted from this file to a live Redis oracle, keep the pinned Redis image and
client/library version in the release proof artifacts.
