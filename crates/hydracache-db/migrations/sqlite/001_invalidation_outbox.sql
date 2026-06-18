create table if not exists hydracache_schema_version (
    artifact text primary key,
    version integer not null
);

insert or ignore into hydracache_schema_version (artifact, version)
values ('hydracache_invalidation_outbox', 1);

create table if not exists hydracache_invalidation_outbox (
    id text primary key,
    namespace text not null,
    commit_position text not null,
    target_hash text not null,
    intent_kind text not null,
    cache_key text null,
    cache_tag text null,
    entity_name text null,
    collection_name text null,
    reason text null,
    payload_json text null,
    created_at_ms integer not null,
    available_at_ms integer not null,
    claimed_at_ms integer null,
    claim_owner text null,
    published_at_ms integer null,
    attempts integer not null default 0,
    state text not null default 'pending',
    last_error text null,
    unique (namespace, commit_position, target_hash)
);

create index if not exists idx_hydracache_outbox_available
on hydracache_invalidation_outbox (namespace, state, available_at_ms, created_at_ms);

create index if not exists idx_hydracache_outbox_claim
on hydracache_invalidation_outbox (claim_owner, claimed_at_ms);

create index if not exists idx_hydracache_outbox_published
on hydracache_invalidation_outbox (namespace, published_at_ms);
