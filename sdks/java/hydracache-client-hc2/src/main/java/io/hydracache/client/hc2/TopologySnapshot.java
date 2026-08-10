package io.hydracache.client.hc2;

import java.util.List;
import java.util.Objects;

/** Immutable latest accepted topology. */
public record TopologySnapshot(long epoch, List<NodeEndpoint> nodes) {
  public TopologySnapshot {
    if (epoch < 0) throw new IllegalArgumentException("epoch must be nonnegative");
    nodes = List.copyOf(Objects.requireNonNull(nodes, "nodes"));
  }
  public static TopologySnapshot empty() { return new TopologySnapshot(0, List.of()); }
}
