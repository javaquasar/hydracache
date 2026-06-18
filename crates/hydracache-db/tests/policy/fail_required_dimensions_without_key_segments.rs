use hydracache_db::query_cache_policy;

fn main() {
    let _policy = query_cache_policy!(
        key = "users",
        required_dimensions = ["tenant"],
    );
}
