# Keys and Tags

Keys and tags are the center of HydraCache's cache contract.

A **key** identifies exactly one cached value. A **tag** identifies a set of cached values that a write can make stale.

Those two jobs should stay separate.

## Key Identity

A good key includes every dimension that changes the result.

This key is usually too weak:

```text
users:active
```

It does not say which tenant, caller, page, sort order, locale, feature variant, or permission policy shaped the result.

A safer key is explicit:

```text
tenant:7:users:status=active:page=1:sort=name:principal=42:policy=3
```

That key is longer because the result has more meaning. The length is a signal, not a failure.

## Tag Invalidation

Tags answer a different question: "Which writes might make this value unsafe?"

For the same cached user search, useful tags might be:

```text
tenant:7
users
users:search
```

A write to one user might invalidate `user:42`. A bulk import might invalidate `users`. A tenant-level policy change might invalidate `tenant:7`.

## Common Mistakes

Do not use a collection tag as the key:

```text
key = users
tag = users
```

That caches one result under a name that sounds like every result. It cannot distinguish active users from disabled users, page 1 from page 2, or tenant 7 from tenant 8.

Do not omit visibility dimensions:

```text
tenant:7:users:active
```

If the caller's permissions shape the result, the caller, role, permission hash, or policy version belongs in the key.

Do not assume TTL fixes a bad key. TTL can limit the time window of a wrong answer, but it cannot make the answer correct.

## Practical Rule

When reviewing a cached read, ask:

1. Does the key identify one exact value shape?
2. Do tags match real write paths?
3. Does the TTL describe fallback freshness rather than primary correctness?
4. Can a future maintainer see why the key and tags were chosen?

If the answer is unclear, keep the cache call explicit. Convenience wrappers should come after the semantics are obvious.
