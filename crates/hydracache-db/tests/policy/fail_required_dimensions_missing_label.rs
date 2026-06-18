use hydracache_db::query_cache_policy;

fn main() {
    let tenant_id = 7_u64;
    let page = 1_u32;

    let _policy = query_cache_policy!(
        key_segments = ["tenant", tenant_id, "page", page],
        required_dimensions = ["tenant", "permission"],
    );
}
