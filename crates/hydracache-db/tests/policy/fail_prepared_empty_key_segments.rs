use hydracache_db::prepared_query_policy;

fn main() {
    let _policy = prepared_query_policy!(
        key_segments = [],
        ttl_secs = 30,
    );
}
