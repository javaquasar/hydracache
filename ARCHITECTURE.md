# Architecture

HydraCache is a local-first cache runtime designed to evolve in layers:

1. Local cache core
2. Cluster fabric and distributed invalidation
3. Named collections and observable runtime objects
4. Database adapters
5. Macro ergonomics
6. Future stateful and streaming-inspired runtime behavior

## Core Principles

- Local-first fast path
- Explicit invalidation semantics
- Typed developer-facing API
- Distributed invalidation before distributed storage
- Neutral integration contracts before library-specific coupling

## Initial Crates

- `hydracache-core`
- `hydracache`
- `hydracache-macros`
- `hydracache-sqlx`
