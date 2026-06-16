use hydracache_db::prepared_query_policy;

fn main() {
    let _policy = prepared_query_policy!(
        key_segments = ["tenant", 7_u64],
        tag_segments = [],
        ttl_secs = 30,
    );
}
