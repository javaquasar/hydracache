package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import org.junit.jupiter.api.Test;

final class ImmutableIMapRequestTest {
  private static BytesValue b(String value) { return BytesValue.fromHex(value); }
  private static UnifiedIMapService owner() { return new UnifiedIMapService(new BoundedIMapApi.Bounds(8, 8, 8, 8, 8, 128, 8), 1); }
  private static UnifiedIMapService.DomainCommand put(long sequence) {
    return new UnifiedIMapService.DomainCommand("set").key(b("61")).value(b("31"))
        .identity(new BoundedIMapApi.MutationIdentity("snapshot", sequence));
  }

  @Test void capturedRequestHasNoMutableBuilderListOrClockAliases() {
    var service = owner();
    var entries = new ArrayList<>(List.of(new BoundedIMapApi.ReplaceInput(b("61"), b("31"))));
    var builder = new UnifiedIMapService.DomainCommand("put_all_detailed")
        .identity(new BoundedIMapApi.MutationIdentity("snapshot", 1));
    builder.entries = entries;
    var request = service.prepareRequest(builder);
    entries.clear(); builder.entries = List.of(); builder.identity = Optional.of(new BoundedIMapApi.MutationIdentity("other", 2));
    builder.ttl = TtlDirective.expireAfter(9); builder.now = 50;
    assertEquals(1, service.executeSnapshot(request).items().size());
    assertEquals(b("31"), service.execute(new UnifiedIMapService.DomainCommand("get").key(b("61"))).value().orElseThrow());
    assertThrows(UnsupportedOperationException.class, () -> request.entries().clear());
    var thawed = request.toBuilder(); thawed.entries = List.of();
    assertEquals(1, request.entries().size());
    assertEquals(0, request.now()); assertEquals(2, request.atTime(2).now());
    var keys = new ArrayList<>(List.of(b("61"))); var read = new UnifiedIMapService.DomainCommand("get_all"); read.keys = keys;
    var stableRead = read.freeze(); keys.clear(); assertEquals(1, service.executeSnapshot(stableRead).items().size());
    assertThrows(UnsupportedOperationException.class, () -> stableRead.keys().clear());
  }

  @Test void reservationExecutionAndReplayUseExactlyTheCapturedDigest() {
    var service = owner(); var builder = put(1); var request = builder.freeze();
    service.beginSnapshot(request); builder.value(b("32")); builder.identity(new BoundedIMapApi.MutationIdentity("snapshot", 2));
    var result = service.executeReservedSnapshot(request);
    assertEquals(result, service.executeSnapshot(request.atTime(0)));
    assertEquals("unknown", service.queryOperationOutcome(builder.identity.orElseThrow()).kind());
    assertEquals("completed", service.queryOperationOutcome(request.identity().orElseThrow()).kind());
    var missing = put(3).freeze();
    assertEquals("invalid_argument", assertThrows(IMapDomainException.class, () -> service.executeReservedSnapshot(missing)).kind());
    assertEquals("mutation_identity_conflict", assertThrows(IMapDomainException.class,
        () -> service.executeSnapshot(request.toBuilder().value(b("ff")).freeze())).kind());
  }

  @Test void staticPreparationAndNullRejectionCannotAdvanceTimeOrRetainOutcomes() {
    var service = owner(); var before = service.resourceUsage();
    assertEquals("invalid_argument", assertThrows(IMapDomainException.class,
        () -> service.prepareRequest(put(1).ttl(TtlDirective.preserve()).now(5))).kind());
    assertEquals(before, service.resourceUsage()); assertEquals(0, service.diagnosticStats(0).requests());
    assertEquals("unknown", service.queryOperationOutcome(new BoundedIMapApi.MutationIdentity("snapshot", 1)).kind());
    assertThrows(NullPointerException.class, () -> service.execute(null));
    assertThrows(NullPointerException.class, () -> service.executeSnapshot(null));
    assertThrows(NullPointerException.class, () -> service.beginSnapshot(null));
    assertThrows(NullPointerException.class, () -> service.executeReservedSnapshot(null));
    var invalid = put(2); invalid.expected = null;
    assertThrows(NullPointerException.class, invalid::freeze);
    var invalidLists = put(2); invalidLists.keys = null;
    assertThrows(NullPointerException.class, invalidLists::freeze);
    assertEquals(put(2).freeze(), put(2).copy().freeze());
  }

  @Test void expiredOrCancelledConflictingSnapshotsCannotAbandonTheOriginalPendingRequest() {
    for (boolean reserved : List.of(false, true)) for (boolean cancelled : List.of(false, true)) {
      var service = owner();
      try (var listener = service.registerListener(1, new UnifiedIMapService.ListenerOptions(true, 8, 8, Optional.empty(), java.util.Set.of()))) {
        service.execute(put(10).ttl(TtlDirective.expireAfter(3))); listener.drain();
        var original = put(1).key(b("62")).deadline(20).freeze(); service.beginSnapshot(original);
        var before = service.resourceUsage();
        var conflict = original.toBuilder().value(b("32")).now(5).deadline(cancelled ? 20 : 5).cancelled(cancelled).freeze();
        var error = assertThrows(IMapDomainException.class, () -> {
          if (reserved) service.executeReservedSnapshot(conflict); else service.executeSnapshot(conflict);
        });
        assertEquals("mutation_identity_conflict", error.kind()); assertEquals(before, service.resourceUsage());
        assertEquals("pending", service.queryOperationOutcome(original.identity().orElseThrow()).kind());
        assertEquals("completed", service.queryOperationOutcome(put(10).identity.orElseThrow()).kind());
        assertTrue(listener.drain().isEmpty());
        var existing = service.execute(new UnifiedIMapService.DomainCommand("get_entry_view").key(b("61")).now(0));
        assertEquals(3, existing.remainingTtl().orElseThrow()); assertEquals(b("31"), existing.value().orElseThrow());
        assertEquals("applied", service.executeReservedSnapshot(original).kind());
        assertEquals("completed", service.queryOperationOutcome(original.identity().orElseThrow()).kind());
        assertEquals(0, service.diagnosticStats(0).inflight());
      }
    }
  }

  @Test void retainedCompletedSnapshotsRecoverBeforeTimeoutOrCancellationWithoutAdvancingTime() {
    for (boolean reserved : List.of(false, true)) for (boolean cancelled : List.of(false, true)) {
      var service = owner();
      try (var listener = service.registerListener(1, new UnifiedIMapService.ListenerOptions(true, 8, 8, Optional.empty(), java.util.Set.of()))) {
        var original = put(1).ttl(TtlDirective.expireAfter(3)).deadline(20).freeze();
        var receipt = service.executeSnapshot(original); listener.drain(); var before = service.resourceUsage();
        var retry = original.toBuilder().now(5).deadline(cancelled ? 20 : 5).cancelled(cancelled).freeze();
        assertEquals(receipt, reserved ? service.executeReservedSnapshot(retry) : service.executeSnapshot(retry));
        assertEquals(before, service.resourceUsage()); assertTrue(listener.drain().isEmpty());
        assertEquals(3, service.execute(new UnifiedIMapService.DomainCommand("get_entry_view").key(b("61")).now(0)).remainingTtl().orElseThrow());
        var conflict = retry.toBuilder().value(b("32")).freeze();
        assertEquals("mutation_identity_conflict", assertThrows(IMapDomainException.class, () -> {
          if (reserved) service.executeReservedSnapshot(conflict); else service.executeSnapshot(conflict);
        }).kind());
        assertEquals(before, service.resourceUsage()); assertEquals(0, service.diagnosticStats(0).inflight());
      }
    }
  }
}
