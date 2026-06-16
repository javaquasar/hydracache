use hydracache_db::prepared_query_policy;

struct User;

fn main() {
    let _policy = prepared_query_policy!(
        per_entity = User,
        key = "user:42",
    );
}
