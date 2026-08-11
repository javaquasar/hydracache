package io.hydracache.client.hc2;

import java.util.Objects;

/** Named, already fail-closed endpoint used by bounded reconnect selection. */
public record ReconnectEndpoint(String id, HydraCacheClientConfig config) {
  public ReconnectEndpoint {
    Objects.requireNonNull(id, "id");
    Objects.requireNonNull(config, "config");
    if (id.isBlank() || id.length() > 128 || id.chars().anyMatch(Character::isISOControl)) {
      throw new IllegalArgumentException("endpoint id must be nonblank, bounded, and contain no controls");
    }
  }
}
