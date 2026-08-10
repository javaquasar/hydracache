# HydraCache HC/2 Rust client (preview)

This crate is the production-shaped, transport-neutral Rust SDK for the
generated HC/2 contract. It is a preview: the gRPC+mTLS adapter is independently
interoperable, but the production daemon listener and reconnect/repair policy
remain blocked on H01 and H11.

The stable HC/1 request/reply SDK remains the separate `hydracache-client`
crate. No HC/1 endpoint, frame, identity, or fallback is accepted here.
