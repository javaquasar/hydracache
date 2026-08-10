# HC/2 v2alpha Schema Registry

The authoritative HC/2 client-plane contract is
`crates/hydracache-client-hc2/proto/hc2_contract.proto`. Generated
Rust, Java, and future SDK bindings consume that file; SDKs must not maintain a
second operation table or handwritten protobuf field identifiers.

## Evolution rules

- The schema remains `v2alpha` until the transport ADR and cross-language gates
  pass. Alpha permits additive work, not silent wire reinterpretation.
- Oneof field numbers `101..119` are the operation/result identifier registry.
  Existing identifiers never change meaning.
- Removed field numbers and names remain `reserved` permanently.
- Additive unknown fields must survive transparent relay byte-for-byte. Rust
  relays use `PreservedMessage`; mutating a message with unknown fields requires
  an explicit lossy-discard call. Java protobuf retains unknown fields natively.
- Stable error and retry enums are contract metadata. SDKs consume generated
  values and do not infer retry policy from free-form text.
- Generation, connection generation, deadline, cancellation, idempotency,
  topology epoch, subscription watermark, and fencing fields are protocol data,
  not transport-specific side channels.

## Compatibility evidence

The Rust contract test decodes the generated descriptor, freezes existing
operation/envelope identifiers, verifies unique names/numbers and reserved
ranges, checks a committed cross-language golden frame, exercises additive
unknown-field relay, and proves vendored `protoc` rejects duplicate and reserved
identifier canaries. The Java fixture consumes the same proto tree and proves
the same golden bytes, stable descriptor IDs, and native unknown-field relay.

Python clean generation and its golden corpus are owned by H15. H02 establishes
the sole source contract it must consume; H15 is still required before the
overall HC/2 cross-language gate can close.
