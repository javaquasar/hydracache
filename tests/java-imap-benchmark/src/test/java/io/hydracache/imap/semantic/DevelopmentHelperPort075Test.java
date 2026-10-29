package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.Set;
import java.util.concurrent.Executor;
import java.util.concurrent.atomic.AtomicLong;
import org.junit.jupiter.api.Test;

final class DevelopmentHelperPort075Test {
  static final class Queue implements Executor {
    final ArrayDeque<Runnable> tasks = new ArrayDeque<>();
    public void execute(Runnable task) { tasks.add(task); }
    void next() { tasks.removeFirst().run(); }
    void drain() { while (!tasks.isEmpty()) next(); }
  }
  static BytesValue b(String hex) { return BytesValue.fromHex(hex); }
  static BoundedIMapApi.MutationIdentity id(long seq) { return new BoundedIMapApi.MutationIdentity("consumer", seq); }
  static DevelopmentBackendPort075.Binding binding() { return new DevelopmentBackendPort075.Binding(1, 2, 3, 0, 4, 5, "consumer"); }
  static DevelopmentHelperPort075.Grant grant(long epoch) { return new DevelopmentHelperPort075.Grant(1, 2, 3, epoch, Set.of(DevelopmentHelperPort075.Right.values()), true, List.of()); }
  static UnifiedIMapService owner() { return new UnifiedIMapService(new BoundedIMapApi.Bounds(1024, 4096, 64, 64, 64, 65536, 64), 3); }
  static DevelopmentHelperPort075 port(UnifiedIMapService service, Queue queue, AtomicLong clock, DevelopmentHelperPort075.Grant grant) {
    return new DevelopmentHelperPort075(binding(), grant, service, queue, clock::get, 8, DevelopmentBackendPort075.Mode.LOCAL_REFERENCE);
  }
  static <T> UnifiedIMapAsyncFacade.HelperInvocation<T> start(DevelopmentHelperPort075 port, long seq, IMapLocalHelper.Command<T> command) {
    return port.invoke(port.prepare(binding(), id(seq), command));
  }
  static <T> T value(UnifiedIMapAsyncFacade.HelperInvocation<T> invocation, Queue queue) {
    queue.drain(); var result = invocation.result().toCompletableFuture().join();
    assertEquals(IMapLocalHelper.Kind.COMPLETED, result.kind()); return result.value().orElseThrow();
  }
  static void set(UnifiedIMapService service) {
    service.execute(new UnifiedIMapService.DomainCommand("set").key(b("61")).value(b("31"))
        .identity(new BoundedIMapApi.MutationIdentity("writer", 1)));
  }
  @Test void allSevenHelpersUseOriginalLedgerAndLeases() {
    var service = owner(); var queue = new Queue(); var clock = new AtomicLong(); var port = port(service, queue, clock, grant(8)); set(service);
    var keys = new ArrayList<>(List.of(b("61"))); var read = new IMapLocalHelper.EntryViews(keys, 4096, 10); keys.clear();
    var views = value(start(port, 1, read), queue); assertEquals(b("31"), views.get(0).value().orElseThrow());
    assertEquals("unknown", service.queryOperationOutcome(id(1)).kind());
    var entry = new UnifiedIMapService.VersionInput(b("61"), views.get(0).token().orElseThrow(), Optional.of(b("32")), TtlDirective.preserve());
    var command = new IMapLocalHelper.VersionBulk(UnifiedIMapService.VersionAction.REPLACE, List.of(entry), id(2), 10, false);
    var result = value(start(port, 2, command), queue); assertEquals("replaced", result.items().get(0).kind());
    assertEquals(result, value(start(port, 2, command), queue));
    assertEquals("completed", value(start(port, 3, new IMapLocalHelper.QueryOutcomes(List.of(id(2)), 4096, 10)), queue).get(0).status().kind());
    assertEquals(1, value(start(port, 4, new IMapLocalHelper.RetireOutcomes("writer", 3, 1, id(4), 10)), queue).removed());
    assertEquals(0, value(start(port, 5, new IMapLocalHelper.ExpireStep(1, id(5), 10)), queue).expired());
    var checkpointInvocation = start(port, 6, new IMapLocalHelper.Checkpoint(10));
    var checkpoint = value(checkpointInvocation, queue); checkpointInvocation.close(); checkpointInvocation.close();
    assertThrows(RuntimeException.class, () -> UnifiedIMapService.restore(checkpoint));
    var listener = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(16));
    var snapshotInvocation = start(port, 7, new IMapLocalHelper.ListenerSnapshot(1, 16, 4096, id(7), 10));
    var snapshot = value(snapshotInvocation, queue); assertEquals(1, snapshot.entries().size());
    assertTrue(service.resourceUsage().resync() > 0); snapshotInvocation.close(); snapshot.close(); assertEquals(0, service.resourceUsage().resync()); listener.close();
  }
  @Test void prepareIsPureAndContextGrantsBoundsAndProductionAreFenced() {
    var service = owner(); var queue = new Queue(); var clock = new AtomicLong(); var port = port(service, queue, clock, grant(8));
    var before = service.resourceUsage(); var command = new IMapLocalHelper.Checkpoint(10);
    var prepared = port.prepare(binding(), id(1), command); assertEquals(before, service.resourceUsage()); assertEquals(0, clock.get()); assertTrue(queue.tasks.isEmpty());
    var other = port(service, queue, clock, grant(9)); assertThrows(SecurityException.class, () -> other.invoke(prepared));
    var bad = new DevelopmentBackendPort075.Binding(1, 2, 3, 0, 4, 6, "consumer");
    assertThrows(DevelopmentBackendPort075.BoundaryException.class, () -> port.prepare(bad, id(1), command));
    assertThrows(DevelopmentBackendPort075.BoundaryException.class, () -> port.prepare(binding(), new BoundedIMapApi.MutationIdentity("other", 1), command));
    assertThrows(DevelopmentBackendPort075.BoundaryException.class, () -> port.prepare(binding(), id(1), new IMapLocalHelper.ExpireStep(1, id(2), 10)));
    assertThrows(RuntimeException.class, () -> port.prepare(binding(), id(1), new IMapLocalHelper.ExpireStep(65, id(1), 10)));
    var scoped = new DevelopmentHelperPort075.Grant(1, 2, 3, 8, Set.of(DevelopmentHelperPort075.Right.READ), false, List.of(b("61")));
    var reader = port(service, queue, clock, scoped);
    assertNotNull(reader.prepare(binding(), id(1), new IMapLocalHelper.EntryViews(List.of(b("61")), 4096, 10)));
    assertThrows(SecurityException.class, () -> reader.prepare(binding(), id(1), new IMapLocalHelper.EntryViews(List.of(b("62")), 4096, 10)));
    assertThrows(SecurityException.class, () -> reader.prepare(binding(), id(1), new IMapLocalHelper.QueryOutcomes(List.of(id(1)), 4096, 10)));
    assertThrows(SecurityException.class, () -> reader.prepare(binding(), id(1), command));
    assertThrows(RuntimeException.class, () -> new DevelopmentHelperPort075(binding(), grant(8), service, queue, clock::get, 1, DevelopmentBackendPort075.Mode.PRODUCTION_MEMBER));
    assertThrows(SecurityException.class, () -> port(service, queue, clock, new DevelopmentHelperPort075.Grant(9, 2, 3, 8, Set.of(), true, List.of())));
    assertEquals(before, service.resourceUsage());
  }
  @Test void cancellationDeadlineAndRetainedReplayKeepAdmissionCertainty() {
    var service = owner(); var queue = new Queue(); var clock = new AtomicLong(); var port = port(service, queue, clock, grant(8));
    var cancelled = start(port, 1, new IMapLocalHelper.Checkpoint(10)); cancelled.cancel(); queue.drain();
    assertEquals(IMapLocalHelper.Kind.NOT_APPLIED, cancelled.result().toCompletableFuture().join().kind());
    clock.set(10); var expired = start(port, 2, new IMapLocalHelper.Checkpoint(10)); queue.drain();
    assertEquals(IMapLocalHelper.Kind.NOT_APPLIED, expired.result().toCompletableFuture().join().kind()); clock.set(0);
    set(service); var token = service.execute(new UnifiedIMapService.DomainCommand("get_entry_view").key(b("61"))).token().orElseThrow();
    var command = new IMapLocalHelper.VersionBulk(UnifiedIMapService.VersionAction.REPLACE, List.of(new UnifiedIMapService.VersionInput(b("61"), token, Optional.of(b("32")), TtlDirective.preserve())), id(3), 10, false);
    var admitted = start(port, 3, command); queue.next(); admitted.cancel(); assertEquals(IMapLocalHelper.Kind.OUTCOME_UNKNOWN, admitted.result().toCompletableFuture().join().kind()); queue.drain();
    var receipt = service.queryOperationOutcome(id(3)).result().orElseThrow(); assertEquals(receipt, value(start(port, 3, command), queue));
    var stale = new IMapLocalHelper.VersionBulk(command.action(), command.entries(), id(4), 10, false);
    assertEquals("mismatch", value(start(port, 4, stale), queue).items().get(0).kind());
    assertEquals(0, service.resourceUsage().streams());
  }
}
