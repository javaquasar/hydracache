package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.CompletionException;
import java.util.concurrent.Executor;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;
import org.junit.jupiter.api.Test;

final class LocalHelperAsyncTest {
  private static final class Queue implements Executor {
    final ArrayDeque<Runnable> actions = new ArrayDeque<>(); boolean rejectNext;
    public void execute(Runnable action) { if (rejectNext) { rejectNext = false; throw new RejectedExecutionException("injected"); } actions.add(action); }
    void next() { actions.removeFirst().run(); }
    void drain() { while (!actions.isEmpty()) next(); }
  }
  private static BytesValue b(String value) { return BytesValue.fromHex(value); }
  private static BoundedIMapApi.MutationIdentity id(long value) { return new BoundedIMapApi.MutationIdentity("helper", value); }
  private static UnifiedIMapService owner() { return new UnifiedIMapService(new BoundedIMapApi.Bounds(16, 16, 32, 16, 32, 4096, 32), 1); }
  private static void set(UnifiedIMapService service, String key, String value, long sequence, TtlDirective ttl) {
    service.execute(new UnifiedIMapService.DomainCommand("set").key(b(key)).value(b(value)).ttl(ttl)
        .identity(new BoundedIMapApi.MutationIdentity("point", sequence)));
  }
  private static UnifiedIMapService.VersionToken token(UnifiedIMapService service, String key) {
    return service.execute(new UnifiedIMapService.DomainCommand("get_entry_view").key(b(key))).token().orElseThrow();
  }
  private static <T> T completed(UnifiedIMapAsyncFacade.HelperInvocation<T> invocation, Queue queue) {
    queue.drain(); var result = invocation.result().toCompletableFuture().join();
    assertEquals(IMapLocalHelper.Kind.COMPLETED, result.kind()); return result.value().orElseThrow();
  }

  @Test void versionBatchUsesSameLedgerAndCancellationNeverInventsNotAppliedAfterAdmission() {
    var service = owner(); var queue = new Queue(); var clock = new AtomicLong();
    var facade = new UnifiedIMapAsyncFacade(service, queue, clock::get, 4);
    set(service, "61", "31", 1, TtlDirective.eternal());
    var entries = new ArrayList<>(List.of(new UnifiedIMapService.VersionInput(b("61"), token(service, "61"), Optional.of(b("32")), TtlDirective.preserve())));
    var request = new IMapLocalHelper.VersionBulk(UnifiedIMapService.VersionAction.REPLACE, entries, id(1), 10, false);
    entries.clear(); var invocation = facade.invokeHelper(request); queue.next();
    assertEquals("pending", service.queryOperationOutcome(id(1)).kind());
    assertTrue(invocation.cancel()); assertEquals(IMapLocalHelper.Kind.OUTCOME_UNKNOWN, invocation.result().toCompletableFuture().join().kind());
    queue.drain(); var receipt = service.queryOperationOutcome(id(1)).result().orElseThrow();
    assertEquals("replaced", receipt.items().get(0).kind());
    assertEquals(receipt, service.versionBulk(request.action(), request.entries(), request.identity(), 0, 0, false));
    var retry = facade.invokeHelper(new IMapLocalHelper.VersionBulk(request.action(), request.entries(), id(1), 0, false));
    assertTrue(retry.cancel()); assertEquals(receipt, retry.result().toCompletableFuture().join().value().orElseThrow()); queue.drain();
    assertFalse(retry.cancel());
    var outcomes = completed(facade.invokeHelper(new IMapLocalHelper.QueryOutcomes(List.of(id(1), id(99)), 4096, 10)), queue);
    assertEquals(receipt, outcomes.get(0).status().result().orElseThrow()); assertEquals("unknown", outcomes.get(1).status().kind());
    assertEquals(0, facade.pending()); assertEquals(0, facade.ownedBytes());
  }

  @Test void retirementAndExpiryReplayOriginalReceiptsWithoutRepeatingSideEffects() {
    var service = owner(); var queue = new Queue(); var clock = new AtomicLong();
    var facade = new UnifiedIMapAsyncFacade(service, queue, clock::get, 4);
    var listener = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(16));
    set(service, "61", "31", 1, TtlDirective.expireAfter(1)); set(service, "62", "32", 2, TtlDirective.expireAfter(1)); listener.drain(); clock.set(1);
    var expiry = new IMapLocalHelper.ExpireStep(1, id(1), 10);
    var first = completed(facade.invokeHelper(expiry), queue); assertEquals(1, first.expired()); assertTrue(first.remaining());
    assertEquals(1, listener.drain().size()); long physical = service.resourceUsage().values();
    assertEquals(first, completed(facade.invokeHelper(expiry), queue)); assertEquals(physical, service.resourceUsage().values()); assertTrue(listener.drain().isEmpty());
    completed(facade.invokeHelper(new IMapLocalHelper.ExpireStep(1, id(2), 10)), queue); assertEquals(0, service.resourceUsage().values());
    var retirement = new IMapLocalHelper.RetireOutcomes("point", 1, 2, id(3), 10);
    var retired = completed(facade.invokeHelper(retirement), queue); assertEquals(2, retired.removed());
    assertEquals(retired, completed(facade.invokeHelper(retirement), queue));
    assertEquals("helper_retired", service.queryOperationOutcome(id(3)).result().orElseThrow().kind());
    var unsafe = facade.invokeHelper(new IMapLocalHelper.RetireOutcomes("helper", 1, 4, id(4), 10)); queue.drain();
    assertThrows(CompletionException.class, () -> unsafe.result().toCompletableFuture().join());
    assertEquals("unknown", service.queryOperationOutcome(id(4)).kind()); listener.close();
  }

  @Test void snapshotReplayReturnsOriginalCutAndLeasesDoNotResetLaterEvents() {
    var service = owner(); var queue = new Queue(); var facade = new UnifiedIMapAsyncFacade(service, queue, () -> 0, 4);
    var listener = service.registerListener(1, new UnifiedIMapService.ListenerOptions(true, 16, 16, Optional.empty(), java.util.Set.of()));
    set(service, "61", "31", 1, TtlDirective.eternal());
    var request = new IMapLocalHelper.ListenerSnapshot(1, 16, 1024, id(1), 10);
    var invocation = facade.invokeHelper(request); queue.next(); set(service, "61", "32", 2, TtlDirective.eternal()); queue.drain();
    var snapshot = invocation.result().toCompletableFuture().join().value().orElseThrow();
    assertEquals(b("32"), snapshot.entries().get(0).value().orElseThrow());
    assertTrue(snapshot.includeValue()); assertEquals(1, snapshot.namespaceGeneration()); assertTrue(snapshot.mapId() > 0);
    long watermark = snapshot.watermark(), revision = snapshot.revision(); assertEquals(2, watermark);
    invocation.close(); invocation.close(); assertEquals(0, service.resourceUsage().resync());
    set(service, "61", "33", 3, TtlDirective.eternal());
    try (var historical = completed(facade.invokeHelper(request), queue)) {
      assertEquals(b("32"), historical.entries().get(0).value().orElseThrow()); assertEquals(watermark, historical.watermark()); assertEquals(revision, historical.revision());
      assertEquals(b("33"), listener.drain().get(0).value().orElseThrow());
    }
    var cancelled = facade.invokeHelper(new IMapLocalHelper.ListenerSnapshot(1, 16, 1024, id(2), 10)); queue.next();
    assertTrue(cancelled.cancel()); queue.drain(); assertEquals(IMapLocalHelper.Kind.OUTCOME_UNKNOWN, cancelled.result().toCompletableFuture().join().kind());
    assertEquals(0, service.resourceUsage().resync()); assertEquals("completed", service.queryOperationOutcome(id(2)).kind());
    listener.close();
    try (var historical = completed(facade.invokeHelper(request), queue)) { assertEquals(watermark, historical.watermark()); }
    assertEquals(0, facade.pending()); assertEquals(0, facade.ownedBytes());
  }

  @Test void observationCancellationSkipsWorkAndCheckpointOwnershipIsExplicit() {
    var service = owner(); var queue = new Queue(); var facade = new UnifiedIMapAsyncFacade(service, queue, () -> 0, 2);
    var query = facade.invokeHelper(new IMapLocalHelper.QueryOutcomes(List.of(id(1)), 4096, 10));
    assertTrue(query.cancel()); queue.drain(); assertEquals(IMapLocalHelper.Kind.NOT_APPLIED, query.result().toCompletableFuture().join().kind());
    var checkpoint = facade.invokeHelper(new IMapLocalHelper.Checkpoint(10)); queue.next();
    assertTrue(checkpoint.cancel()); queue.drain(); assertEquals(IMapLocalHelper.Kind.NOT_APPLIED, checkpoint.result().toCompletableFuture().join().kind()); assertEquals(0, service.resourceUsage().resync());
    set(service, "61", "31", 1, TtlDirective.eternal());
    var completed = facade.invokeHelper(new IMapLocalHelper.Checkpoint(10)); var lease = completed(completed, queue);
    assertTrue(service.resourceUsage().resync() > 0); completed.close(); assertEquals(0, service.resourceUsage().resync());
    assertThrows(IMapDomainException.class, () -> UnifiedIMapService.restore(lease));
    try (var result = facade.invokeHelper(new IMapLocalHelper.QueryOutcomes(List.of(id(1)), 4096, 10))) { completed(result, queue); result.result().toCompletableFuture().join().close(); }
    assertEquals(0, facade.ownedBytes());
  }

  @Test void deadlinesExecutorFailuresAndInputByteBudgetsReleaseActualReservations() {
    var service = owner(); var queue = new Queue(); var clock = new AtomicLong();
    var facade = new UnifiedIMapAsyncFacade(service, queue, clock::get, 1);
    var expired = facade.invokeHelper(new IMapLocalHelper.ExpireStep(1, id(1), 0)); queue.drain();
    assertEquals(IMapLocalHelper.Kind.NOT_APPLIED, expired.result().toCompletableFuture().join().kind());
    var later = facade.invokeHelper(new IMapLocalHelper.ExpireStep(1, id(2), 1)); queue.next(); clock.set(1); queue.drain();
    assertEquals(IMapLocalHelper.Kind.NOT_APPLIED, later.result().toCompletableFuture().join().kind()); assertEquals("unknown", service.queryOperationOutcome(id(2)).kind());
    clock.set(0); var rejected = facade.invokeHelper(new IMapLocalHelper.ExpireStep(1, id(3), 10)); queue.rejectNext = true; queue.next();
    assertThrows(CompletionException.class, () -> rejected.result().toCompletableFuture().join()); assertEquals("unknown", service.queryOperationOutcome(id(3)).kind());
    var rejecting = new UnifiedIMapAsyncFacade(service, action -> { throw new RejectedExecutionException(); }, () -> 0, 1);
    assertThrows(CompletionException.class, () -> rejecting.invokeHelper(new IMapLocalHelper.Checkpoint(10)).result().toCompletableFuture().join());
    var calls = new AtomicInteger(); var brokenClock = new UnifiedIMapAsyncFacade(service, queue, () -> { if (calls.incrementAndGet() > 1) throw new IllegalStateException("clock"); return 0; }, 1);
    var broken = brokenClock.invokeHelper(new IMapLocalHelper.ExpireStep(1, id(4), 10)); queue.drain();
    assertThrows(CompletionException.class, () -> broken.result().toCompletableFuture().join()); assertEquals("unknown", service.queryOperationOutcome(id(4)).kind());
    var tiny = new UnifiedIMapAsyncFacade(service, queue, () -> 0, 1, 1);
    assertThrows(RejectedExecutionException.class, () -> tiny.invokeHelper(new IMapLocalHelper.Checkpoint(10)));
    assertThrows(IllegalArgumentException.class, () -> new UnifiedIMapAsyncFacade(service, queue, () -> 0, 1, 0));
    assertThrows(IMapDomainException.class, () -> facade.invokeHelper(new IMapLocalHelper.QueryOutcomes(List.of(id(1)), Long.MAX_VALUE, 10)));
    var pending = facade.invokeSnapshot(new UnifiedIMapService.DomainCommand("get").key(b("61")).freeze());
    assertThrows(RejectedExecutionException.class, () -> facade.invokeHelper(new IMapLocalHelper.Checkpoint(10))); queue.drain();
    assertEquals("absent", pending.result().toCompletableFuture().join().kind());
    assertEquals(0, facade.pending()); assertEquals(0, facade.ownedBytes()); assertEquals(0, rejecting.ownedBytes()); assertEquals(0, brokenClock.ownedBytes());
  }

  @Test void invalidAndConflictingHelpersNeverFabricateSuccessfulReceipts() {
    var service = owner(); var queue = new Queue(); var facade = new UnifiedIMapAsyncFacade(service, queue, () -> 0, 4);
    set(service, "61", "31", 1, TtlDirective.eternal()); var originalToken = token(service, "61");
    var invalid = facade.invokeHelper(new IMapLocalHelper.VersionBulk(UnifiedIMapService.VersionAction.REMOVE,
        List.of(new UnifiedIMapService.VersionInput(b("61"), originalToken, Optional.of(b("32")), TtlDirective.eternal())), id(1), 10, false));
    queue.drain(); assertThrows(CompletionException.class, () -> invalid.result().toCompletableFuture().join()); assertEquals("unknown", service.queryOperationOutcome(id(1)).kind());
    var wrongListener = facade.invokeHelper(new IMapLocalHelper.ListenerSnapshot(99, 16, 1024, id(2), 10)); queue.drain();
    assertThrows(CompletionException.class, () -> wrongListener.result().toCompletableFuture().join());
    var work = facade.invokeHelper(new IMapLocalHelper.ExpireStep(1000, id(3), 10)); queue.drain(); assertThrows(CompletionException.class, () -> work.result().toCompletableFuture().join());
    var first = facade.invokeHelper(new IMapLocalHelper.ExpireStep(1, id(4), 10)); completed(first, queue);
    var conflict = facade.invokeHelper(new IMapLocalHelper.ExpireStep(2, id(4), 10)); queue.drain(); assertThrows(CompletionException.class, () -> conflict.result().toCompletableFuture().join());
    var cancelledConflict = facade.invokeHelper(new IMapLocalHelper.ExpireStep(2, id(4), 10)); assertTrue(cancelledConflict.cancel()); queue.drain(); assertThrows(CompletionException.class, () -> cancelledConflict.result().toCompletableFuture().join());
    assertEquals(originalToken, token(service, "61")); assertEquals(0, facade.ownedBytes());
    assertThrows(IllegalArgumentException.class, () -> new IMapLocalHelper.Outcome<>(IMapLocalHelper.Kind.COMPLETED, Optional.empty()));
    assertThrows(IllegalArgumentException.class, () -> new IMapLocalHelper.QueryOutcomes(List.of(), 1, 10));
    assertThrows(IllegalArgumentException.class, () -> new IMapLocalHelper.QueryOutcomes(List.of(id(1)), 0, 10));
    assertThrows(IllegalArgumentException.class, () -> new IMapLocalHelper.ExpireStep(0, id(1), 10));
    assertThrows(IllegalArgumentException.class, () -> new IMapLocalHelper.ListenerSnapshot(1, 257, 1, id(1), 10));
  }

  @Test void immutableAsyncBulkAndAllVersionActionsKeepBoundedReceiptsAndTimePolicies() {
    var service = owner(); var queue = new Queue(); var facade = new UnifiedIMapAsyncFacade(service, queue, () -> 0, 4);
    var entries = new ArrayList<>(List.of(new BoundedIMapApi.ReplaceInput(b("61"), b("31")), new BoundedIMapApi.ReplaceInput(b("62"), b("32"))));
    var builder = new UnifiedIMapService.DomainCommand("put_all_detailed").identity(new BoundedIMapApi.MutationIdentity("point", 1)); builder.entries = entries;
    var write = facade.invoke(builder); entries.clear(); builder.entries = List.of(); queue.drain();
    assertEquals(2, write.result().toCompletableFuture().join().items().size());
    var read = facade.invoke(new UnifiedIMapService.DomainCommand("get_all").keys(List.of(b("62"), b("61")))); queue.drain();
    assertEquals(b("32"), read.result().toCompletableFuture().join().items().get(0).value().orElseThrow());
    var compare = facade.invoke(new UnifiedIMapService.DomainCommand("conditional_replace").key(b("61")).expected(b("31")).value(b("33")).identity(new BoundedIMapApi.MutationIdentity("point", 2))); queue.drain();
    assertEquals("replaced", compare.result().toCompletableFuture().join().kind());
    var ttl = new IMapLocalHelper.VersionBulk(UnifiedIMapService.VersionAction.SET_TTL,
        List.of(new UnifiedIMapService.VersionInput(b("61"), token(service, "61"), Optional.empty(), TtlDirective.expireAfter(3))), id(1), 10, true);
    assertEquals("outcome_unknown", completed(facade.invokeHelper(ttl), queue).kind());
    var recovered = completed(facade.invokeHelper(ttl), queue); assertEquals(3, recovered.items().get(0).remainingTtl().orElseThrow());
    var remove = new IMapLocalHelper.VersionBulk(UnifiedIMapService.VersionAction.REMOVE,
        List.of(new UnifiedIMapService.VersionInput(b("61"), token(service, "61"), Optional.empty(), TtlDirective.eternal())), id(2), 10, false);
    assertEquals("removed", completed(facade.invokeHelper(remove), queue).items().get(0).kind());
    assertEquals(0, facade.ownedBytes());
  }

  @Test void conflictingPendingHelperCannotStealTheOriginalReservation() {
    var service = owner(); var queue = new Queue(); var facade = new UnifiedIMapAsyncFacade(service, queue, () -> 0, 4);
    var first = facade.invokeHelper(new IMapLocalHelper.ExpireStep(1, id(1), 10)); queue.next();
    var conflict = facade.invokeHelper(new IMapLocalHelper.ExpireStep(2, id(1), 10)); queue.actions.removeLast().run();
    assertThrows(CompletionException.class, () -> conflict.result().toCompletableFuture().join());
    assertEquals("pending", service.queryOperationOutcome(id(1)).kind());
    assertEquals(0, completed(first, queue).expired()); assertEquals("completed", service.queryOperationOutcome(id(1)).kind());
    assertEquals(0, facade.pending()); assertEquals(0, facade.ownedBytes());
  }

  @Test void snapshotPrivacyAndEpochPreflightLeaveStickyGapAndReceiptOwnershipUnchanged() throws Exception {
    var service = owner(); var queue = new Queue(); var facade = new UnifiedIMapAsyncFacade(service, queue, () -> 0, 4);
    var restricted = service.registerListener(1, new UnifiedIMapService.ListenerOptions(true, 1, 16, Optional.empty(), java.util.Set.of()));
    set(service, "61", "3132", 1, TtlDirective.eternal()); var before = service.resourceUsage();
    var refused = facade.invokeHelper(new IMapLocalHelper.ListenerSnapshot(1, 16, 1024, id(1), 10)); queue.drain();
    var error = assertThrows(CompletionException.class, () -> refused.result().toCompletableFuture().join());
    assertEquals("resource_budget_exhausted", ((IMapDomainException) error.getCause()).kind());
    assertEquals(before, service.resourceUsage()); assertEquals("gap", restricted.drain().get(0).kind());
    var metadata = service.registerListener(2, UnifiedIMapService.ListenerOptions.metadata(16));
    var request = new IMapLocalHelper.ListenerSnapshot(2, 16, 1024, id(2), 10);
    try (var cut = completed(facade.invokeHelper(request), queue)) { assertFalse(cut.includeValue()); assertTrue(cut.entries().get(0).value().isEmpty()); }
    try (var cut = completed(facade.invokeHelper(request), queue)) { assertFalse(cut.includeValue()); assertTrue(cut.entries().get(0).value().isEmpty()); }
    var listeners = UnifiedIMapService.class.getDeclaredField("listeners"); listeners.setAccessible(true);
    Object state = ((java.util.Map<?, ?>) listeners.get(service)).get(2L);
    var epoch = state.getClass().getDeclaredField("deliveryEpoch"); epoch.setAccessible(true); epoch.setLong(state, Long.MAX_VALUE);
    var exhausted = facade.invokeHelper(new IMapLocalHelper.ListenerSnapshot(2, 16, 1024, id(3), 10)); queue.drain();
    var overflow = assertThrows(CompletionException.class, () -> exhausted.result().toCompletableFuture().join());
    assertEquals("capacity", ((IMapDomainException) overflow.getCause()).kind());
    assertEquals("unknown", service.queryOperationOutcome(id(3)).kind()); assertEquals(0, service.resourceUsage().resync());
    var limits = UnifiedIMapService.BudgetLimits.defaults();
    var oneItem = new UnifiedIMapService(new BoundedIMapApi.Bounds(16, 16, 32, 16, 32, 4096, 32), 1,
        new UnifiedIMapService.BudgetLimits(limits.values(), limits.outcomes(), limits.pending(), limits.listeners(),
            limits.streams(), limits.resync(), limits.maxClients(), 1, limits.expiryWork(), limits.queryBytes()));
    var boundedFacade = new UnifiedIMapAsyncFacade(oneItem, queue, () -> 0, 4);
    assertThrows(IMapDomainException.class, () -> boundedFacade.invokeHelper(new IMapLocalHelper.ListenerSnapshot(2, 2, 1024, id(4), 10)));
    assertEquals(0, boundedFacade.pending()); assertEquals(0, boundedFacade.ownedBytes());
    assertThrows(IMapDomainException.class, () -> facade.invokeHelper(new IMapLocalHelper.ListenerSnapshot(2, 16, Long.MAX_VALUE, id(4), 10)));
    restricted.close(); metadata.close();
  }

  @Test void independentAsyncFacadesShareTheActualServiceAggregateByteOwner() {
    var limits = UnifiedIMapService.BudgetLimits.defaults();
    var bounded = new UnifiedIMapService(new BoundedIMapApi.Bounds(16, 16, 32, 16, 32, 4096, 32), 1,
        new UnifiedIMapService.BudgetLimits(limits.values(), limits.outcomes(), limits.pending(), limits.listeners(), 400,
            limits.resync(), limits.maxClients(), limits.maxBatch(), limits.expiryWork(), limits.queryBytes()));
    var queue = new Queue(); var first = new UnifiedIMapAsyncFacade(bounded, queue, () -> 0, 4);
    var second = new UnifiedIMapAsyncFacade(bounded, queue, () -> 0, 4);
    var accepted = first.invokeHelper(new IMapLocalHelper.ExpireStep(1, id(1), 10));
    assertTrue(bounded.resourceUsage().streams() > 0);
    assertThrows(IMapDomainException.class, () -> second.invokeHelper(new IMapLocalHelper.ExpireStep(1, id(2), 10)));
    assertEquals(0, second.pending()); assertEquals(0, second.ownedBytes());
    assertTrue(accepted.cancel()); queue.drain(); assertEquals(0, bounded.resourceUsage().streams());
    completed(second.invokeHelper(new IMapLocalHelper.ExpireStep(1, id(2), 10)), queue);
    assertEquals(0, bounded.resourceUsage().streams()); assertEquals(0, bounded.diagnosticStats(0).activeStreams());
  }
}
