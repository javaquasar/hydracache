package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;
import static io.hydracache.imap.semantic.ExtendedSemanticScenario.*;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.Set;
import org.junit.jupiter.api.Test;

final class ExtendedSemanticOracleTest {
  static BytesValue b(String hex) { return BytesValue.fromHex(hex); }
  static Request request(long n) { return new Request("oracle", n); }
  static Entry value(String key, String value) { return new Entry(b(key), Optional.of(b(value)), Optional.empty(), TtlDirective.eternal()); }
  static Entry token(String key, String label, String value) { return new Entry(b(key), value == null ? Optional.empty() : Optional.of(b(value)), Optional.of(label), TtlDirective.preserve()); }
  static Step control(String id, Op op) { return new Step(id, op, List.of(), List.of(), Optional.empty(), 0, false); }
  static Step mutation(String id, Op op, long identity, Entry... entries) { return new Step(id, op, List.of(entries), List.of(request(identity)), Optional.empty(), 0, false); }
  static Step capture(String id, String key, String label) { return new Step(id, Op.CAPTURE_VERSION, List.of(token(key, label, null)), List.of(), Optional.empty(), 0, false); }
  static Step query(String id, long... ids) { return new Step(id, Op.QUERY_OUTCOMES, List.of(), java.util.Arrays.stream(ids).mapToObj(ExtendedSemanticOracleTest::request).toList(), Optional.empty(), 0, false); }
  static Step retire(String id, long through) { return new Step(id, Op.RETIRE_OUTCOMES, List.of(), List.of(request(through)), Optional.empty(), 0, false); }
  static Step resync(String id, int maximum, long bytes, boolean hold) { return new Step(id, hold ? Op.HOLD_LISTENER_SNAPSHOT : Op.LISTENER_RESYNC, List.of(), List.of(), Optional.of(new Limits(maximum, bytes)), 0, false); }
  static ExtendedSemanticScenario trace(List<Step> steps) { return new ExtendedSemanticScenario(2, 117, steps); }
  static UnifiedIMapExtendedSemanticAdapter actual() { return UnifiedIMapExtendedSemanticAdapter.fresh("extended-actual"); }

  @Test void independentOracleCoversTokensMixedBulkOutcomeQueriesRetirementAndCausality() {
    var steps = List.of(
        mutation("put-a", Op.PUT, 1, value("61", "31")),
        mutation("put-b", Op.PUT, 2, value("62", "32")),
        capture("capture-a", "61", "a0"), capture("capture-b", "62", "b0"),
        mutation("cas-a", Op.REPLACE_IF_VERSION, 3, token("61", "a0", "33")),
        capture("fresh-a", "61", "a1"),
        mutation("mixed-bulk", Op.BULK_REPLACE, 4, token("61", "a0", "34"), token("62", "b0", "35")),
        query("query-results", 3, 4, 99),
        mutation("remove-a", Op.REMOVE_IF_VERSION, 5, token("61", "a1", null)),
        mutation("aba-a", Op.PUT, 6, value("61", "33")),
        mutation("old-token", Op.REPLACE_IF_VERSION, 7, token("61", "a1", "39")),
        capture("capture-aba", "61", "a2"),
        resync("snapshot", 64, 65536, false),
        retire("retire", 4), query("retired-unknown", 1, 4, 6),
        mutation("retired-retry", Op.PUT, 1, value("61", "31")),
        control("resources", Op.RESOURCES));
    try (var adapter = actual()) { new SemanticOracle().compare(trace(steps), adapter).requireEquivalent(); }
  }

  @Test void lostResponsesReservationCancellationRestoreAndLeasesHaveExplicitOutcomes() {
    var put = mutation("lost", Op.PUT, 1, value("61", "31"));
    var lost = new Step(put.id(), put.operation(), put.entries(), put.requests(), put.limits(), 0, true);
    var reserve = mutation("reserve", Op.RESERVE_PUT, 2, value("62", "32"));
    var dispatch = mutation("dispatch", Op.DISPATCH_PUT, 2, value("62", "32"));
    var cancelled = mutation("reserve-cancel", Op.RESERVE_PUT, 3, value("63", "33"));
    var steps = List.of(lost, query("recover", 1), reserve, query("pending", 2), dispatch,
        cancelled, new Step("cancel", Op.CANCEL_RESERVED, List.of(), List.of(request(3)), Optional.empty(), 0, false),
        query("cancelled-unknown", 3), capture("token-before-restore", "61", "old"),
        resync("hold", 64, 65536, true), control("busy-checkpoint", Op.CHECKPOINT_RESTORE),
        control("release", Op.CLOSE_LISTENER_SNAPSHOT), control("restore", Op.CHECKPOINT_RESTORE),
        mutation("old-instance-token", Op.REPLACE_IF_VERSION, 4, token("61", "old", "39")),
        capture("new-instance-token", "61", "new"), query("retained-after-restore", 1, 2),
        control("gap", Op.SIGNAL_GAP), resync("repair", 64, 65536, false));
    try (var adapter = actual()) { new SemanticOracle().compare(trace(steps), adapter).requireEquivalent(); }
  }

  @Test void expiryTtlBulkRemoveEvictionClearDestroyAndDuplicateRejectionUseTheSameOracle() {
    var expiring = new Entry(b("61"), Optional.of(b("31")), Optional.empty(), TtlDirective.expireAfter(3));
    var ttl = new Entry(b("61"), Optional.empty(), Optional.of("a"), TtlDirective.expireAfter(1));
    var steps = List.of(mutation("expiring", Op.PUT, 1, expiring), capture("capture", "61", "a"),
        mutation("ttl", Op.SET_TTL_IF_VERSION, 2, ttl),
        new Step("advance", Op.ADVANCE, List.of(), List.of(), Optional.empty(), 1, false),
        capture("absent", "61", "expired"),
        mutation("repopulate-a", Op.PUT, 3, value("61", "31")), mutation("repopulate-b", Op.PUT, 4, value("62", "32")),
        capture("a1", "61", "a1"), capture("b1", "62", "b1"),
        mutation("duplicate", Op.BULK_REMOVE, 5, token("61", "a1", null), token("61", "a1", null)),
        mutation("remove", Op.BULK_REMOVE, 6, token("61", "a1", null), token("62", "b1", null)),
        mutation("repopulate", Op.PUT, 7, value("63", "33")), mutation("evict", Op.EVICT, 8, new Entry(b("63"), Optional.empty(), Optional.empty(), TtlDirective.preserve())),
        mutation("clear-value", Op.PUT, 9, value("61", "34")), mutation("clear", Op.CLEAR, 10), mutation("empty-clear", Op.CLEAR, 11),
        mutation("destroy", Op.DESTROY, 12), control("after-destroy", Op.RESOURCES));
    try (var adapter = actual()) { new SemanticOracle().compare(trace(steps), adapter).requireEquivalent(); }
  }

  @Test void negativeControlsCannotHideMixedBulkWrongTtlMissingEventsOrTokenReuse() {
    var steps = List.of(mutation("put", Op.PUT, 1, value("61", "31")), capture("token", "61", "t"),
        mutation("put-b", Op.PUT, 2, value("62", "32")), capture("token-b", "62", "b"),
        mutation("bulk", Op.BULK_REPLACE, 3, token("61", "t", "33"), token("62", "t", "34")),
        capture("fresh", "61", "fresh"), resync("cut", 64, 65536, false));
    for (var fault : UnifiedIMapExtendedSemanticAdapter.Fault.values()) {
      if (fault == UnifiedIMapExtendedSemanticAdapter.Fault.NONE) continue;
      try (var adapter = UnifiedIMapExtendedSemanticAdapter.fresh("broken", fault)) {
        var report = new SemanticOracle().compare(trace(steps), adapter);
        assertFalse(report.equivalent(), fault.toString());
        assertThrows(IllegalStateException.class, report::requireEquivalent);
      }
    }
  }

  @Test void boundedGrammarAndCapabilityPreflightRejectBeforeAdapterExecution() {
    assertThrows(IllegalArgumentException.class, () -> new ExtendedSemanticScenario(3, 0, List.of()));
    assertThrows(IllegalArgumentException.class, () -> trace(java.util.Collections.nCopies(257, control("x", Op.RESOURCES))));
    assertThrows(IllegalArgumentException.class, () -> trace(List.of(control("x", Op.RESOURCES), control("x", Op.RESOURCES))));
    assertThrows(IllegalArgumentException.class, () -> new Request("", 1));
    assertThrows(IllegalArgumentException.class, () -> new Request("x", 0));
    assertThrows(IllegalArgumentException.class, () -> new Limits(0, 1));
    assertThrows(IllegalArgumentException.class, () -> new Limits(1, 0));
    assertThrows(IllegalArgumentException.class, () -> mutation("missing-value", Op.PUT, 1, token("61", "t", null)));
    try (var owner = actual()) {
      var restricted = new ExtendedMapSemanticAdapter() {
        public AdapterIdentity identity() { return owner.identity(); }
        public Set<Op> extendedCapabilities() { return Set.of(); }
        public Observation executeExtended(Step step) { fail("capability refusal must precede execution"); return null; }
        public void close() {}
      };
      assertFalse(new SemanticOracle().compare(trace(List.of(control("r", Op.RESOURCES))), restricted).equivalent());
    }
    var mutable = new ArrayList<>(List.of(control("r", Op.RESOURCES)));
    var scenario = trace(mutable); mutable.clear(); assertEquals(1, scenario.steps().size());
    assertThrows(UnsupportedOperationException.class, () -> scenario.steps().clear());
  }

  @Test void rejectionAndRetryTracesPreservePendingRootsCutsAndOriginalResults() {
    var absentPreserve = new Entry(b("61"), Optional.of(b("31")), Optional.empty(), TtlDirective.preserve());
    var steps = List.of(mutation("bad-preserve", Op.PUT, 1, absentPreserve), query("not-admitted", 1),
        mutation("put", Op.PUT, 1, value("61", "31")), mutation("replay", Op.PUT, 1, value("61", "31")),
        mutation("conflicting", Op.PUT, 1, value("61", "32")), capture("capture", "61", "t"),
        mutation("reserve", Op.RESERVE_PUT, 2, value("62", "32")), retire("pending-retire", 2),
        control("pending-checkpoint", Op.CHECKPOINT_RESTORE),
        mutation("pending-conflict", Op.RESERVE_PUT, 2, value("62", "33")),
        mutation("dispatch", Op.DISPATCH_PUT, 2, value("62", "32")),
        mutation("completed-reservation", Op.RESERVE_PUT, 2, value("62", "32")),
        new Step("cancel-absent", Op.CANCEL_RESERVED, List.of(), List.of(request(99)), Optional.empty(), 0, false),
        resync("small-count", 1, 65536, false), resync("small-bytes", 64, 1, false),
        resync("held", 64, 65536, true), resync("held-again", 64, 65536, true),
        control("close", Op.CLOSE_LISTENER_SNAPSHOT), control("close-again", Op.CLOSE_LISTENER_SNAPSHOT),
        retire("retire", 2), retire("backward-retire", 1),
        mutation("remove-missing", Op.REMOVE_IF_VERSION, 3, token("63", "t", null)),
        mutation("evict-missing", Op.EVICT, 4, new Entry(b("63"), Optional.empty(), Optional.empty(), TtlDirective.preserve())));
    try (var adapter = actual()) { new SemanticOracle().compare(trace(steps), adapter).requireEquivalent(); }
  }

  @Test void bulkTtlAndLostBulkReceiptsKeepInputOrderExpiryAndNoDuplicateEvents() {
    var lost = mutation("lost-bulk", Op.BULK_SET_TTL, 3,
        new Entry(b("61"), Optional.empty(), Optional.of("a"), TtlDirective.expireAfter(2)),
        new Entry(b("62"), Optional.empty(), Optional.of("b"), TtlDirective.eternal()));
    var steps = List.of(mutation("a", Op.PUT, 1, value("61", "31")), mutation("b", Op.PUT, 2, value("62", "32")),
        capture("capture-a", "61", "a"), capture("capture-b", "62", "b"),
        new Step(lost.id(), lost.operation(), lost.entries(), lost.requests(), lost.limits(), 0, true),
        query("recover-bulk", 3), new Step("retry", lost.operation(), lost.entries(), lost.requests(), lost.limits(), 0, false),
        mutation("bad-ttl", Op.BULK_SET_TTL, 4, token("61", "a", null)),
        new Step("expire-a", Op.ADVANCE, List.of(), List.of(), Optional.empty(), 2, false),
        resync("live-b", 64, 65536, false));
    try (var adapter = actual()) { new SemanticOracle().compare(trace(steps), adapter).requireEquivalent(); }
    try (var adapter = actual()) {
      new SemanticOracle().compare(trace(List.of(resync("empty-held", 64, 64, true),
          control("empty-held-busy", Op.CHECKPOINT_RESTORE), control("release-empty", Op.CLOSE_LISTENER_SNAPSHOT),
          control("restore-empty", Op.CHECKPOINT_RESTORE))), adapter).requireEquivalent();
    }
  }

  @Test void grammarRejectsOversizedPayloadUnknownReferencesAndInvalidCommandShapes() {
    assertThrows(IllegalArgumentException.class, () -> trace(List.of(capture("a", "61", "same"), capture("b", "61", "same"))));
    assertThrows(IllegalArgumentException.class, () -> trace(List.of(mutation("unknown", Op.REPLACE_IF_VERSION, 1, token("61", "missing", "31")))));
    assertThrows(IllegalArgumentException.class, () -> new Entry(BytesValue.copyOf(new byte[1025]), Optional.empty(), Optional.empty(), TtlDirective.eternal()));
    assertThrows(IllegalArgumentException.class, () -> new Entry(b(""), Optional.of(BytesValue.copyOf(new byte[4097])), Optional.empty(), TtlDirective.eternal()));
    assertThrows(IllegalArgumentException.class, () -> new Request("x".repeat(97), 1));
    assertThrows(IllegalArgumentException.class, () -> control("bad=id", Op.RESOURCES));
    assertThrows(IllegalArgumentException.class, () -> new Limits(65, 1));
    assertThrows(IllegalArgumentException.class, () -> new Limits(1, 1024 * 1024 + 1));
    assertThrows(IllegalArgumentException.class, () -> new Step("bad", Op.RESOURCES, List.of(), List.of(), Optional.empty(), 0, true));
    assertThrows(IllegalArgumentException.class, () -> new Step("bad", Op.ADVANCE, List.of(), List.of(), Optional.empty(), 0, false));
    assertThrows(IllegalArgumentException.class, () -> new Step("bad", Op.LISTENER_RESYNC, List.of(), List.of(), Optional.empty(), 0, false));
    assertThrows(IllegalArgumentException.class, () -> new Step("bad", Op.PUT, List.of(value("61", "31")), List.of(), Optional.empty(), 0, false));
    assertThrows(IllegalArgumentException.class, () -> new Step("bad", Op.BULK_REMOVE, List.of(), List.of(request(1)), Optional.empty(), 0, false));
    var huge = new Entry(b("61"), Optional.of(BytesValue.copyOf(new byte[4096])), Optional.of("a"), TtlDirective.eternal());
    var many = java.util.Collections.nCopies(64, huge);
    assertThrows(IllegalArgumentException.class, () -> trace(List.of(capture("a", "61", "a"),
        new Step("one", Op.BULK_REPLACE, many, List.of(request(1)), Optional.empty(), 0, false),
        new Step("two", Op.BULK_REPLACE, many, List.of(request(2)), Optional.empty(), 0, false))));
    var manyLabels = java.util.stream.IntStream.range(0, 65).mapToObj(i -> capture("id" + i, "61", "label" + i)).toList();
    assertThrows(IllegalArgumentException.class, () -> trace(manyLabels));
  }

  @Test void adapterUsesInjectedOwnerClosesLeasesAndBoundsCounterexampleReports() {
    var owner = new UnifiedIMapService(new BoundedIMapApi.Bounds(1024, 4096, 256, 64, 256, 1024 * 1024, 64), 1);
    var adapter = new UnifiedIMapExtendedSemanticAdapter("injected", owner);
    adapter.executeExtended(mutation("external-observation", Op.PUT, 1, value("61", "31")));
    assertEquals(b("31"), owner.diagnosticSnapshot(0).get(0).value());
    adapter.executeExtended(resync("hold", 64, 65536, true)); assertTrue(owner.resourceUsage().resync() > 0);
    adapter.close(); adapter.close(); assertEquals(0, owner.resourceUsage().resync()); assertEquals(0, owner.diagnosticStats(0).listeners());
    assertThrows(IllegalStateException.class, () -> adapter.executeExtended(control("closed", Op.RESOURCES)));
    try (var bounded = UnifiedIMapExtendedSemanticAdapter.fresh("bounded-canary", UnifiedIMapExtendedSemanticAdapter.Fault.WRONG_CLOCK)) {
      var steps = java.util.stream.IntStream.range(0, 256).mapToObj(i -> control("r" + i, Op.RESOURCES)).toList();
      var report = new SemanticOracle().compare(trace(steps), bounded);
      assertEquals(256, report.checkedSteps()); assertEquals(32, report.differences().size());
      assertThrows(UnsupportedOperationException.class, () -> report.differences().clear());
      assertEquals(117, report.seed()); assertEquals(bounded.identity(), report.actual());
    }
    try (var empty = actual()) { assertEquals(0, new SemanticOracle().compare(trace(List.of()), empty).checkedSteps()); }
  }

  @Test void completedPreserveReplayPrecedesStatefulChecksAndUnknownDispatchCannotApply() {
    var owner = new UnifiedIMapService(new BoundedIMapApi.Bounds(1024, 4096, 256, 64, 256, 1024 * 1024, 64), 1);
    var preserve = new Entry(b("61"), Optional.of(b("32")), Optional.empty(), TtlDirective.preserve());
    var steps = List.of(mutation("initial", Op.PUT, 1, value("61", "31")),
        mutation("preserve", Op.PUT, 2, preserve),
        mutation("evict", Op.EVICT, 3, new Entry(b("61"), Optional.empty(), Optional.empty(), TtlDirective.eternal())),
        mutation("original-receipt", Op.PUT, 2, preserve), query("original-completed", 2),
        mutation("unknown-dispatch", Op.DISPATCH_PUT, 4, value("62", "32")), query("never-admitted", 4));
    try (var adapter = new UnifiedIMapExtendedSemanticAdapter("replay-boundary", owner)) {
      new SemanticOracle().compare(trace(steps), adapter).requireEquivalent();
      assertTrue(owner.diagnosticSnapshot(0).isEmpty());
      assertEquals("unknown", owner.queryOperationOutcome(UnifiedIMapExtendedSemanticAdapter.id(request(4))).kind());
      assertEquals("completed", owner.queryOperationOutcome(UnifiedIMapExtendedSemanticAdapter.id(request(2))).kind());
    }
  }

  @Test void destroyPreservesCleanupAndCannotResurrectRetirementFenceOwnership() {
    var owner = new UnifiedIMapService(new BoundedIMapApi.Bounds(1024, 4096, 256, 64, 256, 1024 * 1024, 64), 1);
    var steps = List.of(mutation("put", Op.PUT, 1, value("61", "31")), resync("held", 64, 65536, true),
        mutation("destroy", Op.DESTROY, 2), control("release-after-destroy", Op.CLOSE_LISTENER_SNAPSHOT),
        new Step("cancel-after-destroy", Op.CANCEL_RESERVED, List.of(), List.of(request(1)), Optional.empty(), 0, false),
        retire("retire-after-destroy", 2), query("destroy-cleared-ledger", 1, 2), control("owners", Op.RESOURCES));
    try (var adapter = new UnifiedIMapExtendedSemanticAdapter("terminal-cleanup", owner)) {
      new SemanticOracle().compare(trace(steps), adapter).requireEquivalent();
      var before = owner.resourceUsage(); assertEquals(0, before.outcomes()); assertEquals(0, before.resync());
      assertEquals("map_destroyed", assertThrows(IMapDomainException.class, () -> owner.retireOutcomes("new-client", 1, 1)).kind());
      assertEquals(before, owner.resourceUsage());
    }
  }

  @Test void snapshotReportsOriginalCutWatermarkAndWrongCutCannotPassCausality() {
    var steps = List.of(mutation("a", Op.PUT, 1, value("61", "31")), mutation("b", Op.PUT, 2, value("62", "32")),
        control("gap", Op.SIGNAL_GAP), mutation("suppressed-event", Op.PUT, 3, value("63", "33")),
        resync("original-cut", 64, 65536, false), mutation("after-cut", Op.PUT, 4, value("63", "34")));
    try (var adapter = actual()) { new SemanticOracle().compare(trace(steps), adapter).requireEquivalent(); }
    try (var faulty = UnifiedIMapExtendedSemanticAdapter.fresh("wrong-cut", UnifiedIMapExtendedSemanticAdapter.Fault.WRONG_CUT)) {
      var report = new SemanticOracle().compare(trace(steps), faulty);
      assertTrue(report.differences().stream().anyMatch(difference -> difference.path().equals("step.original-cut.result")));
    }
    try (var adapter = actual()) {
      for (int index = 0; index < 4; index++) adapter.executeExtended(steps.get(index));
      var cut = adapter.executeExtended(steps.get(4)).result(); assertEquals(4, cut.number()); assertEquals(3, cut.items().size());
      assertEquals(5, adapter.executeExtended(steps.get(5)).events().get(0).watermark());
    }
  }

  @Test void hostileAdapterPayloadsAndNestedOutcomeStatusesFailBeforeReportFormatting() {
    try (var owner = actual()) {
      var oversized = new ExtendedMapSemanticAdapter() {
        public AdapterIdentity identity() { return owner.identity(); }
        public Set<Op> extendedCapabilities() { return owner.extendedCapabilities(); }
        public Observation executeExtended(Step step) {
          var original = owner.executeExtended(step);
          var result = new Result("present", Optional.of(BytesValue.copyOf(new byte[4097])), TtlState.eternal(), List.of(), List.of(), 0);
          return new Observation(result, original.snapshot(), original.events(), original.tokenRelations(), original.owners(), original.tick());
        }
        public void close() {}
      };
      assertThrows(IllegalArgumentException.class, () -> new SemanticOracle().compare(trace(List.of(control("bounded", Op.RESOURCES))), oversized));
      var nested = new ExtendedMapSemanticAdapter() {
        public AdapterIdentity identity() { return owner.identity(); }
        public Set<Op> extendedCapabilities() { return owner.extendedCapabilities(); }
        public Observation executeExtended(Step step) {
          var original = owner.executeExtended(step);
          var retained = new Result("outcomes", Optional.empty(), TtlState.absent(), List.of(),
              List.of(new Status(request(1), "completed", Optional.of(Result.simple("inserted")))), 1);
          var outer = new Result("outcomes", Optional.empty(), TtlState.absent(), List.of(),
              List.of(new Status(request(2), "completed", Optional.of(retained))), 1);
          return new Observation(outer, original.snapshot(), original.events(), original.tokenRelations(), original.owners(), original.tick());
        }
        public void close() {}
      };
      assertThrows(IllegalArgumentException.class, () -> new SemanticOracle().compare(trace(List.of(control("nested", Op.RESOURCES))), nested));
    }
    class HostileOutput { @Override public String toString() { throw new AssertionError("untrusted toString must not execute"); } }
    assertEquals("HostileOutput", ExtendedSemanticOracle.brief(new HostileOutput()));
    assertEquals(256, ExtendedSemanticOracle.brief("x".repeat(1024 * 1024)).length());
    assertEquals("{a=true,}", ExtendedSemanticOracle.brief(java.util.Map.of("a", true)));
    assertTrue(ExtendedSemanticOracle.brief(new Status(request(1), "completed", Optional.of(Result.simple("applied")))).contains("applied"));
    assertEquals(256, ExtendedSemanticOracle.brief(List.of(b("31".repeat(4096)))).length());
    assertTrue(ExtendedSemanticOracle.brief(List.of(Optional.empty(), List.of("x"))).contains("absent"));
  }

  @Test void clockAndExpiryOverflowProduceTypedRejectionsWithoutAdvancingOrRetaining() {
    var advanceMaximum = new Step("maximum", Op.ADVANCE, List.of(), List.of(), Optional.empty(), Long.MAX_VALUE, false);
    var expiryOverflow = new Entry(b("61"), Optional.of(b("31")), Optional.empty(), TtlDirective.expireAfter(1));
    try (var adapter = actual()) {
      new SemanticOracle().compare(trace(List.of(advanceMaximum, mutation("expiry-overflow", Op.PUT, 1, expiryOverflow),
          query("not-retained", 1), mutation("maximum-default-deadline", Op.PUT, 1, value("61", "31")), query("deadline-not-retained", 1))), adapter).requireEquivalent();
    }
    try (var adapter = actual()) {
      new SemanticOracle().compare(trace(List.of(advanceMaximum,
          new Step("clock-overflow", Op.ADVANCE, List.of(), List.of(), Optional.empty(), 1, false),
          control("unchanged", Op.RESOURCES))), adapter).requireEquivalent();
    }
    try (var wrongClock = UnifiedIMapExtendedSemanticAdapter.fresh("maximum-clock-canary", UnifiedIMapExtendedSemanticAdapter.Fault.WRONG_CLOCK)) {
      assertFalse(new SemanticOracle().compare(trace(List.of(advanceMaximum)), wrongClock).equivalent());
    }
    try (var adapter = actual()) {
      new SemanticOracle().compare(trace(List.of(mutation("pending", Op.RESERVE_PUT, 1, value("61", "31")), advanceMaximum,
          mutation("deadline-dispatch", Op.DISPATCH_PUT, 1, value("61", "31")), query("released", 1))), adapter).requireEquivalent();
    }
  }
}
