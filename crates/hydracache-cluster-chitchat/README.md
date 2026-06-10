# hydracache-cluster-chitchat

Real `chitchat`-backed discovery adapter for HydraCache cluster mode.

This crate keeps the base `hydracache` crate free from gossip dependencies.
Applications that need gossip-based candidate discovery can opt in to this
crate and pass `Arc<ChitchatDiscovery>` through
`HydraCache::client().discovery(...)` or `HydraCache::member().discovery(...)`.
