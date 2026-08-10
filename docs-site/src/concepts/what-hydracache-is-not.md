# What HydraCache Is Not

HydraCache is intentionally narrow in several places. These boundaries are part of the design.

## Not an ORM

HydraCache does not replace SQLx, Diesel, SeaORM, or a repository layer.

The database library remains responsible for SQL, query planning, transactions, row mapping, retries, and database errors. HydraCache wraps the result boundary: key, tags, TTL, loader execution, single-flight, serialization, and invalidation.

## Not SQL Interception

HydraCache does not transparently intercept arbitrary SQL and decide what to cache.

Applications need to describe query-result identity explicitly. This keeps tenant, principal, permission, filter, page, sort, feature, and policy dimensions visible in code review.

## Not Automatic CDC

HydraCache does not install database triggers or provide change data capture by default.

If writes happen outside the service, those writers need an invalidation path too. The cache cannot safely infer external changes by looking only at reads.

## Not a Strongly Consistent Distributed Database

Distributed invalidation can reduce stale reads across nodes, but it does not turn a local cache into a strongly consistent replicated database.

HydraCache starts local-first: make the local invalidation contract correct, then carry that invalidation to peers.

## Not Just a Map

HydraCache can store typed values, but its main purpose is not "a map with TTL."

The runtime exists to keep cache semantics explicit:

- key identity;
- tag invalidation;
- TTL and stale policy;
- single-flight loading;
- query-result caching;
- observability around hits, misses, loads, and invalidations.

This makes the API more deliberate than a raw map, and that deliberateness is the point.
