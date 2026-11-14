package io.hydracache.imap.benchmark;

import static io.hydracache.imap.benchmark.IMapLoadCampaignTest.BINDINGS;
import static io.hydracache.imap.benchmark.IMapLoadCampaignTest.POLICY;
import static io.hydracache.imap.benchmark.IMapLoadCampaignTest.READS;
import static io.hydracache.imap.benchmark.IMapLoadCampaignTest.SERIAL;
import static org.junit.jupiter.api.Assertions.*;

import io.hydracache.imap.semantic.InMemorySemanticAdapter;
import io.hydracache.imap.semantic.ErrorClass;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

final class IMapPre074CampaignTest {
  @Test void executionCapturesActualTrialWindowsAndPostIdleResources() {
    var campaign = IMapLoadCampaignTest.campaign(0, 1, 5, POLICY);
    var id = new AtomicInteger();
    var samples = new AtomicInteger();
    var hosts = new IMapCampaignPublication.Hosts(List.of("b".repeat(64)), "c".repeat(64), true);
    var calibration = new IMapCampaignPublication.Calibration(1000, 1000);
    var source = new IMapCampaignCapture.Source() {
      public IMapLoadCampaign.Bindings observedBindings() { return BINDINGS; }
      public IMapCampaignPublication.Hosts hosts() { return hosts; }
      public String startupSha256(IMapResourceMonitor.Side side, io.hydracache.imap.semantic.MapSemanticAdapter adapter) {
        return String.format("%064x", id.incrementAndGet());
      }
      public Optional<IMapCampaignPublication.Calibration> calibrate(IMapResourceMonitor.Side side) {
        return samples.incrementAndGet() == 1 ? Optional.empty() : Optional.of(calibration);
      }
    };
    var targets = new IMapResourceMonitor.Targets(List.of(IMapResourceProbes.currentJvm("load")), List.of(), List.of());
    var results = campaign.executeCaptured(run -> InMemorySemanticAdapter.strict("left"),
        run -> InMemorySemanticAdapter.strict("right"), () -> false, run -> targets, run -> source, 1);
    assertEquals(5, results.size());
    assertEquals(10, id.get());
    assertEquals(20, samples.get());
    for (var row : results) {
      var capture = row.capture().orElseThrow();
      var attempt = assertInstanceOf(IMapLoadCampaign.Measured.class, row.attempt());
      var first = attempt.run().rightFirst() ? capture.right() : capture.left();
      var second = attempt.run().rightFirst() ? capture.left() : capture.right();
      assertTrue(first.measurementEndNanos() <= second.measurementStartNanos());
      assertTrue(second.before().orElseThrow().sampledAtNanos() <= second.measurementStartNanos());
      assertTrue(second.after().orElseThrow().sampledAtNanos() >= second.measurementEndNanos());
      assertEquals(2, attempt.resources().stream().filter(r -> r.phase() == IMapResourceMonitor.Phase.POST_IDLE).count());
      assertEquals(1, attempt.result().left().measured().histograms().orElseThrow().endToEnd().samples());
    }
    var first = (IMapLoadCampaign.Measured) results.get(0).attempt();
    var registration = new IMapCampaignPublication.Registration("d".repeat(64), IMapCampaignPublication.Admission.ADMITTED,
        campaign, hosts, first.result().left().adapter(), first.result().right().adapter(),
        new IMapCampaignPublication.CalibrationPolicy(calibration, 0), List.of());
    var manifest = IMapCampaignPublication.seal(registration, results);
    assertTrue(IMapCampaignPublication.assess(registration, manifest.sha256(), manifest, results).blockers().stream()
        .anyMatch(blocker -> blocker.reason() == IMapCampaignPublication.Refusal.CALIBRATION));
    assertThrows(IllegalArgumentException.class, () -> campaign.executeCaptured(null, null, null, null, null, -1));
    assertThrows(IllegalArgumentException.class, () -> campaign.executeCaptured(null, null, null, null, null, 60_001));
  }

  @Test void aaUsesTheSameAdapterAndSaturationStopsAtTheFirstUnstableRate() {
    var starts = new AtomicInteger();
    var aa = IMapLoadControls.sameAdapter(IMapLoadCampaignTest.campaign(0, 1, 5, POLICY), run -> {
      starts.incrementAndGet();
      return InMemorySemanticAdapter.strict("same");
    }, () -> false);
    assertEquals(20, starts.get());
    assertEquals(5, aa.get(0).attempts().size());

    var plan = ladder(List.of(1, 2, 4));
    var saturated = synthetic(plan, List.of(100), List.of(100));
    var exhausted = synthetic(plan, List.of(99), List.of(100));
    saturated.set(5, exhausted.get(5));
    var result = IMapLoadControls.saturation(plan, saturated);
    assertEquals(1, result.lastSustainableRate().orElseThrow());
    assertEquals(List.of(IMapLoadControls.Limit.STABLE, IMapLoadControls.Limit.CAPACITY, IMapLoadControls.Limit.STABLE),
        result.steps().stream().map(IMapLoadControls.Step::limit).toList());
    assertEquals(IMapLoadControls.Limit.INCOMPLETE, IMapLoadControls.saturation(plan, List.of()).steps().get(0).limit());
    assertEquals(IMapLoadControls.Limit.LATENCY,
        IMapLoadControls.saturation(plan, synthetic(plan, List.of(100), List.of(2000))).steps().get(0).limit());
    assertThrows(IllegalArgumentException.class, () -> IMapLoadControls.saturation(ladder(List.of(2, 1)), List.of()));
    var mixed = IMapLoadCampaign.plan(List.of(READS), List.of(1),
        List.of(SERIAL, new IMapLoadCampaign.Parallelism(2, 2)), new FixedRateIMapLoad.Settings(117, 2, 0, 100,
            1, 1, 1000), 5, BINDINGS, POLICY);
    assertThrows(IllegalArgumentException.class, () -> IMapLoadControls.saturation(mixed, List.of()));
    var first = saturated.get(0).run();
    assertEquals(IMapLoadControls.Limit.INFRASTRUCTURE, IMapLoadControls.saturation(plan,
        List.of(new IMapLoadCampaign.Failed(first, IMapLoadCampaign.Failure.INFRASTRUCTURE, "failed")))
        .steps().get(0).limit());
  }

  @Test void independentAnalysisRefusesRawHistogramWithWrongCounts() {
    var plan = ladder(List.of(1));
    var attempts = synthetic(plan, List.of(100), List.of(100));
    var first = (IMapLoadCampaign.Measured) attempts.get(0);
    var pair = first.result();
    var baseline = pair.left().measured();
    var empty = new FixedRateIMapLoad.Histogram(0, List.of());
    var bad = new FixedRateIMapLoad.Phase(baseline.scheduled(), baseline.completed(), baseline.successful(),
        0, 0, baseline.offeredWindowNanos(), baseline.elapsedNanos(), Map.of(),
        baseline.endToEnd(), baseline.queue(), baseline.service(),
        Optional.of(new FixedRateIMapLoad.Histograms(empty, empty, empty)));
    var changed = new FixedRateIMapLoad.Pair(pair.settings(), pair.workloadFingerprint(), pair.rightFirst(),
        pair.admission(), pair.preloaded(), new FixedRateIMapLoad.Trial(pair.left().adapter(), pair.left().warmup(), bad),
        pair.right());
    attempts.set(0, new IMapLoadCampaign.Measured(first.run(), changed));
    assertThrows(IllegalArgumentException.class, () -> IMapPairedAnalysis.analyze(plan, attempts));
    var different = new FixedRateIMapLoad.Histogram(100, List.of(new FixedRateIMapLoad.Bucket(10, 100)));
    assertThrows(IllegalArgumentException.class, () -> IMapPairedAnalysis.analyze(plan, List.of(
        trialChanged(first, new FixedRateIMapLoad.Phase(100, 100, 100, 0, 0, baseline.offeredWindowNanos(),
            baseline.elapsedNanos(), Map.of(), baseline.endToEnd(), baseline.queue(), baseline.service(),
            Optional.of(new FixedRateIMapLoad.Histograms(different, different, different)))))));
  }

  @Test void saturationExplainsMoreThanCapacityAndChecksCellShape() {
    var plan = ladder(List.of(1));
    var first = (IMapLoadCampaign.Measured) synthetic(plan, List.of(100), List.of(100)).get(0);
    var base = first.result().left().measured();
    long elapsed = base.elapsedNanos();
    var error = new FixedRateIMapLoad.Phase(100, 100, 99, 0, 0, elapsed, elapsed,
        Map.of(ErrorClass.INTERNAL, 1), base.endToEnd(), base.queue(), base.service());
    assertEquals(IMapLoadControls.Limit.OPERATION_ERRORS,
        IMapLoadControls.saturation(plan, List.of(trialChanged(first, error))).steps().get(0).limit());
    var late = new FixedRateIMapLoad.Phase(100, 99, 99, 1, 0, elapsed, elapsed,
        Map.of(), base.endToEnd(), base.queue(), base.service());
    assertEquals(IMapLoadControls.Limit.INFRASTRUCTURE,
        IMapLoadControls.saturation(plan, List.of(trialChanged(first, late))).steps().get(0).limit());
    var tail = Optional.of(new FixedRateIMapLoad.Latencies(50, 100, 100, 1501));
    var tailPhase = new FixedRateIMapLoad.Phase(100, 100, 100, 0, 0, elapsed, elapsed,
        Map.of(), tail, base.queue(), base.service());
    assertEquals(IMapLoadControls.Limit.LATENCY,
        IMapLoadControls.saturation(plan, List.of(trialChanged(first, tailPhase))).steps().get(0).limit());
    var noncomparable = new ArrayList<>(synthetic(plan, List.of(100), List.of(100)));
    var pair = first.result();
    var oracle = new io.hydracache.imap.semantic.OracleReport(pair.admission().seed(), pair.admission().leftIdentity(),
        pair.admission().rightIdentity(), List.of(new io.hydracache.imap.semantic.SemanticDifference("key", "a", "b")),
        pair.admission().leftSnapshot(), pair.admission().rightSnapshot());
    noncomparable.set(0, new IMapLoadCampaign.Measured(first.run(), new FixedRateIMapLoad.Pair(pair.settings(),
        pair.workloadFingerprint(), pair.rightFirst(), oracle, pair.preloaded(), pair.left(), pair.right())));
    assertEquals(IMapLoadControls.Limit.INFRASTRUCTURE,
        IMapLoadControls.saturation(plan, noncomparable).steps().get(0).limit());
  }

  private static IMapLoadCampaign.Measured trialChanged(IMapLoadCampaign.Measured attempt, FixedRateIMapLoad.Phase phase) {
    var pair = attempt.result();
    return new IMapLoadCampaign.Measured(attempt.run(), new FixedRateIMapLoad.Pair(pair.settings(),
        pair.workloadFingerprint(), pair.rightFirst(), pair.admission(), pair.preloaded(),
        new FixedRateIMapLoad.Trial(pair.left().adapter(), pair.left().warmup(), phase), pair.right()));
  }

  private static IMapLoadCampaign ladder(List<Integer> rates) {
    return IMapLoadCampaign.plan(List.of(READS), rates, List.of(SERIAL),
        new FixedRateIMapLoad.Settings(117, 2, 0, 100, 1, 1, 1000), 5, BINDINGS, POLICY);
  }

  private static ArrayList<IMapLoadCampaign.Attempt> synthetic(IMapLoadCampaign plan, List<Integer> successes, List<Integer> latency) {
    return new ArrayList<>(IMapPairedAnalysisTest.attempts(plan, successes, successes, latency, latency));
  }
}
