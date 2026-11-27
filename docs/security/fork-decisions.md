# Maintenance fork decisions

These forks carry the 0.74.1 maintenance dependency graph at Rust 1.88. For
each upstream finding, record the advisory ID, fork package and exact
revision, affected code and test or reason for non-applicability.

| Fork packages | Upstream | Fork source | Exit condition |
| --- | --- | --- | --- |
| `hydra-raft`, `hydra-raft-proto` | TiKV raft-rs 0.7.0 | [javaquasar/raft-rs](https://github.com/javaquasar/raft-rs/tree/hydra/0.7.0-hydra.1) `hydra/0.7.0-hydra.1` | Switch to an upstream release with the Prost codec fixes and a clean RustSec graph on the supported Rust version. Recheck Raft wire and on-disk compatibility. |
| `hydra-sled` | sled 0.34.7 | [javaquasar/sled](https://github.com/javaquasar/sled/tree/hydra/0.34.7-hydra.1) `hydra/0.34.7-hydra.1` | Retire after an approved storage migration replaces sled and preserves reopen and rollback behavior. |
| `hydra-sqlx`, `hydra-sqlx-sqlite`, `hydra-sqlx-macros`, `hydra-sqlx-macros-core` | SQLx 0.8.6 | [javaquasar/sqlx](https://github.com/javaquasar/sqlx/tree/hydra/0.8.6-hydra.1) `hydra/0.8.6-hydra.1` | Switch to upstream SQLx when the supported compiler can build a fixed driver and macro graph that resolves one compatible SQLite binding with Diesel. Recheck SQLite and PostgreSQL consumers. |
