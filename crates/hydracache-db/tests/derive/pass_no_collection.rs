use hydracache_db::{CacheEntity, HydraCacheEntity};

#[derive(HydraCacheEntity)]
#[hydracache(entity = "wrapper", id = String)]
struct Wrapper<T>
where
    T: Clone,
{
    value: T,
}

fn main() {
    assert_eq!(Wrapper::<u8>::cache_key_for(&"abc".to_owned()), "wrapper:abc");
    assert_eq!(Wrapper::<u8>::collection_tag(), None);
}
