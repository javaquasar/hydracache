package io.hydracache.client.hc2;

import java.util.concurrent.CompletableFuture;

/** Active server subscription. Closing emits an HC/2 unsubscribe command exactly once. */
public interface Subscription extends AutoCloseable {
  long id();
  long initialWatermark();
  /** Last accepted watermark when recovery ownership is enabled. */
  default long lastWatermark() { return initialWatermark(); }
  /** Explicitly confirm cache repair and re-register from the supplied watermark. */
  default CompletableFuture<Void> repair(long watermark) {
    return CompletableFuture.failedFuture(new HydraCacheException(
        ErrorCode.UNSUPPORTED, RetryAdvice.NEVER, "subscription is not recovery-owned"));
  }
  @Override void close();
}
