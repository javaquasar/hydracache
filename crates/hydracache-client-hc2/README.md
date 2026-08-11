# HydraCache HC/2 Rust client (preview)

This crate is the production-shaped, transport-neutral Rust SDK for the
generated HC/2 contract. It is a preview: the gRPC+mTLS adapter is independently
interoperable with the production daemon and the transport-neutral recovery owner provides bounded
reconnect, conservative subscription repair, duplicate suppression, and
fail-loud fenced-session loss. The real-process gate covers data, push,
fenced-session, mTLS identity, clean close, and bounded daemon drain.

The stable HC/1 request/reply SDK remains the separate `hydracache-client`
crate. No HC/1 endpoint, frame, identity, or fallback is accepted here.
