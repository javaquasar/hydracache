package io.hydracache.client.hc2.internal;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;

import io.hydracache.client.hc2.ErrorCode;
import io.hydracache.client.hc2.HydraCacheException;
import io.hydracache.client.hc2.RetryAdvice;
import org.junit.jupiter.api.Test;

final class GrpcHydraCacheClientTest {
  @Test
  void preHandshakeClosedTransportIsReconnectableButSecurityFailureIsTerminal() {
    HydraCacheException closed = new HydraCacheException(
        ErrorCode.UNAVAILABLE, RetryAdvice.NEVER, "client is closed");

    HydraCacheException classified =
        GrpcHydraCacheClient.classifyPreHandshakeFailure(closed);

    assertEquals(ErrorCode.UNAVAILABLE, classified.code());
    assertEquals(RetryAdvice.RECONNECT_IDEMPOTENT, classified.retryAdvice());
    assertEquals("HC/2 stream closed before handshake", classified.getMessage());
    assertSame(closed, classified.getCause());

    HydraCacheException security = new HydraCacheException(
        ErrorCode.UNAUTHENTICATED, RetryAdvice.NEVER, "certificate rejected");
    assertSame(security, GrpcHydraCacheClient.classifyPreHandshakeFailure(security));
  }
}
