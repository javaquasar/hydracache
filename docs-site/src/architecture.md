# Architecture

HydraCache is built as a set of explicit cache boundaries rather than one invisible cache layer.

The local runtime owns typed serialization, TTL, tags, single-flight loading, diagnostics, and event streams. Database adapters build query-result descriptors on top of that runtime. Distributed invalidation and cluster APIs carry invalidation intent and membership metadata without hiding the local cache.

## Runtime Flow

```mermaid
flowchart TD
    A["Application call site"] --> B["Key + tags + TTL/refresh policy"]
    B --> C["HydraCache local runtime"]
    C --> D{"Fresh entry?"}
    D -->|"yes"| E["Decode typed value"]
    D -->|"no"| F["Single-flight loader"]
    F --> G["Store encoded value"]
    G --> E
    C --> H["Stats, diagnostics, events"]
```

On a hit, the runtime decodes and returns the cached value. On a miss, `get_or_load` runs a loader and stores the result under the chosen key and tags. Concurrent same-key misses share one in-flight loader.

## Query Flow

```mermaid
flowchart TD
    A["Repository method"] --> B["DbCache namespace"]
    B --> C["QueryCachePolicy or PreparedQueryPolicy"]
    C --> D["HydraCache local runtime"]
    D --> E{"Cache hit?"}
    E -->|"yes"| F["Return cached query result"]
    E -->|"no"| G["Run SQLx, Diesel, SeaORM, or repository loader"]
    G --> H["Store result with tags"]
    H --> F
```

HydraCache does not parse SQL or infer table dependencies. The application names the result and the writes that can invalidate it.

## Invalidation Flow

```mermaid
flowchart LR
    A["Write path after commit"] --> B["invalidate_key/tag/flush"]
    B --> C["Local tag index"]
    B --> D["Optional invalidation bus"]
    D --> E["Peer local cache"]
    E --> F["Remove local copies"]
```

The bus propagates intent, not values. This keeps distributed behavior local-first: each process remains responsible for its own cache contents and loader code.

## Cluster Flow

Client/member cluster mode adds role, node id, generation, membership, ownership, and peer-fetch vocabulary. It does not turn HydraCache into a production data grid by itself.

Use cluster APIs when the application needs stable membership diagnostics or a future route toward owner-based peer reads. Keep local cache semantics visible even when multiple processes participate.
