package io.hydracache.client.hc2;

/** Result of a single-key fenced-lock acquisition attempt. */
public record LockAcquireResult(boolean acquired, long fence) {
  public LockAcquireResult {
    if ((acquired && fence <= 0) || (!acquired && fence != 0)) {
      throw new IllegalArgumentException("fence must be positive exactly when acquired");
    }
  }
}
