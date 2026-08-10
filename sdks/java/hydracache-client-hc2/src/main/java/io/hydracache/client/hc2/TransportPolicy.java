package io.hydracache.client.hc2;

/** Fail-closed transport selection for the HC/2 preview SDK. */
public enum TransportPolicy {
  /** Generated bidirectional gRPC over mutually authenticated TLS. */
  GRPC_MTLS_REQUIRED
}
