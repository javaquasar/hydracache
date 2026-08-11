# Getting Started

The quickest way to understand HydraCache is to build one local cache flow:

1. create a cache;
2. store a typed value with tags and TTL;
3. load a missing value through `get_or_load`;
4. invalidate the related tag after a write.

For a published crate, add HydraCache to an application:

```powershell
cargo add hydracache
```

Inside this repository, the documentation examples use path dependencies so they are checked against the current branch.

```rust
{{#include ../examples/src/bin/quick_start.rs:quick-start}}
```

The important parts are:

- values are typed at the cache boundary;
- `CacheOptions` carries TTL and invalidation metadata;
- the loader runs only when the value is missing or expired;
- tag invalidation removes values related to a write;
- the example is compiled by the documentation examples crate.

Run the checked example from the repository root:

```powershell
cargo run --manifest-path docs-site/examples/Cargo.toml --bin quick_start
```

Next, read [Keys and Tags](concepts/keys-and-tags.md). Most HydraCache usage becomes straightforward once key identity and invalidation tags are clear.
