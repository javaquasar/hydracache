# Local-first Invalidation

Distributed invalidation should start with a correct local invalidation model.

HydraCache treats local invalidation as the source of truth:

1. the local cache removes values by key or tag;
2. the invalidation event can be published to peers;
3. peers apply the same invalidation locally;
4. each node keeps read behavior simple and observable.

This avoids making the distributed layer responsible for interpreting database writes or query semantics. The application still decides which keys and tags describe a value. The distributed layer carries that decision to other nodes.

## Why Local First

If a cache cannot invalidate correctly inside one process, distributing that invalidation only spreads uncertainty.

The local model should be clear before multi-node behavior enters the picture:

- writes know which tags they affect;
- cache entries attach those tags consistently;
- invalidation happens after commit;
- metrics show which keys and tags were removed.

After that, distributed invalidation becomes a transport problem: carry the same invalidation decision to other nodes.

## Expected Semantics

Local-first distributed invalidation should be treated as a freshness improvement, not a replacement for the database's consistency model.

Applications should still design for:

- delayed messages;
- duplicated invalidation messages;
- nodes that miss a message and rely on TTL fallback;
- external writers that need their own invalidation path.
