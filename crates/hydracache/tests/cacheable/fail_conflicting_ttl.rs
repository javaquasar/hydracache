use hydracache::{cacheable_loader, HydraCache};

// Keep the macro span on double-digit line numbers. Rustc versions can format
// single-digit line-number padding differently, which makes trybuild snapshots
// fragile across the stable and MSRV toolchains.
//
// The test itself verifies only the conflicting TTL diagnostic below.
fn main() {
    let cache = HydraCache::local().build();
    let _future = cacheable_loader!(
        cache = cache,
        key = "value:1",
        ttl = std::time::Duration::from_secs(60),
        ttl_secs = 60,
        load = || async { Ok::<_, std::io::Error>(1_u64) },
    );
}
