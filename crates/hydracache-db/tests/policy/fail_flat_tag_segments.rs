use hydracache_db::query_cache_policy;

fn main() {
    let tenant_id = 7_u64;
    let _policy = query_cache_policy!(
        key = "users",
        tag_segments = ["tenant", tenant_id],
    );
}
