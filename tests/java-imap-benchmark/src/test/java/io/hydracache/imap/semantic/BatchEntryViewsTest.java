package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.Executor;
import org.junit.jupiter.api.Test;

final class BatchEntryViewsTest {
  private static BytesValue b(String value) { return BytesValue.fromHex(value); }
  private static UnifiedIMapService owner() { return new UnifiedIMapService(new BoundedIMapApi.Bounds(16, 16, 32, 16, 32, 4096, 32), 7); }
  private static void put(UnifiedIMapService owner, String key, long sequence, TtlDirective ttl) { owner.execute(new UnifiedIMapService.DomainCommand("put").key(b(key)).value(b("76")).ttl(ttl).identity(new BoundedIMapApi.MutationIdentity("writer", sequence))); }

  @Test void orderedViewsIncludeAbsentDuplicateEntriesAndUsableFullTokens() {
    var service = owner(); put(service, "61", 1, TtlDirective.expireAfter(10));
    var views = service.entryViews(List.of(b("61"), b("6d697373"), b("61")), 0, 248);
    assertEquals(List.of(0, 1, 2), views.stream().map(UnifiedIMapService.VersionedEntryView::index).toList());
    assertEquals(b("76"), views.get(0).value().orElseThrow());
    assertEquals(10, views.get(0).remainingTtl().orElseThrow());
    assertEquals(7, views.get(0).token().orElseThrow().namespaceGeneration());
    assertEquals(views.get(0).token(), views.get(2).token());
    assertTrue(views.get(1).value().isEmpty()); assertTrue(views.get(1).token().isEmpty()); assertTrue(views.get(1).remainingTtl().isEmpty());
    var token = views.get(0).token().orElseThrow();
    var replaced = service.execute(new UnifiedIMapService.DomainCommand("replace_if_version").key(b("61")).value(b("6e657874")).token(token).identity(new BoundedIMapApi.MutationIdentity("cas", 1)));
    assertEquals("replaced", replaced.kind());
    assertEquals("mismatch", service.execute(new UnifiedIMapService.DomainCommand("replace_if_version").key(b("61")).value(b("6e657874")).token(token).identity(new BoundedIMapApi.MutationIdentity("cas", 2))).kind());
    var other = owner(); put(other, "61", 1, TtlDirective.eternal());
    assertNotEquals(token.mapId(), other.entryViews(List.of(b("61")), 0, 1024).get(0).token().orElseThrow().mapId());
  }

  @Test void fullBatchBudgetsRejectBeforeTimeEventsAndOwnersChange() {
    var service = owner(); put(service, "61", 1, TtlDirective.expireAfter(1));
    var listener = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(8));
    var resources = service.resourceUsage(); var state = service.snapshot(0);
    assertEquals(IMapDomainException.Kind.AGGREGATE_BYTE_BUDGET, assertThrows(IMapDomainException.class, () -> service.entryViews(List.of(b("61")), 1, 1)).error());
    assertEquals(resources, service.resourceUsage()); assertEquals(state, service.snapshot(0)); assertTrue(listener.drain().isEmpty());
    assertThrows(IMapDomainException.class, () -> service.entryViews(List.of(), 0, 1024));
    assertThrows(IMapDomainException.class, () -> service.entryViews(java.util.Collections.nCopies(257, b("61")), 0, 1024));
    assertThrows(IMapDomainException.class, () -> service.entryViews(List.of(b("61")), 0, 0));
    assertThrows(IMapDomainException.class, () -> service.entryViews(List.of(b("61")), 0, 4097));
    assertEquals(IMapDomainException.Kind.KEY_TOO_LARGE, assertThrows(IMapDomainException.class, () -> service.entryViews(List.of(BytesValue.copyOf(new byte[17])), 0, 1024)).error());
    assertThrows(IMapDomainException.class, () -> service.entryViews(List.of(b("61")), -1, 1024));
    assertTrue(service.entryViews(List.of(b("61")), 1, 1024).get(0).value().isEmpty());
    assertEquals("expired", listener.drain().get(0).kind());
    assertEquals(IMapDomainException.Kind.CLOCK_REGRESSED, assertThrows(IMapDomainException.class, () -> service.entryViews(List.of(b("61")), 0, 1024)).error());
    service.execute(new UnifiedIMapService.DomainCommand("destroy").now(1).identity(new BoundedIMapApi.MutationIdentity("destroy", 1)));
    assertEquals(IMapDomainException.Kind.MAP_DESTROYED, assertThrows(IMapDomainException.class, () -> service.entryViews(List.of(b("61")), 1, 1024)).error());
    listener.close();
  }

  private static final class Queue implements Executor { final ArrayDeque<Runnable> tasks = new ArrayDeque<>(); public void execute(Runnable task) { tasks.add(task); } void drain() { while (!tasks.isEmpty()) tasks.removeFirst().run(); } }
  @Test void asynchronousViewsFreezeKeysAndRemainReadHelpersWithoutMutationIdentity() {
    var service = owner(); put(service, "61", 1, TtlDirective.eternal());
    var queue = new Queue(); var facade = new UnifiedIMapAsyncFacade(service, queue, () -> 0, 2);
    var keys = new ArrayList<>(List.of(b("61"))); var request = new IMapLocalHelper.EntryViews(keys, 1024, 10); keys.clear();
    long receipts = service.diagnosticStats(0).retainedOutcomes();
    var cancelled = facade.invokeHelper(request); assertTrue(cancelled.cancel()); queue.drain();
    assertEquals(IMapLocalHelper.Kind.NOT_APPLIED, cancelled.result().toCompletableFuture().join().kind());
    var invocation = facade.invokeHelper(request); queue.drain();
    var result = invocation.result().toCompletableFuture().join(); assertEquals(IMapLocalHelper.Kind.COMPLETED, result.kind());
    assertTrue(result.value().orElseThrow().get(0).token().isPresent());
    assertEquals(receipts, service.diagnosticStats(0).retainedOutcomes());
    assertEquals(0, facade.pending()); assertEquals(0, facade.ownedBytes()); assertEquals(0, service.resourceUsage().streams());
    assertThrows(IllegalArgumentException.class, () -> new IMapLocalHelper.EntryViews(List.of(), 1, 10));
    assertThrows(IllegalArgumentException.class, () -> new IMapLocalHelper.EntryViews(List.of(b("61")), 0, 10));
  }

  @Test void checkpointWaitsForSnapshotOwnersEvenWhenTheirPayloadIsEmpty() {
    var service = owner();
    var listener = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(8));
    var empty = service.snapshotListener(1, 8, 1024, 0); listener.close();
    assertTrue(empty.entries().isEmpty()); assertEquals(0, service.resourceUsage().resync());
    assertEquals(IMapDomainException.Kind.CHECKPOINT_BUSY, assertThrows(IMapDomainException.class, service::checkpoint).error());
    empty.close(); empty.close();
    try (var checkpoint = service.checkpoint()) { assertEquals(IMapDomainException.Kind.CHECKPOINT_BUSY, assertThrows(IMapDomainException.class, service::checkpoint).error()); }
    assertEquals(0, service.resourceUsage().resync());
    put(service, "61", 1, TtlDirective.eternal());
    var nonempty = service.registerListener(2, UnifiedIMapService.ListenerOptions.metadata(8));
    var held = service.snapshotListener(2, 8, 1024, 0); nonempty.close();
    assertEquals(IMapDomainException.Kind.CHECKPOINT_BUSY, assertThrows(IMapDomainException.class, service::checkpoint).error());
    held.close(); try (var checkpoint = service.checkpoint()) { assertTrue(service.resourceUsage().resync() > 0); }
  }

  @Test void entryViewRecordsRejectInconsistentPresenceShapes() {
    var token = new UnifiedIMapService.VersionToken(1, 1, 1);
    assertThrows(IMapDomainException.class, () -> new UnifiedIMapService.VersionedEntryView(-1, b("61"), java.util.Optional.empty(), java.util.Optional.empty(), java.util.OptionalLong.empty()));
    assertThrows(IMapDomainException.class, () -> new UnifiedIMapService.VersionedEntryView(0, b("61"), java.util.Optional.of(b("76")), java.util.Optional.empty(), java.util.OptionalLong.empty()));
    assertThrows(IMapDomainException.class, () -> new UnifiedIMapService.VersionedEntryView(0, b("61"), java.util.Optional.empty(), java.util.Optional.of(token), java.util.OptionalLong.empty()));
    assertThrows(IMapDomainException.class, () -> new UnifiedIMapService.VersionedEntryView(0, b("61"), java.util.Optional.empty(), java.util.Optional.empty(), java.util.OptionalLong.of(1)));
  }

  @Test void emptySnapshotLeaseCapacityAndRetainedReplayUseTheSameOwnerCount() {
    var service = new UnifiedIMapService(new BoundedIMapApi.Bounds(16, 16, 1, 1, 1, 4096, 1), 1);
    var listener = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(1));
    var held = service.snapshotListener(1, 1, 64, 0);
    assertEquals(IMapDomainException.Kind.CAPACITY, assertThrows(IMapDomainException.class, () -> service.snapshotListener(1, 1, 64, 0)).error());
    held.close();
    var queue = new Queue(); var facade = new UnifiedIMapAsyncFacade(service, queue, () -> 0, 4);
    var request = new IMapLocalHelper.ListenerSnapshot(1, 1, 64, new BoundedIMapApi.MutationIdentity("snapshot", 1), 10);
    var first = facade.invokeHelper(request); queue.drain();
    var snapshot = first.result().toCompletableFuture().join().value().orElseThrow();
    var replay = facade.invokeHelper(request); queue.drain();
    assertThrows(java.util.concurrent.CompletionException.class, () -> replay.result().toCompletableFuture().join());
    assertEquals("completed", service.queryOperationOutcome(request.identity()).kind());
    snapshot.close();
    var released = facade.invokeHelper(request); queue.drain(); released.result().toCompletableFuture().join().value().orElseThrow().close();
    listener.close(); assertEquals(0, service.resourceUsage().resync());
  }

  @Test void adoptedAliasCannotChangePayloadBetweenBatchAdmissionAndItsReadCut() throws Exception {
    var backing = new BoundedIMapApi(new BoundedIMapApi.Bounds(16, 32, 16, 16, 32, 4096, 32));
    backing.put(b("61"), b("30"), null, 0); backing.put(b("62"), b("30"), null, 0);
    var service = new UnifiedIMapService(backing, 1);
    var started = new java.util.concurrent.atomic.AtomicBoolean();
    var failure = new java.util.concurrent.atomic.AtomicReference<Throwable>();
    var writer = new Thread(() -> {
      try { backing.put(b("61"), BytesValue.copyOf(new byte[32]), null, 0); backing.put(b("62"), BytesValue.copyOf(new byte[32]), null, 0); }
      catch (Throwable error) { failure.set(error); }
    }, "entry-view-adopted-alias");
    writer.setDaemon(true);
    var keys = new java.util.AbstractList<BytesValue>() {
      @Override public int size() { return 2; }
      @Override public BytesValue get(int index) {
        if (started.compareAndSet(false, true)) {
          writer.start(); long deadline = System.nanoTime() + java.util.concurrent.TimeUnit.SECONDS.toNanos(2);
          while (writer.isAlive() && writer.getState() != Thread.State.BLOCKED && System.nanoTime() < deadline) Thread.yield();
          assertTrue(!writer.isAlive() || writer.getState() == Thread.State.BLOCKED, "alias writer must reach its actual map boundary");
        }
        return index == 0 ? b("61") : b("62");
      }
    };
    try {
      // Exactly the old payload fits. A writer winning during admission would exceed this bound.
      var views = service.entryViews(keys, 0, 180);
      assertEquals(List.of(b("30"), b("30")), views.stream().map(view -> view.value().orElseThrow()).toList());
      assertEquals(List.of(2L, 3L), views.stream().map(view -> view.token().orElseThrow().version()).toList());
    } finally { writer.join(2000); }
    assertFalse(writer.isAlive()); assertNull(failure.get());
    assertTrue(service.entryViews(List.of(b("61"), b("62")), 0, 1024).stream().allMatch(view -> view.value().orElseThrow().size() == 32));
  }
}
