use hydracache::{cacheable_loader, HydraCache};

fn main() {
    let cache = HydraCache::local().build();
    let _future = cacheable_loader!(
        cache = cache,
        load = || async { Ok::<_, std::io::Error>(1_u64) },
    );
}
