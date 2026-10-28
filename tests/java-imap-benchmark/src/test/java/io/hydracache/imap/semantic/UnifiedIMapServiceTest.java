package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;

import java.util.HashSet;
import java.util.List;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.Set;
import org.junit.jupiter.api.Test;

final class UnifiedIMapServiceTest {
  private static final BoundedIMapApi.Bounds BOUNDS = new BoundedIMapApi.Bounds(64, 64, 64, 64, 64, 4096, 64);
  private static BytesValue b(String value) { return BytesValue.fromHex(value); }
  private static BoundedIMapApi.MutationIdentity id(long value) { return new BoundedIMapApi.MutationIdentity("unified", value); }
  private static UnifiedIMapService.DomainCommand c(String op, long id) {
    return new UnifiedIMapService.DomainCommand(op).key(b("61")).identity(id(id));
  }
  private static BoundedIMapApi.ReplaceInput entry(String key, String value) {
    return new BoundedIMapApi.ReplaceInput(b(key), b(value));
  }

  @Test
  void allFortyOperationsUseOneStoreAndTypedResults() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    var observed = new HashSet<String>();
    long next = 1;
    var commands = List.of(
        c("get", next++), c("contains_key", next++),
        c("put", next++).value(b("31")), c("set", next++).value(b("32")),
        c("put_if_absent", next++).value(b("ff")),
        c("replace_if_present", next++).value(b("33")).ttl(TtlDirective.preserve()),
        c("conditional_replace", next++).expected(b("33")).value(b("34")),
        c("conditional_remove", next++).expected(b("00")),
        c("get_and_put", next++).value(b("35")), c("contains_entry", next++).expected(b("35")),
        c("get_or_default", next++).value(b("ff")), c("get_entry_view", next++),
        c("set_ttl", next++).ttl(TtlDirective.expireAfter(20)), c("remaining_ttl", next++),
        c("get_all", next++).keys(List.of(b("61"), b("62"))),
        c("put_all_detailed", next++).entries(List.of(entry("61", "36"), entry("62", "37"))),
        c("replace_all_detailed", next++).entries(List.of(entry("61", "38"), entry("63", "39"))),
        c("key_set_page", next++).limit(1), c("values_page", next++).limit(1),
        c("entry_set_page", next++).limit(64), c("size", next++), c("is_empty", next++),
        c("contains_value_bounded", next++).value(b("38")).limit(1),
        c("entry_listener", next++).listenerId(1).limit(64).includeValue(true),
        c("remove_entry_listener", next++).listenerId(1), c("capability_descriptor", next++),
        c("query_operation_outcome", next++).identity(id(3)), c("local_statistics", next++),
        c("try_put", next++).value(b("39")), c("try_remove", next++),
        c("remove_all_detailed", next++).keys(List.of(b("62"))),
        c("get_and_remove", next++).key(b("62")), c("delete", next++).key(b("62")),
        c("put", next++).value(b("31")));
    for (var command : commands) { observed.add(command.operation); assertNotNull(service.execute(command)); }
    var token = service.execute(c("get_entry_view", next++)).token().orElseThrow();
    var replaced = service.execute(c("replace_if_version", next++).token(token).value(b("32")));
    assertEquals("replaced", replaced.kind()); observed.add("replace_if_version");
    token = replaced.token().orElseThrow();
    var expiry = service.execute(c("set_ttl_if_version", next++).token(token).ttl(TtlDirective.expireAfter(20)));
    assertEquals(OptionalLong.of(20), expiry.remainingTtl()); observed.add("set_ttl_if_version");
    service.execute(c("remove_if_version", next++).token(expiry.token().orElseThrow()));
    observed.add("remove_if_version");
    for (String operation : List.of("evict", "clear_detailed", "evict_all_detailed", "destroy")) {
      if (!operation.equals("destroy")) service.execute(c("put", next++).value(b("31")));
      assertNotNull(service.execute(c(operation, next++))); observed.add(operation);
    }
    assertEquals(IMapCapabilityDescriptor.local(BOUNDS).plannedOperations(), observed);
    assertTrue(service.snapshot(0).isEmpty());
    assertEquals(0, service.stats(0).retainedOutcomes());
  }

  @Test
  void versionTokensRejectAbaAndWrongGeneration() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    var initial = service.execute(c("put", 1).value(b("31"))).token().orElseThrow();
    service.execute(c("set", 2).value(b("32")));
    service.execute(c("set", 3).value(b("31")));
    var mismatch = service.execute(c("replace_if_version", 4).token(initial).value(b("ff")));
    assertEquals("mismatch", mismatch.kind()); assertTrue(mismatch.value().isEmpty());
    var current = service.execute(c("get_entry_view", 5)).token().orElseThrow();
    assertEquals("mismatch", service.execute(c("remove_if_version", 6).token(new UnifiedIMapService.VersionToken(
        2, current.mapId(), current.version()))).kind());
    assertEquals("mismatch", service.execute(c("set_ttl_if_version", 7).token(new UnifiedIMapService.VersionToken(
        1, current.mapId() + 1, current.version())).ttl(TtlDirective.expireAfter(1))).kind());
    assertEquals(b("31"), service.snapshot(0).get(0).value());
    assertEquals("removed", service.execute(c("remove_if_version", 8).token(current)).kind());
    assertEquals("absent", service.execute(c("remove_if_version", 9).token(current)).kind());
    assertThrows(IMapDomainException.class, () -> new UnifiedIMapService.VersionToken(0, 1, 1));
  }

  @Test
  void outcomeQueriesDistinguishPendingCompletedAndUnknown() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    assertEquals("unknown", service.queryOperationOutcome(id(1)).kind());
    var command = c("put", 1).value(b("31"));
    service.beginOperation(command);
    assertEquals("pending", service.queryOperationOutcome(id(1)).kind());
    assertEquals(1, service.stats(0).inflight());
    assertEquals("mutation_identity_conflict", assertThrows(IMapDomainException.class,
        () -> service.beginOperation(c("put", 1).value(b("32")))).kind());
    assertEquals("pending", service.queryOperationOutcome(id(1)).kind());
    assertTrue(service.snapshot(0).isEmpty());
    var result = service.executeReserved(command);
    assertEquals(result, service.queryOperationOutcome(id(1)).result().orElseThrow());
    service.beginOperation(command);
    assertEquals("completed", service.queryOperationOutcome(id(1)).kind());
    assertEquals("completed", service.execute(c("query_operation_outcome", 1)).kind());
    long version = service.snapshot(0).get(0).version();
    assertEquals(result, service.execute(command.copy().deadline(0)));
    assertEquals(version, service.snapshot(0).get(0).version());
    assertEquals("mutation_identity_conflict", assertThrows(IMapDomainException.class,
        () -> service.execute(c("put", 1).value(b("32")))).kind());
    assertEquals("outcome_unknown", service.execute(c("set", 2).value(b("33")).loseResponse(true)).kind());
    assertEquals("applied", service.execute(c("set", 2).value(b("33"))).kind());
    var pending = c("set", 3).value(b("34")); service.beginOperation(pending);
    assertTrue(service.cancelReserved(id(3))); assertFalse(service.cancelReserved(id(3)));
    service.beginOperation(pending); service.abandonOperation(id(3));
    assertEquals(0, service.stats(0).inflight());
    assertEquals("not_applied", service.execute(c("try_put", 4).value(b("35")).deadline(0)).kind());
    assertEquals("unknown", service.queryOperationOutcome(id(4)).kind());
    assertThrows(IMapDomainException.class, () -> service.beginOperation(c("get", 5)));
    assertThrows(IMapDomainException.class, () -> service.executeReserved(c("set", 5).value(b("35"))));
  }

  @Test
  void listenerOptionsPreservePrivacyAndCleanup() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    var metadata = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(64));
    var values = service.registerListener(2, new UnifiedIMapService.ListenerOptions(true, 64, 64,
        Optional.empty(), Set.of()));
    service.execute(c("set", 1).value(b("31")).ttl(TtlDirective.expireAfter(2)));
    service.execute(c("set", 2).value(b("32")).ttl(TtlDirective.expireAfter(2)));
    assertTrue(metadata.drain().stream().allMatch(event -> event.oldValue().isEmpty() && event.value().isEmpty()));
    assertEquals(List.of("added", "updated"), values.drain().stream().map(UnifiedIMapService.Event::kind).toList());
    service.execute(c("get", 3).now(2));
    assertEquals("expired", values.drain().get(0).kind());
    service.execute(c("set", 4).value(b("33")).now(2));
    service.execute(c("evict", 5).now(2));
    service.execute(c("set", 6).value(b("34")).now(2));
    service.execute(c("clear_detailed", 7).now(2));
    assertTrue(values.drain().stream().anyMatch(event -> event.kind().equals("cleared")));
    service.execute(c("destroy", 8).now(2));
    assertEquals("destroyed", values.drain().get(0).kind());
    metadata.close(); values.close(); values.close();
    assertTrue(values.drain().isEmpty());
    assertEquals(0, service.stats(2).listeners());
    assertEquals(0, service.stats(2).queuedEvents());
    assertThrows(IMapDomainException.class, () -> service.registerListener(3, UnifiedIMapService.ListenerOptions.metadata(1)));
  }

  @Test
  void listenerOverflowValueBoundsAndFiltersAreExplicit() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    var overflow = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(1));
    var payload = service.registerListener(2, new UnifiedIMapService.ListenerOptions(true, 0, 2, Optional.empty(), Set.of()));
    var filtered = service.registerListener(3, new UnifiedIMapService.ListenerOptions(false, 0, 64, Optional.of(b("62")), Set.of("removed")));
    service.execute(c("set", 1).value(b("31")));
    service.execute(c("set", 2).value(b("32")));
    assertEquals("gap", overflow.drain().get(0).kind());
    assertEquals("gap", payload.drain().get(0).kind());
    service.execute(c("set", 3).value(b("33")));
    assertTrue(overflow.drain().isEmpty());
    service.repairListener(1);
    service.execute(c("set", 4).key(b("62")).value(b("34")));
    service.execute(c("delete", 5).key(b("62")));
    assertEquals(1, filtered.drain().size());
    assertThrows(IMapDomainException.class, () -> service.repairListener(9));
    assertThrows(IMapDomainException.class, () -> service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(1)));
    assertThrows(IMapDomainException.class, () -> service.registerListener(9, UnifiedIMapService.ListenerOptions.metadata(65)));
    assertThrows(IMapDomainException.class, () -> new UnifiedIMapService.ListenerOptions(false, 0, 0, Optional.empty(), Set.of()));
    filtered.close(); payload.close(); overflow.close();
  }

  @Test
  void localStatisticsRemainBoundedAndLocal() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    service.execute(c("set", 1).value(b("3132")));
    service.execute(c("set", 2).key(b("62")).value(b("33")));
    var stats = service.stats(0);
    assertEquals(2, stats.liveEntries()); assertEquals(3, stats.valueBytes());
    assertEquals(2, stats.retainedOutcomes()); assertEquals(2, stats.requests());
    assertEquals("unsupported_operation", assertThrows(IMapDomainException.class,
        () -> service.execute(c("sql", 3))).kind());
    assertEquals(1, service.stats(0).rejections());
    service.execute(c("clear_detailed", 4)); assertEquals(0, service.stats(0).valueBytes());
  }

  @Test
  void bulkAndPointErrorsFailBeforeChangingVisibleState() {
    var tiny = new BoundedIMapApi.Bounds(1, 2, 1, 1, 1, 64, 1);
    var service = new UnifiedIMapService(tiny, 1);
    service.execute(c("put", 1).value(b("31")));
    for (var command : List.of(c("put_all_detailed", 2).entries(List.of(entry("61", "32"), entry("62", "33"))),
        c("put_all_detailed", 3).entries(List.of(entry("61", "32"), entry("61", "33"))),
        c("put", 4).key(b("6162")).value(b("31")), c("put", 5).value(b("313233")),
        c("put", 6), c("put", 7).identity(id(7)).key(b("62")).value(b("31")).ttl(TtlDirective.preserve()),
        c("remove_if_version", 8), c("get_all", 9), c("get", 10).limit(0))) {
      assertThrows(IMapDomainException.class, () -> service.execute(command));
      assertEquals(b("31"), service.snapshot(0).get(0).value());
    }
    assertThrows(IMapDomainException.class, () -> service.execute(new UnifiedIMapService.DomainCommand("set").value(b("31"))));
    var terminal = new UnifiedIMapService(BOUNDS, 1);
    terminal.execute(c("destroy", 11));
    assertEquals("map_destroyed", assertThrows(IMapDomainException.class,
        () -> terminal.execute(c("get", 12))).kind());
  }

  @Test
  void adoptingExistingMapPreservesValuesAndCompletePointAndBulkReceipts() {
    var original = new BoundedIMapApi(BOUNDS);
    var lost = original.tryPut(id(1), b("61"), b("31"), null, 10, 0, true);
    assertEquals(BoundedIMapApi.TryStatus.OUTCOME_UNKNOWN, lost.status());
    var bulk = original.tryReplaceAllDetailed(id(2), List.of(entry("61", "32"), entry("62", "33")),
        10, 0, false);
    var service = new UnifiedIMapService(original, 1);
    assertEquals(b("32"), service.execute(c("get", 3)).value().orElseThrow());
    var point = service.queryOperationOutcome(id(1)).result().orElseThrow();
    assertEquals("applied", point.kind()); assertEquals(OptionalLong.of(2), point.version());
    var receipts = service.queryOperationOutcome(id(2)).result().orElseThrow();
    assertEquals("bulk", receipts.kind()); assertEquals(2, receipts.items().size());
    assertEquals(bulk.items().get(0).version(), receipts.items().get(0).version());
    assertEquals("absent", receipts.items().get(1).kind());
    service.execute(c("set", 4).value(b("34")));
    assertTrue(original.containsEntry(b("61"), b("34"), 0));
    original.set(b("61"), b("35"), null, 0);
    assertEquals(b("35"), service.execute(c("get", 5)).value().orElseThrow());
  }

  @Test
  void foreignRequestBindingTakesPrecedenceOverAnOtherwiseStaleCursor() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    service.execute(c("set", 1).value(b("31")));
    service.execute(c("set", 2).key(b("62")).value(b("32")));
    var cursor = service.execute(c("key_set_page", 3).limit(1)).cursor().orElseThrow();
    service.execute(c("set", 4).key(b("63")).value(b("33")));
    assertEquals("cursor_mismatch", assertThrows(IMapDomainException.class,
        () -> service.execute(c("values_page", 5).cursor(cursor).limit(1))).kind());
    assertEquals("scan_cursor_stale", assertThrows(IMapDomainException.class,
        () -> service.execute(c("key_set_page", 6).cursor(cursor).limit(1))).kind());
  }

  @Test
  void bulkEventsFollowInputOrderAndPreserveEachCommittedVersion() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    var listener = service.registerListener(1, new UnifiedIMapService.ListenerOptions(true, 64, 64,
        Optional.empty(), Set.of()));
    service.execute(c("put", 1).value(b("31")));
    service.execute(c("put", 2).key(b("62")).value(b("32")));
    listener.drain();
    var written = service.execute(c("put_all_detailed", 3)
        .entries(List.of(entry("62", "33"), entry("61", "34"))));
    var writes = listener.drain();
    assertEquals(List.of(b("62"), b("61")), writes.stream().map(UnifiedIMapService.Event::key).toList());
    assertEquals(List.of(4L, 5L), writes.stream().map(UnifiedIMapService.Event::version).toList());
    assertEquals(written.items().stream().map(item -> item.version().orElseThrow()).toList(),
        writes.stream().map(UnifiedIMapService.Event::version).toList());
    service.execute(c("remove_all_detailed", 4).keys(List.of(b("62"), b("61"))));
    var removals = listener.drain();
    assertEquals(List.of(b("62"), b("61")), removals.stream().map(UnifiedIMapService.Event::key).toList());
    assertEquals(List.of(6L, 7L), removals.stream().map(UnifiedIMapService.Event::version).toList());
    assertEquals(List.of(b("33"), b("34")), removals.stream().map(event -> event.oldValue().orElseThrow()).toList());
    listener.close();
  }

  @Test
  void conditionalRemoveReturnsPreviousValueAndEmptyClearHasNoInventedEvent() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    var listener = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(64));
    service.execute(c("put", 1).value(b("31")));
    var removed = service.execute(c("conditional_remove", 2).expected(b("31")));
    assertEquals(Optional.of(b("31")), removed.value());
    listener.drain();
    service.execute(c("clear_detailed", 3));
    assertTrue(listener.drain().isEmpty());
    service.execute(c("destroy", 4));
    var terminal = listener.drain();
    assertEquals(1, terminal.size()); assertEquals("destroyed", terminal.get(0).kind());
    assertEquals(4, terminal.get(0).version());
  }

  @Test
  void adoptedLegacyWritersCannotBypassPendingIdentityOrTotalDedupCapacity() {
    var backing = new BoundedIMapApi(new BoundedIMapApi.Bounds(8, 8, 1, 1, 1, 64, 1));
    var service = new UnifiedIMapService(backing, 1);
    service.beginOperation(c("set", 1).value(b("31")));
    assertThrows(IllegalArgumentException.class, () -> backing.tryPut(id(1), b("61"), b("32"), null, 10, 0, false));
    assertThrows(IllegalArgumentException.class, () -> backing.tryReplaceAllDetailed(id(1), List.of(entry("61", "32")), 10, 0, false));
    assertThrows(IllegalArgumentException.class, () -> backing.tryPut(id(2), b("61"), b("32"), null, 10, 0, false));
    assertThrows(IllegalArgumentException.class, () -> backing.tryReplaceAllDetailed(id(2), List.of(entry("61", "32")), 10, 0, false));
    assertEquals("pending", service.queryOperationOutcome(id(1)).kind());
    assertTrue(service.snapshot(0).isEmpty());
    service.cancelReserved(id(1));
    assertEquals(BoundedIMapApi.TryStatus.APPLIED, backing.tryPut(id(2), b("61"), b("32"), null, 10, 0, false).status());
  }

  @Test
  void cancellationAndPreflightFailuresAreTypedAndDoNotInventSuccessfulMutations() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    assertEquals("not_applied", service.execute(c("set", 1).value(b("31")).cancelled(true)).kind());
    assertTrue(service.snapshot(0).isEmpty()); assertEquals(1, service.stats(0).cancellations());
    assertEquals("advanced", service.execute(c("advance", 2).now(1)).kind());
    assertThrows(IMapDomainException.class, () -> service.executeReserved(c("set", 3).value(b("31")).now(1)));
    assertThrows(IMapDomainException.class, () -> service.execute(c("put_all_detailed", 4)
        .entries(List.of(entry("61", "31"))).ttl(TtlDirective.preserve()).now(1)));
    var tinyResponse = new UnifiedIMapService(new BoundedIMapApi.Bounds(8, 8, 8, 2, 8, 1, 8), 1);
    assertEquals("response_too_large", assertThrows(IMapDomainException.class,
        () -> tinyResponse.execute(c("set", 5).value(b("3132")))).kind());
    assertTrue(tinyResponse.snapshot(0).isEmpty());
    assertEquals("response_too_large", assertThrows(IMapDomainException.class,
        () -> tinyResponse.execute(c("put_all_detailed", 6).entries(List.of(entry("61", "3132"))))).kind());
    assertTrue(tinyResponse.snapshot(0).isEmpty());
  }

  @Test
  void timedRemovalReportsOnlyActualChangeAndRetainsItsExactOutcome() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    service.execute(c("try_put", 1).value(b("31")));
    var removed = service.execute(c("try_remove", 2));
    assertEquals("applied", removed.kind()); assertTrue(removed.flag());
    assertTrue(removed.version().isPresent()); assertTrue(removed.value().isEmpty());
    assertEquals(removed, service.executeReserved(c("try_remove", 2)));
    var absent = service.execute(c("try_remove", 3));
    assertEquals("applied", absent.kind()); assertFalse(absent.flag()); assertTrue(absent.version().isEmpty());
  }
}
