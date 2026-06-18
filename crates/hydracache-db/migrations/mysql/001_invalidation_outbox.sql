create table if not exists hydracache_schema_version (
    artifact varchar(191) primary key,
    version bigint not null
);

insert ignore into hydracache_schema_version (artifact, version)
values ('hydracache_invalidation_outbox', 1);

create table if not exists hydracache_invalidation_outbox (
    id varchar(191) primary key,
    namespace varchar(191) not null,
    commit_position varchar(191) not null,
    target_hash varchar(64) not null,
    intent_kind varchar(32) not null,
    cache_key text null,
    cache_tag text null,
    entity_name text null,
    collection_name text null,
    reason text null,
    payload_json text null,
    created_at_ms bigint not null,
    available_at_ms bigint not null,
    claimed_at_ms bigint null,
    claim_owner text null,
    published_at_ms bigint null,
    attempts bigint not null default 0,
    state varchar(32) not null default 'pending',
    last_error text null,
    unique key uq_hydracache_outbox_idempotency (namespace, commit_position, target_hash)
);

create index idx_hydracache_outbox_available
on hydracache_invalidation_outbox (namespace, state, available_at_ms, created_at_ms);

create index idx_hydracache_outbox_claim
on hydracache_invalidation_outbox (claim_owner(191), claimed_at_ms);

create index idx_hydracache_outbox_published
on hydracache_invalidation_outbox (namespace, published_at_ms);
