# Configurable Persistence

HydraCache persistence is opt-in per namespace. The SQL database remains the
system of record; this layer is a Hazelcast-style value-plane hot-restart aid for
selected cache namespaces.

Unconfigured namespaces are RAM-only. A node must refuse startup when a
persistent namespace is configured without a storage directory or without the
`durable-value-store` feature. This avoids the dangerous fallback where an
operator asks for persistence and silently gets RAM.

## Example

Hazelcast-style intent:

```yaml
cache.jwt.pem:
  persist: true
  durability: sync
  snapshot-interval-seconds: 30
  regions: [eu]
cache.*:
  persist: false
```

HydraCache config shape:

```json
{
  "storage_dir": "/var/lib/hydracache/values",
  "snapshot_interval_default_secs": 30,
  "recovery": {
    "mode": "full_recovery_only",
    "validation_timeout_secs": 30,
    "data_load_timeout_secs": 30,
    "auto_remove_stale_data": false
  },
  "namespaces": {
    "cache.jwt.pem": {
      "persist": true,
      "durability": "sync",
      "snapshot_interval_secs": 30,
      "regions": { "only": ["eu"] }
    },
    "cache.*": {
      "persist": false
    }
  }
}
```

Exact namespace rules are eligible for bounded metric labels. Wildcard and ad-hoc
namespaces aggregate into `other`, with per-namespace detail left to diagnostics.

## Recovery Contract

`FullRecoveryOnly` refuses to serve if durable validation or data loading cannot
finish safely. Recovered records are fenced by the control-plane epoch: older
records are counted and not served, so recovery cannot resurrect stale data.

Switching a namespace from persistent to RAM-only stops recovering it on restart.
Switching from RAM-only to persistent affects new writes only; historical data is
not backfilled without an explicit rebuild.
