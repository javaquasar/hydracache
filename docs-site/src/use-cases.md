# Use Cases

HydraCache fits services where cache correctness depends on domain knowledge.

## Expensive Async Work

Use `get_or_load` or `cacheable_loader!` when a value is expensive to compute and the application can name the value with a stable key.

Good examples:

- HTTP API responses keyed by tenant, endpoint, and parameters;
- generated reports keyed by tenant, date range, and permission scope;
- authorization or feature-flag lookups keyed by subject and context.

## Database Query Results

Use `hydracache-db` or an adapter crate when a repository result is reusable and the application knows which writes make it stale.

Good examples:

- one user by id;
- one page of a tenant-scoped search;
- a small collection used on many requests;
- a repository method whose SQL or ORM shape should remain in application code.

## Write-side Invalidation

Use tags when a write affects more than one cached key.

Examples:

- `user:42` for one entity;
- `users` for collection-level reads;
- `tenant:7` for tenant-wide invalidation;
- `permission:abc` for permission-scoped query families.

## Request Storm Control

Use single-flight loaders when many requests can miss the same key at the same time. HydraCache lets one loader run while other callers join the in-flight work.

## Local Near-cache Before Coordination

Use the local cache first. Add the invalidation bus or cluster APIs when several local caches need to react to the same invalidation intent.
