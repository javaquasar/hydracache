package io.hydracache.client.hc2.internal;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.hydracache.client.hc2.ErrorCode;
import io.hydracache.client.hc2.HydraCacheException;
import io.hydracache.client.hc2.RetryAdvice;
import io.hydracache.client.hc2.internal.wire.BatchResult;
import io.hydracache.client.hc2.internal.wire.InvocationResponse;
import io.hydracache.client.hc2.internal.wire.MutationResult;
import io.hydracache.client.hc2.internal.wire.ResponseMeta;
import io.hydracache.client.hc2.internal.wire.RetryDirective;
import io.hydracache.client.hc2.internal.wire.StableErrorCode;
import io.hydracache.client.hc2.internal.wire.ValueResult;
import org.junit.jupiter.api.Test;

final class GrpcHydraCacheClientTest {
  @Test
  void partialBatchKeepsSuccessFailureAndMissInInputOrder() {
    ResponseMeta confirmed = ResponseMeta.newBuilder().setTopologyEpoch(12).build();
    ResponseMeta rejected = ResponseMeta.newBuilder().setTopologyEpoch(13)
        .setError(StableErrorCode.STABLE_ERROR_UNAVAILABLE)
        .setRetry(RetryDirective.RETRY_DIRECTIVE_SAME_CONNECTION).build();
    InvocationResponse response = InvocationResponse.newBuilder().setMeta(confirmed)
        .setBatch(BatchResult.newBuilder()
            .addItems(InvocationResponse.newBuilder().setMeta(confirmed)
                .setMutation(MutationResult.newBuilder().setApplied(true)))
            .addItems(InvocationResponse.newBuilder().setMeta(rejected))
            .addItems(InvocationResponse.newBuilder().setMeta(confirmed)
                .setValue(ValueResult.newBuilder().setFound(false)))).build();
    var results = GrpcHydraCacheClient.mapBatchOutcomes(response, 3);
    assertEquals(java.util.List.of(1, 2, 3), results.stream().map(item -> item.itemId()).toList());
    assertTrue(results.get(0).result().orElseThrow().mutation().orElseThrow().applied());
    assertEquals(ErrorCode.UNAVAILABLE, results.get(1).error().orElseThrow().code());
    assertEquals(13, results.get(1).topologyEpoch());
    assertTrue(results.get(2).result().orElseThrow().value().isEmpty());
  }

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
