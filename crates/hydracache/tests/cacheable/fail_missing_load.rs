use hydracache::{cacheable_loader, HydraCache};

fn main() {
    let cache = HydraCache::local().build();
    let _future = cacheable_loader!(
        cache = cache,
        key = "value:1",
    );
}
