# Local Cache

Use the local cache when the current process can safely reuse a value without contacting a backing source.

```rust
{{#include ../../examples/src/bin/local_cache.rs:local-cache}}
```

This guide intentionally keeps the example small:

- `put` stores a typed value;
- `get` reads the same typed value;
- `get_or_load` avoids repeated loader calls;
- tag invalidation removes related entries after writes.

Production code should give keys and tags names that match the domain model. A key identifies one cached value. A tag identifies a group of values that a write can make stale.
