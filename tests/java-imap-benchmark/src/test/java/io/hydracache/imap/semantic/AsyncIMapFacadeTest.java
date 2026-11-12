package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayDeque;
import java.util.List;
import java.util.concurrent.CompletionException;
import java.util.concurrent.Executor;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.atomic.AtomicLong;
import org.junit.jupiter.api.Test;

final class AsyncIMapFacadeTest {
  private static final BoundedIMapApi.Bounds BOUNDS =
      new BoundedIMapApi.Bounds(8, 8, 8, 2, 2, 256, 8);
  private static BytesValue bytes(String hex) { return BytesValue.fromHex(hex); }
  private static BoundedIMapApi.MutationIdentity id(long sequence) {
    return new BoundedIMapApi.MutationIdentity("async", sequence);
  }

  private static final class QueueExecutor implements Executor {
    private final ArrayDeque<Runnable> queue = new ArrayDeque<>();
    @Override public void execute(Runnable action) { queue.add(action); }
    void next() { queue.removeFirst().run(); }
    void drain() { while (!queue.isEmpty()) next(); }
  }

  @Test
  void cancellationBeforeDispatchAndAfterAdmissionPreservesIdentity() {
    var map = new BoundedIMapApi(BOUNDS);
    var queue = new QueueExecutor();
    var facade = new AsyncIMapFacade(map, queue, () -> 0, 2);
    var before = facade.tryPut(id(1), bytes("61"), bytes("31"), null, 10);
    assertTrue(before.cancel());
    assertEquals(BoundedIMapApi.TryStatus.NOT_APPLIED,
        before.result().toCompletableFuture().join().status());
    queue.drain();
    assertEquals(0, map.size(0));
    assertEquals(0, facade.pendingInvocations());

    var admitted = facade.set(id(2), bytes("61"), bytes("31"), null, 10);
    queue.next();
    assertEquals(id(2), admitted.identity());
    assertTrue(admitted.cancel());
    assertEquals(BoundedIMapApi.TryStatus.OUTCOME_UNKNOWN,
        admitted.result().toCompletableFuture().join().status());
    queue.drain();
    long revision = map.revision();
    var replay = facade.tryPut(id(2), bytes("61"), bytes("31"), null, 10);
    queue.drain();
    assertEquals(BoundedIMapApi.TryStatus.APPLIED,
        replay.result().toCompletableFuture().join().status());
    assertEquals(revision, map.revision());
    assertFalse(admitted.cancel());
    assertEquals(0, facade.pendingInvocations());
  }

  @Test
  void asyncReadsBulkRemovalAndLogicalDeadlinesShareTheReferenceStore() {
    var map = new BoundedIMapApi(BOUNDS);
    var queue = new QueueExecutor();
    var tick = new AtomicLong();
    var facade = new AsyncIMapFacade(map, queue, tick::get, 8);
    facade.set(id(1), bytes("61"), bytes("31"), null, 10);
    queue.drain();
    var read = facade.getOrDefault(bytes("61"), bytes("ff"), 10);
    var contains = facade.containsEntry(bytes("61"), bytes("31"), 10);
    var view = facade.getEntryView(bytes("61"), 10);
    var replaced = facade.replaceAllDetailed(id(4), List.of(
        new BoundedIMapApi.ReplaceInput(bytes("61"), bytes("32"))), 10);
    queue.drain();
    assertEquals(bytes("31"), read.toCompletableFuture().join());
    assertTrue(contains.toCompletableFuture().join());
    assertTrue(view.toCompletableFuture().join().isPresent());
    assertTrue(replaced.result().toCompletableFuture().join().items().get(0).replaced());
    map.addEntryListener(1);
    var listener = facade.removeEntryListener(1, 10);
    var removed = facade.tryRemove(id(2), bytes("61"), 10);
    queue.drain();
    assertTrue(listener.toCompletableFuture().join().removed());
    assertEquals(BoundedIMapApi.TryStatus.APPLIED,
        removed.result().toCompletableFuture().join().status());
    var expiredRead = facade.getEntryView(bytes("61"), 10);
    var expiredMutation = facade.tryRemove(id(3), bytes("61"), 10);
    tick.set(10);
    queue.drain();
    assertThrows(CompletionException.class, () -> expiredRead.toCompletableFuture().join());
    assertEquals(BoundedIMapApi.TryStatus.NOT_APPLIED,
        expiredMutation.result().toCompletableFuture().join().status());
  }

  @Test
  void cancelledAdmittedBulkReplaysTheOriginalReceiptsOnlyOnce() {
    var map = new BoundedIMapApi(BOUNDS);
    var queue = new QueueExecutor();
    var facade = new AsyncIMapFacade(map, queue, () -> 0, 2);
    map.set(bytes("61"), bytes("31"), 5L, 0);
    var request = List.of(new BoundedIMapApi.ReplaceInput(bytes("61"), bytes("32")));
    var before = facade.replaceAllDetailed(id(1), request, 10);
    assertTrue(before.cancel());
    queue.drain();
    assertEquals(BoundedIMapApi.TryStatus.NOT_APPLIED,
        before.result().toCompletableFuture().join().outcome().status());
    var admitted = facade.replaceAllDetailed(id(1), request, 10);
    queue.next();
    assertTrue(admitted.cancel());
    queue.drain();
    assertEquals(BoundedIMapApi.TryStatus.OUTCOME_UNKNOWN,
        admitted.result().toCompletableFuture().join().outcome().status());
    long revision = map.revision();
    var replay = facade.replaceAllDetailed(id(1), request, 10);
    queue.drain();
    assertTrue(replay.result().toCompletableFuture().join().items().get(0).replaced());
    assertEquals(revision, map.revision());
    assertEquals(0, facade.pendingInvocations());
  }

  @Test
  void admittedRemoveCancellationReportsUnknownAndReplayProvesTheDeletion() {
    var map = new BoundedIMapApi(BOUNDS);
    var queue = new QueueExecutor();
    var facade = new AsyncIMapFacade(map, queue, () -> 0, 2);
    map.set(bytes("61"), bytes("31"), null, 0);
    var removal = facade.tryRemove(id(1), bytes("61"), 10);
    queue.next();
    assertTrue(removal.cancel());
    assertEquals(BoundedIMapApi.TryStatus.OUTCOME_UNKNOWN,
        removal.result().toCompletableFuture().join().status());
    queue.drain();
    assertTrue(map.isEmpty(0));
    long revision = map.revision();
    var replay = facade.tryRemove(id(1), bytes("61"), 10);
    queue.drain();
    assertEquals(BoundedIMapApi.TryStatus.APPLIED,
        replay.result().toCompletableFuture().join().status());
    assertEquals(revision, map.revision());
    assertTrue(map.isEmpty(0));
    assertEquals(0, facade.pendingInvocations());
  }

  @Test
  void expiredOrCancelledReplayNeverInventsNotAppliedAfterOriginalCommit() {
    var map = new BoundedIMapApi(BOUNDS);
    var queue = new QueueExecutor();
    var facade = new AsyncIMapFacade(map, queue, () -> 1, 4);
    map.tryPut(id(1), bytes("61"), bytes("31"), null, 10, 0, false);
    long revision = map.revision();
    var expired = facade.tryPut(id(1), bytes("61"), bytes("31"), null, 0);
    var cancelled = facade.tryPut(id(1), bytes("61"), bytes("31"), null, 10);
    assertTrue(cancelled.cancel());
    assertEquals(BoundedIMapApi.TryStatus.APPLIED,
        cancelled.result().toCompletableFuture().join().status());
    queue.drain();
    assertEquals(BoundedIMapApi.TryStatus.APPLIED,
        expired.result().toCompletableFuture().join().status());
    assertEquals(revision, map.revision());
    var conflict = facade.tryRemove(id(1), bytes("61"), 0);
    assertTrue(conflict.cancel());
    assertThrows(CompletionException.class, () -> conflict.result().toCompletableFuture().join());
    queue.drain();
    assertEquals(0, facade.pendingInvocations());
  }

  @Test
  void pendingLimitsExecutorRejectionAndBackendErrorsReleaseOwnership() {
    var map = new BoundedIMapApi(BOUNDS);
    var queue = new QueueExecutor();
    var facade = new AsyncIMapFacade(map, queue, () -> 0, 1);
    facade.getEntryView(bytes("61"), 10);
    assertThrows(RejectedExecutionException.class,
        () -> facade.tryRemove(id(1), bytes("61"), 10));
    queue.drain();
    var rejected = new AsyncIMapFacade(map,
        action -> { throw new RejectedExecutionException("rejected"); }, () -> 0, 2);
    assertThrows(CompletionException.class,
        () -> rejected.getEntryView(bytes("61"), 10).toCompletableFuture().join());
    assertThrows(CompletionException.class, () -> rejected.tryRemove(id(1), bytes("61"), 10)
        .result().toCompletableFuture().join());
    assertEquals(0, rejected.pendingInvocations());

    var failure = facade.tryPut(id(1), bytes("61"), bytes("31"), 0L, 10);
    queue.drain();
    assertThrows(CompletionException.class, () -> failure.result().toCompletableFuture().join());
    assertEquals(0, facade.pendingInvocations());
    Executor rejectingSecondPhase = new Executor() {
      private boolean admitted;
      @Override public void execute(Runnable action) {
        if (admitted) throw new RejectedExecutionException("second phase");
        admitted = true;
        action.run();
      }
    };
    var secondPhase = new AsyncIMapFacade(map, rejectingSecondPhase, () -> 0, 1);
    assertThrows(CompletionException.class, () -> secondPhase.tryRemove(id(2), bytes("61"), 10)
        .result().toCompletableFuture().join());
    assertEquals(0, secondPhase.pendingInvocations());
    var badClock = new AsyncIMapFacade(map, queue,
        () -> { throw new IllegalStateException("clock failed"); }, 1);
    var badInvocation = badClock.tryRemove(id(3), bytes("61"), 10);
    queue.drain();
    assertThrows(CompletionException.class, () -> badInvocation.result().toCompletableFuture().join());
    assertEquals(0, badClock.pendingInvocations());
    assertThrows(IllegalArgumentException.class, () -> new AsyncIMapFacade(map, queue, () -> 0, 0));
  }
}
