# Typed Cache

Use a typed cache view when several call sites share one value type and one domain namespace.

```rust
{{#include ../../examples/src/bin/typed_cache.rs:typed-cache}}
```

The typed view does not create separate storage. It is a namespaced view over the same `HydraCache` runtime, so stats, events, invalidation safety, and single-flight behavior remain shared.

The namespace prefixes keys. In the example, the typed key `42` is stored as `profiles:42` in the underlying cache. Use namespaces that match domain boundaries rather than module names.

Typed views are most useful when they remove repeated annotations without hiding cache semantics. Keep keys, tags, and TTLs visible at the call site.
