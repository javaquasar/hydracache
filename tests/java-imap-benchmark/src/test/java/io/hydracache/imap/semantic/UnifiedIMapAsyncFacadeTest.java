package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayDeque;
import java.util.concurrent.CompletionException;
import java.util.concurrent.Executor;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.atomic.AtomicLong;
import org.junit.jupiter.api.Test;

final class UnifiedIMapAsyncFacadeTest {
  private static final BoundedIMapApi.Bounds BOUNDS = new BoundedIMapApi.Bounds(8, 8, 8, 2, 8, 64, 8);
  private static final class Queue implements Executor {
    final ArrayDeque<Runnable> actions = new ArrayDeque<>();
    public void execute(Runnable action) { actions.add(action); }
    void next() { actions.removeFirst().run(); }
    void drain() { while (!actions.isEmpty()) next(); }
  }
  private static UnifiedIMapService.DomainCommand put(long id) {
    return new UnifiedIMapService.DomainCommand("set").key(BytesValue.fromHex("61"))
        .value(BytesValue.fromHex("31")).identity(new BoundedIMapApi.MutationIdentity("async", id)).deadline(10);
  }

  @Test
  void unifiedAsyncUsesSharedStoreAndRetainedIdentityAcrossCancellation() {
    var service = new UnifiedIMapService(BOUNDS, 1); var queue = new Queue();
    var tick = new AtomicLong(); var facade = new UnifiedIMapAsyncFacade(service, queue, tick::get, 2);
    var before = facade.invoke(put(1)); assertTrue(before.cancel()); queue.drain();
    assertEquals("not_applied", before.result().toCompletableFuture().join().kind());
    assertTrue(service.snapshot(0).isEmpty());
    var admitted = facade.invoke(put(2)); queue.next();
    assertEquals(1, service.stats(0).inflight()); assertTrue(admitted.cancel());
    assertEquals("outcome_unknown", admitted.result().toCompletableFuture().join().kind()); queue.drain();
    long version = service.snapshot(0).get(0).version();
    var replay = facade.invoke(put(2).deadline(0)); assertTrue(replay.cancel()); queue.drain();
    assertEquals("applied", replay.result().toCompletableFuture().join().kind());
    assertEquals(version, service.snapshot(0).get(0).version()); assertFalse(replay.cancel());
    var read = facade.invoke(new UnifiedIMapService.DomainCommand("get").key(BytesValue.fromHex("61")));
    queue.drain(); assertEquals(BytesValue.fromHex("31"), read.result().toCompletableFuture().join().value().orElseThrow());
    assertEquals(0, facade.pending());
  }

  @Test
  void deadlineLimitsAndExecutorFailuresReleaseReservations() {
    var service = new UnifiedIMapService(BOUNDS, 1); var queue = new Queue(); var tick = new AtomicLong();
    var facade = new UnifiedIMapAsyncFacade(service, queue, tick::get, 1);
    var expired = facade.invoke(put(1).deadline(0));
    assertThrows(RejectedExecutionException.class, () -> facade.invoke(put(2))); queue.drain();
    assertEquals("not_applied", expired.result().toCompletableFuture().join().kind());
    var admitted = facade.invoke(put(3)); queue.next(); tick.set(10); queue.drain();
    assertEquals("not_applied", admitted.result().toCompletableFuture().join().kind());
    assertEquals(0, service.stats(10).inflight());
    var rejected = new UnifiedIMapAsyncFacade(service, action -> { throw new RejectedExecutionException(); }, tick::get, 1);
    assertThrows(CompletionException.class, () -> rejected.invoke(put(4)).result().toCompletableFuture().join());
    assertEquals(0, rejected.pending());
    Executor secondRejected = new Executor() {
      int calls;
      public void execute(Runnable action) { if (++calls == 2) throw new RejectedExecutionException(); action.run(); }
    };
    var second = new UnifiedIMapAsyncFacade(service, secondRejected, tick::get, 1);
    assertThrows(CompletionException.class, () -> second.invoke(put(5).deadline(20)).result().toCompletableFuture().join());
    assertEquals(0, service.stats(10).inflight());
    var invalid = facade.invoke(new UnifiedIMapService.DomainCommand("set").now(10)); queue.drain();
    assertThrows(CompletionException.class, () -> invalid.result().toCompletableFuture().join());
    assertThrows(IllegalArgumentException.class, () -> new UnifiedIMapAsyncFacade(service, queue, tick::get, 0));
  }

  @Test
  void postAdmissionClockFailureAndConflictingReplayReleaseOwnership() {
    var service = new UnifiedIMapService(BOUNDS, 1); var queue = new Queue();
    var clockCalls = new java.util.concurrent.atomic.AtomicInteger();
    var broken = new UnifiedIMapAsyncFacade(service, queue, () -> {
      if (clockCalls.incrementAndGet() > 1) throw new IllegalStateException("logical clock unavailable");
      return 0;
    }, 1);
    var failed = broken.invoke(put(1)); queue.drain();
    assertThrows(CompletionException.class, () -> failed.result().toCompletableFuture().join());
    assertEquals(0, broken.pending()); assertEquals(0, service.stats(0).inflight());
    var facade = new UnifiedIMapAsyncFacade(service, queue, () -> 0, 1);
    var success = facade.invoke(put(2)); queue.drain(); success.result().toCompletableFuture().join();
    var conflict = facade.invoke(put(2).value(BytesValue.fromHex("32"))); queue.drain();
    assertThrows(CompletionException.class, () -> conflict.result().toCompletableFuture().join());
    assertEquals(0, facade.pending());
  }
}
