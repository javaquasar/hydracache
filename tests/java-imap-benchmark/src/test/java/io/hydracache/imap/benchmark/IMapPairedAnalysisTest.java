package io.hydracache.imap.benchmark;

import static io.hydracache.imap.benchmark.IMapLoadCampaignTest.*;
import static org.junit.jupiter.api.Assertions.*;

import io.hydracache.imap.semantic.AdapterIdentity;
import io.hydracache.imap.semantic.ErrorClass;
import io.hydracache.imap.semantic.InMemorySemanticAdapter;
import io.hydracache.imap.semantic.OracleReport;
import io.hydracache.imap.semantic.SemanticDifference;
import io.hydracache.imap.semantic.StateSnapshot;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.Test;

final class IMapPairedAnalysisTest {
  private static final AdapterIdentity LEFT = new AdapterIdentity("left", "fixture", "diagnostic");
  private static final AdapterIdentity RIGHT = new AdapterIdentity("right", "fixture", "diagnostic");
  private static final AdapterIdentity OTHER = new AdapterIdentity("other", "fixture", "diagnostic");
  private static final StateSnapshot EMPTY = InMemorySemanticAdapter.strict("empty").snapshot();
  private static final StateSnapshot FILLED = new StateSnapshot("a".repeat(64), 1, 0);

  @Test void policyMustBeBoundedBeforeRuns() {
    for (int iterations : List.of(999, 5001)) {
      assertThrows(IllegalArgumentException.class, () -> new IMapPairedAnalysis.Policy(1, iterations, 950, 100, 150));
    }
    for (int confidence : List.of(899, 1000)) {
      assertThrows(IllegalArgumentException.class, () -> new IMapPairedAnalysis.Policy(1, 1000, confidence, 100, 150));
    }
    assertThrows(IllegalArgumentException.class, () -> new IMapPairedAnalysis.Policy(1, 1000, 950, 0, 150));
    assertThrows(IllegalArgumentException.class, () -> new IMapPairedAnalysis.Policy(1, 1000, 950, 150, 100));
  }

  @Test void pairedEffectsRetainRawRunsAndAccountForDrainInGoodput() {
    var campaign = campaign(0, 100, 6, POLICY);
    var attempts = attempts(campaign, List.of(80), List.of(100), List.of(100), List.of(100));
    var report = analyze(campaign, attempts);
    assertEquals(IMapPairedAnalysis.Verdict.RIGHT_ADVANTAGE, report.verdict());
    assertEquals(attempts, report.attempts());
    assertEquals(report, analyze(campaign, attempts));
    var stats = report.statistics().orElseThrow();
    assertEquals(6, stats.observations().size());
    assertEquals(1.6, stats.goodput().leftMedian());
    assertEquals(2, stats.goodput().rightMedian());
    assertEquals(0.25, stats.goodput().relativeEffect(), 1e-12);
    assertEquals(0.25, stats.goodput().lower(), 1e-12);
    assertEquals(0.25, stats.goodput().upper(), 1e-12);
    assertEquals(110, stats.p999().rightMedian());
    assertEquals(1, stats.observations().get(0).pairNumber());

    var withDrain = new ArrayList<IMapLoadCampaign.Attempt>();
    for (var attempt : attempts) {
      var measured = (IMapLoadCampaign.Measured) attempt;
      var pair = measured.result();
      var phase = pair.right().measured();
      var slower = new FixedRateIMapLoad.Phase(phase.scheduled(), phase.completed(), phase.successful(),
          0, 0, phase.offeredWindowNanos(), phase.offeredWindowNanos() * 2, phase.errors(),
          phase.endToEnd(), phase.queue(), phase.service());
      withDrain.add(new IMapLoadCampaign.Measured(measured.run(),
          pairWithTrials(pair, pair.left(), trial(RIGHT, pair.right().warmup(), slower))));
    }
    var drained = analyze(campaign, withDrain);
    assertEquals(IMapPairedAnalysis.Verdict.LEFT_ADVANTAGE, drained.verdict());
    assertEquals(1, drained.statistics().orElseThrow().goodput().rightMedian());
  }

  @Test void symmetricAdvantageAndEquivalenceUseBothRegressionGuards() {
    assertVerdict(IMapPairedAnalysis.Verdict.LEFT_ADVANTAGE, 100, 80, 100, 100);
    assertVerdict(IMapPairedAnalysis.Verdict.RIGHT_ADVANTAGE, 100, 97, 100, 80);
    assertVerdict(IMapPairedAnalysis.Verdict.RIGHT_ADVANTAGE, 100, 100, 100, 90);
    assertVerdict(IMapPairedAnalysis.Verdict.LEFT_ADVANTAGE, 97, 100, 80, 100);
    assertVerdict(IMapPairedAnalysis.Verdict.EQUIVALENT, 100, 100, 100, 101);
    assertVerdict(IMapPairedAnalysis.Verdict.INCONCLUSIVE, 80, 100, 80, 100);
    assertVerdict(IMapPairedAnalysis.Verdict.INCONCLUSIVE, 100, 94, 100, 80);
    assertVerdict(IMapPairedAnalysis.Verdict.INCONCLUSIVE, 90, 96, 100, 100);
    assertVerdict(IMapPairedAnalysis.Verdict.INCONCLUSIVE, 100, 100, 100, 108);
  }

  @Test void effectAloneCannotWinWhenPairedBootstrapSpansZero() {
    var campaign = campaign(0, 100, 5, POLICY);
    var varyingGoodput = analyze(campaign, attempts(campaign, List.of(50),
        List.of(80, 80, 80, 40, 40), List.of(100), List.of(100)));
    var goodput = varyingGoodput.statistics().orElseThrow().goodput();
    assertTrue(goodput.relativeEffect() > 0.10);
    assertTrue(goodput.lower() < 0 && goodput.upper() > 0);
    assertEquals(IMapPairedAnalysis.Verdict.INCONCLUSIVE, varyingGoodput.verdict());

    var varyingTail = analyze(campaign, attempts(campaign, List.of(100), List.of(100),
        List.of(100), List.of(55, 55, 55, 130, 130)));
    var p99 = varyingTail.statistics().orElseThrow().p99();
    assertTrue(p99.relativeEffect() < -0.10);
    assertTrue(p99.lower() < 0 && p99.upper() > 0);
    assertEquals(IMapPairedAnalysis.Verdict.INCONCLUSIVE, varyingTail.verdict());
  }

  @Test void preregisteredSloCanVetoAStatisticalAdvantage() {
    var campaign = campaign(0, 100, 5, new IMapPairedAnalysis.Policy(1, 1000, 950, 100, 110));
    for (var values : List.of(List.of(101, 90), List.of(90, 101))) {
      var report = analyze(campaign, attempts(campaign, List.of(80), List.of(100),
          List.of(values.get(0)), List.of(values.get(1))));
      assertEquals(IMapPairedAnalysis.Verdict.INCONCLUSIVE, report.verdict());
      assertTrue(report.reason().contains("SLO"));
      assertTrue(report.statistics().isPresent());
    }
    var tightTail = campaign(0, 100, 5, new IMapPairedAnalysis.Policy(1, 1000, 950, 100, 105));
    var report = analyze(tightTail, attempts(tightTail, List.of(80), List.of(100), List.of(100), List.of(100)));
    assertTrue(report.reason().contains("p99.9"));
    assertEquals(IMapPairedAnalysis.Verdict.INCONCLUSIVE, report.verdict());
  }

  @Test void refusesIncompleteFailedOrReorderedPairsAndRetainsTheLedger() {
    var campaign = campaign(0, 100, 5, POLICY);
    var attempts = attempts(campaign, List.of(80), List.of(100), List.of(100), List.of(100));
    var incomplete = analyze(campaign, attempts.subList(0, 4));
    assertEquals(IMapPairedAnalysis.Verdict.INCONCLUSIVE, incomplete.verdict());
    assertEquals(4, incomplete.attempts().size());
    assertTrue(incomplete.statistics().isEmpty());
    var reordered = new ArrayList<>(attempts);
    Collections.swap(reordered, 0, 1);
    assertThrows(IllegalArgumentException.class, () -> analyze(campaign, reordered));
    var extra = new ArrayList<>(attempts);
    extra.add(attempts.get(0));
    assertThrows(IllegalArgumentException.class, () -> analyze(campaign, extra));
    var changedPolicy = campaign(0, 100, 5, new IMapPairedAnalysis.Policy(2, 1000, 950, 1000, 1500));
    assertThrows(IllegalArgumentException.class, () -> analyze(changedPolicy, attempts));

    for (var failure : IMapLoadCampaign.Failure.values()) {
      var broken = new ArrayList<>(attempts);
      broken.set(2, new IMapLoadCampaign.Failed(campaign.runs().get(2), failure, "observed failure"));
      var report = analyze(campaign, broken);
      assertEquals(broken, report.attempts());
      assertTrue(report.statistics().isEmpty());
      assertEquals(switch (failure) {
        case NOT_COMPARABLE -> IMapPairedAnalysis.Verdict.NOT_COMPARABLE;
        case INFRASTRUCTURE -> IMapPairedAnalysis.Verdict.INVALID_INFRASTRUCTURE;
        case CANCELLED -> IMapPairedAnalysis.Verdict.INCONCLUSIVE;
      }, report.verdict());
      assertEquals("observed failure", report.reason());
    }
  }

  @Test void incompleteMultiCellLedgerKeepsLaterCellsPending() {
    var campaign = IMapLoadCampaign.plan(List.of(READS), List.of(2, 4), List.of(SERIAL),
        new FixedRateIMapLoad.Settings(117, 2, 0, 100, 1, 1, 1000), 5, BINDINGS, POLICY);
    var reports = IMapPairedAnalysis.analyze(campaign, attempts(campaign, List.of(100),
        List.of(100), List.of(100), List.of(100)).subList(0, 6));
    assertEquals(2, reports.size());
    assertEquals(5, reports.get(0).attempts().size());
    assertEquals(IMapPairedAnalysis.Verdict.EQUIVALENT, reports.get(0).verdict());
    assertEquals(1, reports.get(1).attempts().size());
    assertEquals(IMapPairedAnalysis.Verdict.INCONCLUSIVE, reports.get(1).verdict());
  }

  @Test void rejectsRelabelledObservationsAndAdapterIdentitySubstitution() {
    var campaign = campaign(0, 100, 5, POLICY);
    var first = first(campaign);
    var run = first.run();
    var pair = first.result();
    var settings = pair.settings();
    var changed = new FixedRateIMapLoad.Settings(settings.seed() + 1, settings.requestsPerSecond(),
        settings.warmupRequests(), settings.measuredRequests(), settings.workers(),
        settings.maxInFlight(), settings.drainTimeoutMillis());
    for (var substitute : List.of(
        new FixedRateIMapLoad.Pair(changed, pair.workloadFingerprint(), pair.rightFirst(), pair.admission(),
            pair.preloaded(), pair.left(), pair.right()),
        new FixedRateIMapLoad.Pair(settings, "different", pair.rightFirst(), pair.admission(),
            pair.preloaded(), pair.left(), pair.right()),
        new FixedRateIMapLoad.Pair(settings, pair.workloadFingerprint(), !pair.rightFirst(), pair.admission(),
            pair.preloaded(), pair.left(), pair.right()),
        pairWithTrials(pair, trial(OTHER, pair.left().warmup(), pair.left().measured()), pair.right()),
        pairWithTrials(pair, pair.left(), trial(OTHER, pair.right().warmup(), pair.right().measured())))) {
      assertThrows(IllegalArgumentException.class,
          () -> analyze(campaign, List.of(new IMapLoadCampaign.Measured(run, substitute))));
    }
    var otherRun = new IMapLoadCampaign.Run(run.cell(), run.pairNumber(), run.rightFirst(),
        run.workloadFingerprint(), run.settings(), bindings("b"), POLICY);
    assertThrows(IllegalArgumentException.class,
        () -> analyze(campaign, List.of(new IMapLoadCampaign.Measured(otherRun, pair))));
  }

  @Test void incompatibleSemanticAdmissionAndPreloadOrAdapterDriftAreReported() {
    var campaign = campaign(0, 100, 5, POLICY);
    var attempts = attempts(campaign, List.of(100), List.of(100), List.of(100), List.of(100));
    var second = (IMapLoadCampaign.Measured) attempts.get(1);
    var pair = second.result();
    var divergence = new OracleReport(117, LEFT, RIGHT, List.of(new SemanticDifference("snapshot", "left", "right")),
        EMPTY, FILLED);
    for (var changed : List.of(
        new FixedRateIMapLoad.Pair(pair.settings(), pair.workloadFingerprint(), pair.rightFirst(),
            divergence, EMPTY, pair.left(), pair.right()),
        new FixedRateIMapLoad.Pair(pair.settings(), pair.workloadFingerprint(), pair.rightFirst(),
            pair.admission(), FILLED, pair.left(), pair.right()),
        new FixedRateIMapLoad.Pair(pair.settings(), pair.workloadFingerprint(), pair.rightFirst(),
            admitted(OTHER, RIGHT), EMPTY, trial(OTHER, pair.left().warmup(), pair.left().measured()), pair.right()),
        new FixedRateIMapLoad.Pair(pair.settings(), pair.workloadFingerprint(), pair.rightFirst(),
            admitted(LEFT, OTHER), EMPTY, pair.left(), trial(OTHER, pair.right().warmup(), pair.right().measured())))) {
      var report = analyze(campaign, List.of(attempts.get(0), new IMapLoadCampaign.Measured(second.run(), changed)));
      assertEquals(IMapPairedAnalysis.Verdict.NOT_COMPARABLE, report.verdict());
      assertTrue(report.statistics().isEmpty());
    }
  }

  @Test void schedulerMissesErrorsAndUndefinedRatesCannotProduceAdvantage() {
    var campaign = campaign(1, 100, 5, POLICY);
    var first = first(campaign);
    var pair = first.result();
    long window = pair.left().measured().offeredWindowNanos();
    long warm = pair.left().warmup().offeredWindowNanos();
    var warmMiss = phase(1, 0, 0, 1, 0, warm, warm, Map.of(), Optional.empty());
    var warmError = phase(1, 1, 0, 0, 0, warm, warm, Map.of(ErrorClass.DEADLINE, 1), latency(100));
    var miss = phase(100, 99, 99, 1, 0, window, window, Map.of(), latency(100));
    var error = phase(100, 100, 99, 0, 0, window, window, Map.of(ErrorClass.PARTIAL, 1), latency(100));
    var none = phase(100, 0, 0, 0, 100, window, window, Map.of(), Optional.empty());
    var zero = phase(100, 100, 100, 0, 0, window, window, Map.of(), latency(0));
    var variants = List.of(
        trial(LEFT, warmMiss, pair.left().measured()), trial(LEFT, pair.left().warmup(), miss),
        trial(LEFT, warmError, pair.left().measured()), trial(LEFT, pair.left().warmup(), error),
        trial(LEFT, pair.left().warmup(), none), trial(LEFT, pair.left().warmup(), zero));
    for (int i = 0; i < variants.size(); i++) {
      var changed = pairWithTrials(pair, variants.get(i), pair.right());
      var report = analyze(campaign, List.of(new IMapLoadCampaign.Measured(first.run(), changed)));
      assertEquals(i < 2 ? IMapPairedAnalysis.Verdict.INVALID_INFRASTRUCTURE
          : IMapPairedAnalysis.Verdict.INCONCLUSIVE, report.verdict());
      assertTrue(report.statistics().isEmpty());
    }
  }

  @Test void malformedPhaseCountersOrPercentilesAreRejected() {
    var campaign = campaign(0, 100, 5, POLICY);
    var first = first(campaign);
    var pair = first.result();
    long w = pair.left().measured().offeredWindowNanos();
    var valid = latency(100);
    var invalid = List.of(
        phase(99, 100, 100, 0, 0, w, w, Map.of(), valid),
        phase(100, 100, 100, 0, 0, w + 1, w, Map.of(), valid),
        phase(100, 100, 100, 0, 0, w, -1, Map.of(), valid),
        phase(100, 100, 100, -1, 1, w, w, Map.of(), valid),
        phase(100, 99, 99, 0, 0, w, w, Map.of(), valid),
        phase(100, 100, 100, 0, 0, w, w, Map.of(), Optional.empty()),
        phase(100, 100, 99, 0, 0, w, w, Map.of(ErrorClass.NONE, 1), valid),
        phase(100, 100, 100, 0, 0, w, w, Map.of(ErrorClass.INTERNAL, 0), valid),
        phase(100, 100, 99, 0, 0, w, w, Map.of(), valid),
        phase(100, 100, 100, 0, 0, w, w, Map.of(), Optional.of(new FixedRateIMapLoad.Latencies(-1, 1, 2, 3))),
        phase(100, 100, 100, 0, 0, w, w, Map.of(), Optional.of(new FixedRateIMapLoad.Latencies(1, 4, 2, 3))));
    for (var bad : invalid) {
      var changed = pairWithTrials(pair, trial(LEFT, pair.left().warmup(), bad), pair.right());
      assertThrows(IllegalArgumentException.class,
          () -> analyze(campaign, List.of(new IMapLoadCampaign.Measured(first.run(), changed))));
    }
  }

  private static IMapLoadCampaign.Measured first(IMapLoadCampaign campaign) {
    return (IMapLoadCampaign.Measured) attempts(campaign, List.of(100), List.of(100),
        List.of(100), List.of(100)).get(0);
  }

  private static void assertVerdict(IMapPairedAnalysis.Verdict verdict, int left, int right,
      int leftP99, int rightP99) {
    var campaign = campaign(0, 100, 5, POLICY);
    assertEquals(verdict, analyze(campaign, attempts(campaign, List.of(left), List.of(right),
        List.of(leftP99), List.of(rightP99))).verdict());
  }

  static List<IMapLoadCampaign.Attempt> attempts(IMapLoadCampaign campaign,
      List<Integer> left, List<Integer> right, List<Integer> leftP99, List<Integer> rightP99) {
    var output = new ArrayList<IMapLoadCampaign.Attempt>();
    for (var run : campaign.runs()) {
      int index = run.pairNumber() - 1;
      var settings = run.settings();
      int warmup = settings.warmupRequests();
      var warm = measuredPhase(settings, warmup, warmup, 100);
      var a = measuredPhase(settings, settings.measuredRequests(), left.get(index % left.size()),
          leftP99.get(index % leftP99.size()));
      var b = measuredPhase(settings, settings.measuredRequests(), right.get(index % right.size()),
          rightP99.get(index % rightP99.size()));
      output.add(new IMapLoadCampaign.Measured(run, new FixedRateIMapLoad.Pair(settings,
          run.workloadFingerprint(), run.rightFirst(), admitted(LEFT, RIGHT), EMPTY,
          trial(LEFT, warm, a), trial(RIGHT, warm, b))));
    }
    return output;
  }

  private static OracleReport admitted(AdapterIdentity left, AdapterIdentity right) {
    return new OracleReport(117, left, right, List.of(), EMPTY, EMPTY);
  }

  private static FixedRateIMapLoad.Pair pairWithTrials(FixedRateIMapLoad.Pair pair,
      FixedRateIMapLoad.Trial left, FixedRateIMapLoad.Trial right) {
    return new FixedRateIMapLoad.Pair(pair.settings(), pair.workloadFingerprint(), pair.rightFirst(),
        pair.admission(), pair.preloaded(), left, right);
  }

  private static FixedRateIMapLoad.Trial trial(AdapterIdentity identity, FixedRateIMapLoad.Phase warmup,
      FixedRateIMapLoad.Phase measured) {
    return new FixedRateIMapLoad.Trial(identity, warmup, measured);
  }

  private static FixedRateIMapLoad.Phase measuredPhase(FixedRateIMapLoad.Settings settings,
      int scheduled, int success, int p99) {
    long window = settings.offset(scheduled);
    return phase(scheduled, success, success, 0, scheduled - success, window, window,
        Map.of(), success == 0 ? Optional.empty() : latency(p99));
  }

  private static Optional<FixedRateIMapLoad.Latencies> latency(int p99) {
    return Optional.of(new FixedRateIMapLoad.Latencies(p99 / 2, p99 * 9 / 10, p99, p99 * 11L / 10));
  }

  private static FixedRateIMapLoad.Phase phase(int scheduled, int completed, int success,
      int late, int rejected, long window, long elapsed, Map<ErrorClass, Integer> errors,
      Optional<FixedRateIMapLoad.Latencies> latency) {
    return new FixedRateIMapLoad.Phase(scheduled, completed, success, late, rejected, window, elapsed,
        errors, latency, latency, latency);
  }

  private static IMapPairedAnalysis.CellReport analyze(IMapLoadCampaign campaign,
      List<IMapLoadCampaign.Attempt> attempts) {
    return IMapPairedAnalysis.analyze(campaign, attempts).get(0);
  }
}
