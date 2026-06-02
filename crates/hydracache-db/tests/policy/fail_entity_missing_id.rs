use hydracache_db::query_cache_policy;

struct User;

fn main() {
    let _policy = query_cache_policy!(entity = User);
}
