package io.hydracache.client.hc2;

/** Privacy-safe recovery counters without endpoint, tenant, key, or value labels. */
public record RecoveryMetricsSnapshot(
    long reconnectAttempts,
    long reconnectSuccesses,
    long reconnectExhausted,
    long invocationRetries,
    long subscriptionRepairs,
    long duplicateEvents,
    long gapRepairsRequired,
    long sessionLosses) {
  public static RecoveryMetricsSnapshot empty() {
    return new RecoveryMetricsSnapshot(0, 0, 0, 0, 0, 0, 0, 0);
  }
}
