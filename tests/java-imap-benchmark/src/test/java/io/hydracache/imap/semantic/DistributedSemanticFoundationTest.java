package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.StringReader;
import java.util.List;
import java.util.Optional;
import org.junit.jupiter.api.Test;

final class DistributedSemanticFoundationTest {
  private static final BytesValue KEY_A = BytesValue.fromHex("61");
  private static final BytesValue KEY_B = BytesValue.fromHex("62");
  private static final BytesValue KEY_C = BytesValue.fromHex("63");

  @Test
  void conditionalRemoveMismatchSuccessAndExpiryRemainEquivalent() throws Exception {
    ScenarioManifest scenario = ScenarioManifestLoader.load(new StringReader("""
        format=imap-semantic-v1
        seed=751
        step=s1 put key=61 value=31 ttl=expire_after:2
        step=s2 remove_if_value key=61 expected=39
        step=s3 remaining_ttl key=61
        step=s4 remove_if_value key=61 expected=31
        step=s5 get key=61
        step=s6 put_if_absent key=61 value=32 ttl=expire_after:1
        step=s7 advance ticks=1
        step=s8 get key=61
        """), ManifestLimits.defaults());
    assertEquals(Operation.REMOVE_IF_VALUE, scenario.steps().get(1).operation());
    try (MapSemanticAdapter left = InMemorySemanticAdapter.strict("left");
         MapSemanticAdapter right = InMemorySemanticAdapter.strict("right")) {
      OracleReport report = new SemanticOracle().compare(scenario, left, right);
      assertTrue(report.equivalent(), report::render);
      assertEquals(0, report.leftSnapshot().liveCardinality());
    }

    try (MapSemanticAdapter adapter = InMemorySemanticAdapter.strict("outcome-check")) {
      assertEquals(OutcomeKind.INSERTED, adapter.execute(scenario.steps().get(0)).kind());
      assertEquals(OutcomeKind.MISMATCH, adapter.execute(scenario.steps().get(1)).kind());
      assertEquals(OutcomeKind.REMOVED, adapter.execute(scenario.steps().get(3)).kind());
      assertEquals(OutcomeKind.ABSENT, adapter.execute(scenario.steps().get(4)).kind());
    }
  }

  @Test
  void conditionalRemoveRejectsWrongShapeBeforeExecution() {
    assertThrows(ManifestException.class, () -> ScenarioManifestLoader.load(new StringReader("""
        format=imap-semantic-v1
        seed=752
        step=s1 remove_if_value key=61 expected=31 ttl=eternal
        """), ManifestLimits.defaults()));
  }

  @Test
  void bulkRetryGroupsPartitionsAndRetriesOnlyPendingPositions() {
    var ledger = new BulkRetryLedger(List.of(KEY_A, KEY_B, KEY_C));
    List<BulkRetryLedger.PartitionBatch> first = ledger.routePending(7, key -> key.equals(KEY_B) ? 1 : 0);
    assertEquals(List.of(0, 2), first.get(0).inputIndices());
    assertEquals(List.of(1), first.get(1).inputIndices());

    ledger.complete(0, 7, item(0, KEY_A));
    assertThrows(SemanticMismatchException.class, () -> ledger.complete(1, 6, item(1, KEY_B)));
    DetailedBulkResult partial = ledger.result();
    assertFalse(partial.complete());
    partial.requireCoverage(3);

    List<BulkRetryLedger.PartitionBatch> retry = ledger.routePending(8, key -> 2);
    assertEquals(List.of(1, 2), retry.get(0).inputIndices());
    assertThrows(SemanticMismatchException.class, () -> ledger.complete(0, 8, item(0, KEY_A)));
    ledger.complete(2, 8, item(2, KEY_C));
    ledger.complete(1, 8, item(1, KEY_B));
    DetailedBulkResult complete = ledger.result();
    complete.requireCoverage(3);
    assertTrue(complete.complete());
    assertEquals(List.of(0, 1, 2), complete.items().stream().map(ItemOutcome::inputIndex).toList());
  }

  @Test
  void bulkRetryRejectsAmbiguousInputAndNonAdvancingGeneration() {
    assertThrows(IllegalArgumentException.class, () -> new BulkRetryLedger(List.of()));
    assertThrows(IllegalArgumentException.class,
        () -> new BulkRetryLedger(List.of(KEY_A, KEY_A)));
    var ledger = new BulkRetryLedger(List.of(KEY_A));
    assertThrows(IllegalArgumentException.class, () -> ledger.routePending(0, key -> 0));
    assertThrows(IllegalArgumentException.class, () -> ledger.routePending(1, key -> -1));
    ledger.routePending(1, key -> 0);
    assertThrows(SemanticMismatchException.class, () -> ledger.routePending(1, key -> 0));
    assertThrows(IllegalArgumentException.class, () -> ledger.complete(1, 1, item(0, KEY_A)));
    assertThrows(SemanticMismatchException.class,
        () -> ledger.complete(0, 1, item(0, KEY_B)));
  }

  @Test
  void listenerMigrationOpensGapAndRequiresGenerationBoundRepair() {
    var progress = new PartitionListenerProgress(3, 4, 10);
    assertTrue(progress.accept(4, 11));
    assertFalse(progress.accept(4, 11));
    assertFalse(progress.accept(5, 14));
    assertEquals(PartitionListenerProgress.Phase.GAP_DETECTED, progress.phase());
    assertThrows(SemanticMismatchException.class, () -> progress.accept(5, 14));
    progress.beginRepair();
    assertThrows(SemanticMismatchException.class, () -> progress.completeRepair(4, 13, 14));
    progress.completeRepair(5, 13, 14);
    assertEquals(14, progress.watermark());
    assertEquals(5, progress.generation());
    assertEquals(3, progress.partition());
    assertTrue(progress.accept(5, 15));
    assertThrows(SemanticMismatchException.class, () -> progress.accept(4, 16));
  }

  @Test
  void listenerCursorRejectsInvalidTransitionsAndRepairsSequenceGaps() {
    assertThrows(IllegalArgumentException.class, () -> new PartitionListenerProgress(-1, 1, 0));
    var progress = new PartitionListenerProgress(0, 1, 2);
    assertThrows(SemanticMismatchException.class, progress::beginRepair);
    assertFalse(progress.accept(1, 5));
    progress.beginRepair();
    assertThrows(SemanticMismatchException.class, () -> progress.completeRepair(1, 1, 5));
    progress.completeRepair(1, 4, 5);
    assertTrue(progress.accept(1, 6));
  }

  private static ItemOutcome item(int index, BytesValue key) {
    return new ItemOutcome(index, key, OutcomeKind.INSERTED, Optional.empty(), TtlState.eternal());
  }
}
