use hydracache::HydraCache;
use hydracache_seaorm::{SeaOrmCache, SeaOrmQueryExt};

#[tokio::main]
async fn main() -> hydracache_seaorm::Result<()> {
    // ANCHOR: seaorm-adapter
    let queries = SeaOrmCache::new(HydraCache::local().build(), "seaorm");

    let user_name = queries
        .entity::<String>("user", 42)
        .collection_tag("users")
        .sea_one(|| async {
            // Replace this with a SeaORM query or repository method.
            Ok::<_, hydracache_seaorm::sea_orm::DbErr>("Ada".to_owned())
        })
        .await?;

    assert_eq!(user_name, "Ada");
    // ANCHOR_END: seaorm-adapter

    Ok(())
}
