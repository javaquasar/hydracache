package io.hydracache.imap.semantic;

import java.util.Objects;

/** Non-versioned scaffold identity; product versions are frozen only after 0.74 publishes. */
public record AdapterIdentity(String product, String implementation, String readiness) {
  public AdapterIdentity {
    if (Objects.requireNonNull(product, "product").isBlank()
        || Objects.requireNonNull(implementation, "implementation").isBlank()
        || Objects.requireNonNull(readiness, "readiness").isBlank()) {
      throw new IllegalArgumentException("adapter identity fields must not be blank");
    }
  }
}
