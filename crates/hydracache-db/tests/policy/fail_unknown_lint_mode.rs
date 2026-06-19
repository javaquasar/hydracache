use hydracache_db::query_cache_policy;

fn main() {
    let _policy = query_cache_policy!(
        key_segments = ["tenant", 7_u64],
        sql = "select * from users",
        dependency_lint = strict,
    );
}
