package io.hydracache.client.hc2;

/** Bounded, pull-based client counters and current retained resources. */
public record ClientMetricsSnapshot(
    long submitted,
    long completed,
    long failed,
    long cancelled,
    long events,
    long listenerFailures,
    int pendingInvocations,
    int activeSubscriptions,
    int activeSessions) {}
