# Cacheable Functions

Use cacheable function helpers when ordinary async work needs the same cache boundary with less boilerplate.

The macros are intentionally explicit. They do not discover a global cache, generate hidden keys from every function argument, or hide the loader. They build `CacheOptions` and call the same runtime methods you could call manually.

## Loader Macro

`cacheable_loader!` is the compact form for fallible async loaders.

```rust
{{#include ../../examples/src/bin/cacheable_functions.rs:cacheable-loader}}
```

Use this when you already have a cache instance and want the call site to show key, tags, TTL, and loader in one expression.

## Infallible Loader

Use `cacheable_infallible!` when the loader cannot fail and `Ok::<_, Error>(value)` would only add ceremony.

```rust
{{#include ../../examples/src/bin/cacheable_functions.rs:cacheable-infallible}}
```

The cache operation can still fail because serialization, storage, or runtime boundaries can fail. The macro only removes the loader error wrapper.

## Attribute Macro

Use `#[cacheable]` when the cached operation is naturally an async function.

```rust
{{#include ../../examples/src/bin/cacheable_functions.rs:cacheable-attribute}}
```

The cache remains an explicit function argument. The generated wrapper returns `hydracache::CacheResult<T>` because cache errors can occur outside the loader.

## Choosing A Form

Use the explicit local cache API first when designing a new cached operation. Move to a macro when the key, tags, TTL, and loader boundary are already obvious.

Prefer:

- `cacheable_loader!` for one-off fallible loaders;
- `cacheable_infallible!` for one-off loaders that cannot fail;
- `#[cacheable]` for reusable async functions with stable key/tag metadata.
