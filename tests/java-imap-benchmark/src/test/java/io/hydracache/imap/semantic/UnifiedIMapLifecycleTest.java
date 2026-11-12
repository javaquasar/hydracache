package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;

import java.util.List;
import java.util.Optional;
import java.util.Set;
import org.junit.jupiter.api.Test;

final class UnifiedIMapLifecycleTest {
  private static final BoundedIMapApi.Bounds BOUNDS = new BoundedIMapApi.Bounds(128, 256, 64, 64, 64, 4096, 64);
  private static BytesValue b(String hex) { return BytesValue.fromHex(hex); }
  private static BoundedIMapApi.MutationIdentity id(long sequence) { return new BoundedIMapApi.MutationIdentity("c", sequence); }
  private static UnifiedIMapService.DomainCommand set(long sequence, String key, String value) {
    return new UnifiedIMapService.DomainCommand("set").identity(id(sequence)).key(b(key)).value(b(value));
  }
  private static UnifiedIMapService.VersionToken token(UnifiedIMapService service, String key, long now) {
    return service.execute(new UnifiedIMapService.DomainCommand("get_entry_view").key(b(key)).now(now)).token().orElseThrow();
  }
  private static UnifiedIMapService.BudgetLimits limits(long values, long outcomes, long pending,
      long listeners, long streams, long resync, int clients, int work) {
    return new UnifiedIMapService.BudgetLimits(values, outcomes, pending, listeners, streams, resync, clients, 64, work, 4096);
  }
  private static UnifiedIMapService.BudgetLimits roomy() { return limits(10000, 10000, 10000, 10000, 10000, 10000, 64, 1); }
  private static void error(String kind, org.junit.jupiter.api.function.Executable action) {
    assertEquals(kind, assertThrows(IMapDomainException.class, action).kind());
  }

  @Test
  void retirementFencesRejectOldRetriesAndProtectPending() {
    var backing = new BoundedIMapApi(BOUNDS); var service = new UnifiedIMapService(backing, 1, roomy());
    assertEquals("outcome_unknown", service.execute(set(1, "61", "31").loseResponse(true)).kind());
    service.beginOperation(set(2, "62", "32"));
    error("pending_retirement", () -> service.retireOutcomes("c", 1, 2));
    assertEquals("completed", service.queryOperationOutcome(id(1)).kind());
    assertEquals(1, service.retireOutcomes("c", 1, 1).removed());
    assertEquals("unknown", service.queryOperationOutcome(id(1)).kind());
    error("retired_identity", () -> service.execute(set(1, "61", "31")));
    error("retired_identity", () -> service.beginOperation(set(1, "61", "31")));
    assertThrows(IllegalArgumentException.class, () -> backing.tryRemove(id(1), b("61"), 10, 0, false));
    assertTrue(service.cancelReserved(id(2))); assertEquals(0, service.retireOutcomes("c", 1, 1).removed());
    error("invalid_argument", () -> service.retireOutcomes("c", 2, 2));
    service.retireOutcomes("c", 1, 3);
    error("invalid_argument", () -> service.retireOutcomes("c", 1, 2));
    service.execute(set(4, "61", "32"));
    assertEquals(b("32"), service.diagnosticSnapshot(0).get(0).value());
    var bounded = new UnifiedIMapService(BOUNDS, 1, limits(1000, 1000, 1000, 1000, 10000, 1000, 1, 1));
    bounded.retireOutcomes("first", 1, 1);
    error("resource_budget_exhausted", () -> bounded.retireOutcomes("second", 1, 1));
    assertThrows(IllegalArgumentException.class, () -> new BoundedIMapApi.MutationIdentity("é".repeat(65), 1));
  }

  @Test
  void aggregateBudgetsPreflightEveryOwnerAndReleaseExactlyOnce() {
    var backing = new BoundedIMapApi(BOUNDS);
    var service = new UnifiedIMapService(backing, 1, limits(26, 1000, 49, 128, 4160, 1000, 8, 1));
    service.execute(set(1, "61", "31")); assertEquals(26, service.resourceUsage().values());
    error("resource_budget_exhausted", () -> service.execute(set(2, "62", "32")));
    assertThrows(IllegalArgumentException.class, () -> backing.set(b("62"), b("32"), null, 0));
    assertEquals(1, service.diagnosticSnapshot(0).size());
    service.beginOperation(set(2, "61", "32"));
    assertEquals(49, service.resourceUsage().pending());
    error("resource_budget_exhausted", () -> service.beginOperation(set(3, "61", "33")));
    service.cancelReserved(id(2)); assertEquals(0, service.resourceUsage().pending());
    var listener = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(1));
    service.execute(set(4, "61", "34")); assertEquals("gap", listener.drain().get(0).kind());
    assertEquals(128, service.resourceUsage().listeners());
    error("resource_budget_exhausted", () -> service.registerListener(2, UnifiedIMapService.ListenerOptions.metadata(1)));
    listener.close(); assertEquals(0, service.resourceUsage().listeners());
    var pages = service.pages(BoundedIMapApi.Projection.KEYS, 1, () -> 0);
    assertEquals(4160, service.resourceUsage().streams());
    error("resource_budget_exhausted", () -> service.pages(BoundedIMapApi.Projection.KEYS, 1, () -> 0));
    pages.close(); pages.close(); assertEquals(0, service.resourceUsage().streams());
    var outcomes = new UnifiedIMapService(BOUNDS, 1, limits(1000, 113, 1000, 1000, 10000, 1000, 8, 1));
    outcomes.execute(set(1, "61", "31")); assertEquals(113, outcomes.resourceUsage().outcomes());
    error("resource_budget_exhausted", () -> outcomes.execute(set(2, "61", "32")));
    assertEquals(b("31"), outcomes.diagnosticSnapshot(0).get(0).value());
    outcomes.retireOutcomes("c", 1, 1); assertEquals(25, outcomes.resourceUsage().outcomes());
    var reservations = new UnifiedIMapService(BOUNDS, 1, limits(1000, 225, 1000, 1000, 10000, 1000, 8, 1));
    reservations.beginOperation(set(1, "61", "31"));
    error("resource_budget_exhausted", () -> reservations.beginOperation(set(2, "62", "32")));
    reservations.cancelReserved(id(1)); assertEquals(0, reservations.resourceUsage().outcomes());
    error("invalid_argument", () -> limits(0, 1, 1, 1, 1, 1, 1, 1));
    error("invalid_argument", () -> new UnifiedIMapService.BudgetLimits(1, 1, 1, 1, 1, 1, 1, 1, 1, 0));
  }

  @Test
  void repairSnapshotCutsOverAtomicallyOrKeepsGap() {
    var service = new UnifiedIMapService(BOUNDS, 1, limits(1000, 1000, 1000, 1000, 10000, 68, 8, 1));
    var listener = service.registerListener(1, new UnifiedIMapService.ListenerOptions(true, 16, 1, Optional.empty(), Set.of()));
    service.execute(set(1, "61", "31")); service.execute(set(2, "62", "32"));
    error("resource_budget_exhausted", () -> service.snapshotListener(1, 1, 68, 0));
    assertEquals("gap", listener.drain().get(0).kind());
    var snapshot = service.snapshotListener(1, 2, 68, 0);
    assertEquals(List.of(b("61"), b("62")), snapshot.entries().stream().map(UnifiedIMapService.ResyncEntry::key).toList());
    assertEquals(Optional.of(b("31")), snapshot.entries().get(0).value());
    assertEquals(68, service.resourceUsage().resync());
    error("resource_budget_exhausted", () -> service.snapshotListener(1, 2, 68, 0));
    service.execute(set(3, "61", "33")); assertTrue(listener.drain().get(0).watermark() > snapshot.watermark());
    snapshot.close(); snapshot.close(); assertEquals(0, service.resourceUsage().resync());
    listener.close();
    var metadata = service.registerListener(2, new UnifiedIMapService.ListenerOptions(false, 0, 1, Optional.of(b("61")), Set.of()));
    try (var filtered = service.snapshotListener(2, 1, 33, 0)) {
      assertEquals(1, filtered.entries().size()); assertTrue(filtered.entries().get(0).value().isEmpty());
    }
    error("invalid_argument", () -> service.snapshotListener(2, 0, 33, 0));
    metadata.close(); error("unknown_listener", () -> service.snapshotListener(2, 1, 33, 0));
  }

  @Test
  void versionBulkKeepsOrderedIndependentReceiptsAndReplay() {
    var service = new UnifiedIMapService(BOUNDS, 1, roomy());
    service.execute(set(1, "61", "31")); service.execute(set(2, "62", "32"));
    var a = token(service, "61", 0); var oldB = token(service, "62", 0);
    service.execute(set(3, "62", "33"));
    var inputs = List.of(new UnifiedIMapService.VersionInput(b("61"), a, Optional.of(b("34")), TtlDirective.preserve()),
        new UnifiedIMapService.VersionInput(b("62"), oldB, Optional.of(b("35")), TtlDirective.eternal()),
        new UnifiedIMapService.VersionInput(b("63"), a, Optional.of(b("36")), TtlDirective.eternal()));
    assertEquals("outcome_unknown", service.versionBulk(UnifiedIMapService.VersionAction.REPLACE, inputs, id(4), 0, 10, true).kind());
    var result = service.versionBulk(UnifiedIMapService.VersionAction.REPLACE, inputs, id(4), 1, 10, false);
    assertEquals(List.of("replaced", "mismatch", "absent"), result.items().stream().map(UnifiedIMapService.Item::kind).toList());
    assertEquals(result, service.queryOperationOutcome(id(4)).result().orElseThrow());
    assertTrue(result.items().stream().allMatch(item -> item.value().isEmpty()));
    var ttlInput = List.of(new UnifiedIMapService.VersionInput(b("61"), token(service, "61", 0), Optional.empty(), TtlDirective.expireAfter(5)));
    service.versionBulk(UnifiedIMapService.VersionAction.SET_TTL, ttlInput, id(5), 0, 10, false);
    assertEquals(5, service.diagnosticSnapshot(0).get(0).remainingTtl().orElseThrow());
    var remove = List.of(new UnifiedIMapService.VersionInput(b("61"), token(service, "61", 0), Optional.empty(), TtlDirective.preserve()));
    assertEquals("removed", service.versionBulk(UnifiedIMapService.VersionAction.REMOVE, remove, id(6), 0, 10, false).items().get(0).kind());
    error("mutation_identity_conflict", () -> service.versionBulk(UnifiedIMapService.VersionAction.REMOVE, remove, id(4), 0, 10, false));
    error("invalid_argument", () -> service.versionBulk(UnifiedIMapService.VersionAction.REMOVE, List.of(remove.get(0), remove.get(0)), id(7), 0, 10, false));
    error("invalid_argument", () -> service.versionBulk(UnifiedIMapService.VersionAction.REPLACE, remove, id(7), 0, 10, false));
    assertEquals("not_applied", service.versionBulk(UnifiedIMapService.VersionAction.REMOVE, remove, id(7), 0, 0, false).kind());
  }

  @Test
  void batchOutcomeQueryBoundsAndPreservesExactResults() {
    var service = new UnifiedIMapService(BOUNDS, 1, roomy());
    var original = service.execute(set(1, "61", "31")); service.beginOperation(set(2, "62", "32"));
    var result = service.queryOutcomes(List.of(id(2), id(1), id(3), id(1)), 4096);
    assertEquals(List.of("pending", "completed", "unknown", "completed"), result.stream().map(item -> item.status().kind()).toList());
    assertEquals(original, result.get(1).status().result().orElseThrow());
    assertEquals(3, result.get(3).index()); assertEquals(1, service.diagnosticSnapshot(0).size());
    error("resource_budget_exhausted", () -> service.queryOutcomes(List.of(id(1)), 64));
    error("invalid_argument", () -> service.queryOutcomes(List.of(), 4096));
    error("invalid_argument", () -> service.queryOutcomes(List.of(id(1)), 0));
    error("invalid_argument", () -> service.queryOutcomes(List.of(id(1)), 4097));
    service.cancelReserved(id(2)); service.retireOutcomes("c", 1, 1);
    assertEquals("unknown", service.queryOutcomes(List.of(id(1)), 4096).get(0).status().kind());
  }

  @Test
  void incrementalExpiryBoundsWorkAndNeverExposesExpiredEntries() {
    var service = new UnifiedIMapService(BOUNDS, 1, roomy());
    var listener = service.registerListener(1, new UnifiedIMapService.ListenerOptions(true, 16, 64, Optional.empty(), Set.of()));
    for (int index = 0; index < 5; index++) service.execute(set(index + 1, String.format("%02x", 97 + index), "31").ttl(TtlDirective.expireAfter(1)));
    listener.drain(); long revision = token(service, "61", 0).version();
    assertTrue(service.diagnosticSnapshot(1).isEmpty());
    assertEquals(5 * 26, service.resourceUsage().values());
    var first = service.expireStep(1, 1); assertEquals(1, first.work()); assertEquals(1, first.expired()); assertTrue(first.remaining());
    assertEquals(4 * 26, service.resourceUsage().values());
    assertEquals("absent", service.execute(new UnifiedIMapService.DomainCommand("get").key(b("65")).now(1)).kind());
    assertTrue(service.execute(new UnifiedIMapService.DomainCommand("key_set_page").now(1).limit(64)).items().isEmpty());
    var rest = service.expireStep(1, 64); assertEquals(2, rest.work()); assertFalse(rest.remaining());
    assertEquals(0, service.expireStep(1, 1).expired());
    var events = listener.drain(); assertEquals(5, events.size()); assertTrue(events.stream().allMatch(event -> event.kind().equals("expired")));
    assertEquals(1, events.stream().map(UnifiedIMapService.Event::version).distinct().count());
    assertTrue(events.get(0).version() > revision); assertEquals(0, service.resourceUsage().values());
    error("invalid_argument", () -> service.expireStep(1, 0)); error("clock_regressed", () -> service.expireStep(0, 1));
    listener.close();
  }

  @Test
  void memoryCheckpointRestoresLedgerTtlFencesWithFreshOwnership() {
    var service = new UnifiedIMapService(BOUNDS, 1, roomy());
    service.execute(set(1, "61", "31").ttl(TtlDirective.expireAfter(5)));
    service.execute(set(2, "62", "32")); service.retireOutcomes("c", 1, 1);
    var oldToken = token(service, "62", 0);
    var oldCursor = service.execute(new UnifiedIMapService.DomainCommand("key_set_page").limit(1)).cursor().orElseThrow();
    var checkpoint = service.checkpoint(); assertTrue(service.resourceUsage().resync() > 0);
    service.execute(set(3, "62", "33"));
    var restored = UnifiedIMapService.restore(checkpoint); assertEquals(0, service.resourceUsage().resync());
    checkpoint.close(); error("invalid_argument", () -> UnifiedIMapService.restore(checkpoint));
    assertEquals(b("32"), restored.diagnosticSnapshot(0).get(1).value());
    assertEquals(5, restored.diagnosticSnapshot(0).get(0).remainingTtl().orElseThrow());
    assertEquals(service.queryOperationOutcome(id(2)), restored.queryOperationOutcome(id(2)));
    error("retired_identity", () -> restored.execute(set(1, "61", "31")));
    assertEquals("mismatch", restored.execute(new UnifiedIMapService.DomainCommand("remove_if_version").identity(id(4)).key(b("62")).token(oldToken)).kind());
    error("cursor_mismatch", () -> restored.execute(new UnifiedIMapService.DomainCommand("key_set_page").cursor(oldCursor).limit(1)));
    service.beginOperation(set(5, "61", "34")); error("active_owners", service::checkpoint); service.cancelReserved(id(5));
    var listener = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(1));
    error("active_owners", service::checkpoint); listener.close();
    var pages = service.pages(BoundedIMapApi.Projection.KEYS, 1, () -> 0);
    error("active_owners", service::checkpoint); pages.close();
    try (var abandoned = service.checkpoint()) { assertTrue(service.resourceUsage().resync() > 0); }
    assertEquals(0, service.resourceUsage().resync());
  }

  @Test
  void rejectedBudgetDoesNotAdvanceExpiryOrEmitAndBulkChecksIntermediateOwnership() {
    var service = new UnifiedIMapService(BOUNDS, 1, limits(26, 1000, 1000, 1000, 10000, 1000, 8, 1));
    var listener = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(64));
    service.execute(set(1, "61", "31").ttl(TtlDirective.expireAfter(1))); listener.drain();
    error("resource_budget_exhausted", () -> service.execute(set(2, "62", "32").now(1)));
    assertEquals(26, service.resourceUsage().values()); assertTrue(listener.drain().isEmpty());
    assertEquals(0, service.expireStep(0, 1).expired());
    assertEquals(1, service.expireStep(1, 1).expired());
    service.execute(set(2, "62", "32").now(1)); listener.close();
    var backing = new BoundedIMapApi(BOUNDS);
    backing.put(b("61"), b("31"), null, 0); backing.put(b("62"), b("323334"), null, 0);
    var bounded = new UnifiedIMapService(backing, 1, limits(54, 1000, 1000, 1000, 10000, 1000, 8, 1));
    var changes = List.of(new UnifiedIMapService.VersionInput(b("61"), token(bounded, "61", 0), Optional.of(b("313233")), TtlDirective.preserve()),
        new UnifiedIMapService.VersionInput(b("62"), token(bounded, "62", 0), Optional.of(b("32")), TtlDirective.preserve()));
    error("resource_budget_exhausted", () -> bounded.versionBulk(UnifiedIMapService.VersionAction.REPLACE, changes, id(1), 0, 10, false));
    assertThrows(IllegalArgumentException.class, () -> backing.replaceAllDetailed(List.of(
        new BoundedIMapApi.ReplaceInput(b("61"), b("313233")), new BoundedIMapApi.ReplaceInput(b("62"), b("32"))), 0));
    assertEquals(b("31"), bounded.diagnosticSnapshot(0).get(0).value()); assertEquals(0, bounded.resourceUsage().pending());
  }

  @Test
  void targetExpiryAndStandaloneIndexCleanupStayBoundedWithoutHiddenSweeps() {
    var service = new UnifiedIMapService(BOUNDS, 1, roomy());
    var listener = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(64));
    for (int index = 0; index < 5; index++) service.execute(set(index + 1, String.format("%02x", 97 + index), "31").ttl(TtlDirective.expireAfter(1)));
    listener.drain(); service.execute(set(6, "65", "32").now(1));
    assertEquals(2, listener.drain().stream().filter(event -> event.kind().equals("expired")).count());
    assertEquals(b("32"), service.diagnosticSnapshot(1).get(0).value());
    error("expiry_backlog", () -> service.execute(new UnifiedIMapService.DomainCommand("clear_detailed").identity(id(7)).now(1)));
    service.expireStep(1, 64);
    service.execute(new UnifiedIMapService.DomainCommand("clear_detailed").identity(id(7)).now(1)); listener.close();
    var standalone = new BoundedIMapApi(BOUNDS);
    for (int tick = 0; tick < 1000; tick++) {
      standalone.put(b("61"), b("31"), (long) tick + 1, tick);
      assertTrue(standalone.getEntryView(b("61"), tick + 1).isEmpty());
    }
    var adopted = new UnifiedIMapService(standalone, 1, roomy());
    assertEquals(0, adopted.expireStep(1000, 1).work());
    assertTrue(adopted.diagnosticSnapshot(1000).isEmpty());
    assertEquals(0, adopted.diagnosticStats(1000).liveEntries());
    assertThrows(IllegalStateException.class, () -> BoundedIMapApi.fingerprint("payload", "unavailable-digest"));
  }

  @Test
  void failedPendingResumeKeepsItsCreditAndDestroyReleasesRetirementFences() {
    var service = new UnifiedIMapService(BOUNDS, 1, limits(26, 1000, 1000, 1000, 10000, 1000, 8, 1));
    service.beginOperation(set(1, "61", "31"));
    service.execute(set(2, "62", "32"));
    var credit = service.resourceUsage();
    error("resource_budget_exhausted", () -> service.executeReserved(set(1, "61", "31")));
    assertEquals("pending", service.queryOperationOutcome(id(1)).kind());
    assertEquals(credit.pending(), service.resourceUsage().pending());
    assertEquals(credit.outcomes(), service.resourceUsage().outcomes());
    service.cancelReserved(id(1)); service.retireOutcomes("c", 1, 2);
    service.execute(new UnifiedIMapService.DomainCommand("destroy").identity(id(3)));
    assertEquals(new UnifiedIMapService.ResourceUsage(0, 0, 0, 0, 0, 0), service.resourceUsage());
    error("map_destroyed", () -> service.expireStep(1, 1));
    error("map_destroyed", () -> service.snapshotListener(1, 1, 64, 1));
  }

  @Test
  void attachedAliasBudgetRejectionAndEarlyResumeDoNotCrossTheAdmissionBoundary() {
    var backing = new BoundedIMapApi(BOUNDS);
    var service = new UnifiedIMapService(backing, 1, limits(26, 1000, 1000, 1000, 10000, 1000, 8, 1));
    service.beginOperation(set(2, "62", "32"));
    service.execute(set(1, "61", "31").ttl(TtlDirective.expireAfter(1)));
    long revision = backing.revision();
    assertThrows(IllegalArgumentException.class, () -> backing.put(b("62"), b("32"), null, 1));
    assertEquals(revision, backing.revision()); assertEquals(26, service.resourceUsage().values());
    assertEquals(0, service.expireStep(0, 1).expired());
    var stats = service.diagnosticStats(0);
    error("resource_budget_exhausted", () -> service.executeReserved(set(2, "62", "32")));
    assertEquals(stats, service.diagnosticStats(0));
    assertEquals("pending", service.queryOperationOutcome(id(2)).kind());
    service.cancelReserved(id(2));
  }

  @Test
  void adoptedLegacyBulkReceiptsParticipateInBatchQueryBudgetAndRetirement() {
    var backing = new BoundedIMapApi(BOUNDS);
    backing.put(b("61"), b("31"), null, 0);
    var original = backing.tryReplaceAllDetailed(id(1), List.of(new BoundedIMapApi.ReplaceInput(b("61"), b("32")),
        new BoundedIMapApi.ReplaceInput(b("62"), b("33"))), 10, 0, false);
    var service = new UnifiedIMapService(backing, 1, roomy());
    error("resource_budget_exhausted", () -> service.queryOutcomes(List.of(id(1)), 100));
    var queried = service.queryOutcomes(List.of(id(1)), 4096).get(0).status().result().orElseThrow();
    assertEquals(List.of("replaced", "absent"), queried.items().stream().map(UnifiedIMapService.Item::kind).toList());
    assertEquals(original.items().get(0).version(), queried.items().get(0).version());
    assertEquals(1, service.retireOutcomes("c", 1, 1).removed());
    assertEquals(25, service.resourceUsage().outcomes());
    assertEquals("unknown", service.queryOutcomes(List.of(id(1)), 4096).get(0).status().kind());
  }
}
