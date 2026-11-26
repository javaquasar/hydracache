package io.hydracache.client.hc2.internal;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.hydracache.client.hc2.ErrorCode;
import io.hydracache.client.hc2.HydraCacheException;
import io.hydracache.client.hc2.RetryAdvice;
import io.hydracache.client.hc2.SubscriptionGap;
import io.hydracache.client.hc2.internal.wire.BatchItemReceipt;
import io.hydracache.client.hc2.internal.wire.BatchItemState;
import io.hydracache.client.hc2.internal.wire.BatchResult;
import io.hydracache.client.hc2.internal.wire.EventGapDetails;
import io.hydracache.client.hc2.internal.wire.EventGapReason;
import io.hydracache.client.hc2.internal.wire.InvocationResponse;
import io.hydracache.client.hc2.internal.wire.MutationResult;
import io.hydracache.client.hc2.internal.wire.PartitionGap;
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
    assertTrue(results.stream().allMatch(item -> item.receipt().isEmpty()));
  }

  @Test
  void typedBatchReceiptPreservesVersionZeroAndRejectsWrongPosition() {
    var receipt = BatchItemReceipt.newBuilder().setItemId(1).setPartitionId(27)
        .setState(BatchItemState.BATCH_ITEM_STATE_CONFIRMED).setAppliedEpoch(3).setAppliedVersion(0);
    var batch = BatchResult.newBuilder()
        .addItems(InvocationResponse.newBuilder().setValue(ValueResult.getDefaultInstance()))
        .addReceipts(receipt);
    var response = InvocationResponse.newBuilder().setBatch(batch).build();
    var applied = GrpcHydraCacheClient.mapBatchOutcomes(response, 1)
        .get(0).receipt().orElseThrow();
    assertEquals(27, applied.partitionId());
    assertEquals(3, applied.applied().orElseThrow().epoch());
    assertEquals(0, applied.applied().orElseThrow().version());
    batch.setReceipts(0, receipt.setItemId(2));
    assertThrows(HydraCacheException.class, () -> GrpcHydraCacheClient.mapBatchOutcomes(
        InvocationResponse.newBuilder().setBatch(batch).build(), 1));
  }

  @Test
  void listenerGapDecodesPartitionVersionsAndWidensMalformedDetails() {
    var part = PartitionGap.newBuilder().setPartitionId(21).setPreviousVersion(5).setObservedVersion(19);
    var wire = EventGapDetails.newBuilder().setReason(EventGapReason.EVENT_GAP_REASON_HISTORY_LOST)
        .addPartitions(part);
    var gap = GrpcHydraCacheClient.mapGap(wire.build());
    assertEquals(SubscriptionGap.Reason.HISTORY_LOST, gap.reason());
    assertEquals(java.util.List.of(new SubscriptionGap.Partition(21, 5, 19)), gap.partitions());
    gap = GrpcHydraCacheClient.mapGap(wire.addPartitions(part).build());
    assertEquals(SubscriptionGap.Reason.UNKNOWN, gap.reason());
    assertTrue(gap.partitions().isEmpty());
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
