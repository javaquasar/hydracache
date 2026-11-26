package io.hydracache.client.hc2;

import java.util.List;
import java.util.Objects;

/** Repair information for a gap. An empty partition list is stream-wide. */
public record SubscriptionGap(Reason reason, List<Partition> partitions) {
  public enum Reason { UNKNOWN, TOPOLOGY_CHANGED, HISTORY_LOST, RESUME_UNBOUND, CLIENT_OVERFLOW }

  public record Partition(int partitionId, long previousVersion, long observedVersion) {}

  public SubscriptionGap {
    Objects.requireNonNull(reason, "reason");
    partitions = List.copyOf(partitions);
  }
}
