package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.StringReader;
import java.util.List;
import java.util.Optional;
import org.junit.jupiter.api.Test;

final class ExtendedSemanticFoundationTest {
  @Test
  void barrierRaceHasExactlyOneConditionalWinner() throws Exception {
    try (MapSemanticAdapter adapter = InMemorySemanticAdapter.strict("race")) {
      var report = new ConcurrentSemanticOracle().putIfAbsentRace(adapter,
          BytesValue.fromHex("61"), BytesValue.fromHex("31"), BytesValue.fromHex("32"));
      report.requireExactlyOnePutIfAbsentWinner();
    }
  }

  @Test
  void partialBulkResultsRetainInputIndicesAndFailOnFalseCompleteness() {
    var key = BytesValue.fromHex("61");
    var partial = new DetailedBulkResult(List.of(new ItemOutcome(0, key, OutcomeKind.INSERTED,
        Optional.empty(), TtlState.eternal())), false, ErrorClass.PARTIAL);
    partial.requireCoverage(2);

    var falselyComplete = new DetailedBulkResult(partial.items(), true, ErrorClass.NONE);
    assertThrows(SemanticMismatchException.class, () -> falselyComplete.requireCoverage(2));
  }

  @Test
  void listenerGapRequiresRepairAndCutoverBeforeEventsResume() {
    var boundary = new ListenerRepairBoundary();
    boundary.accept(1);
    boundary.gap(2, 5);
    assertThrows(SemanticMismatchException.class, () -> boundary.accept(5));
    boundary.beginRepair();
    boundary.completeRepair(4, 5);
    boundary.accept(6);
    assertEquals(6, boundary.lastDelivered());
  }

  @Test
  void containsReplaceTtlAndGapAreComparedByTheSemanticOracle() throws Exception {
    ScenarioManifest scenario = ScenarioManifestLoader.load(new StringReader("""
        format=imap-semantic-v1
        seed=75
        step=s1 contains_key key=61
        step=s2 put key=61 value=31 ttl=eternal
        step=s3 replace_if_present key=61 value=32 ttl=preserve
        step=s4 set_ttl key=61 ttl=expire_after:2
        step=s5 remaining_ttl key=61
        step=s6 listener_gap key=61
        step=s7 advance ticks=2
        step=s8 contains_key key=61
        """), ManifestLimits.defaults());
    try (MapSemanticAdapter left = InMemorySemanticAdapter.strict("left");
         MapSemanticAdapter right = InMemorySemanticAdapter.strict("right")) {
      OracleReport report = new SemanticOracle().compare(scenario, left, right);
      assertTrue(report.equivalent(), report::render);
      assertEquals(0, report.leftSnapshot().liveCardinality());
    }
  }
}
