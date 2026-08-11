# HydraCache HC/2 Python Client

This package is the installable preview Python SDK for the generated HC/2
client plane. It uses one long-lived bidirectional gRPC stream, generated
protobuf messages, bounded pending/subscription/session state, explicit mTLS,
asyncio cancellation, and bounded reconnect with listener restoration.

```python
from hydracache_hc2 import AsyncHydraCacheClient, ClientConfig

client = await AsyncHydraCacheClient.connect(ClientConfig(
    endpoint="cache.example:9444",
    client_id="orders-api",
    tenant="orders",
    root_certificate="ca.pem",
    client_certificate="client.pem",
    client_private_key="client.key",
    server_name="cache.example",
))
await client.put(b"key", b"value")
value = await client.get(b"key")
await client.close()
```

TLS is mandatory unless `insecure=True` is stated explicitly. The insecure
mode exists for loopback tests and never activates implicitly. Values and keys
are opaque bytes; the SDK does not deserialize application objects.

The API is preview until the 0.68 release cut. It is not a Hazelcast wire
client and does not implement smart routing.
