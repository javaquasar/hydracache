package io.hydracache.client.hc2;

/** Bounded total operation attempts, intentionally independent from reconnect attempts. */
public record InvocationRetryPolicy(int maxAttempts) {
  public InvocationRetryPolicy {
    if (maxAttempts < 1 || maxAttempts > 4) {
      throw new IllegalArgumentException("invocation retry attempts must be in [1, 4]");
    }
  }

  public static InvocationRetryPolicy defaults() { return new InvocationRetryPolicy(2); }
}
