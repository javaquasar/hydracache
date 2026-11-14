package io.hydracache.imap.benchmark;

import static io.hydracache.imap.benchmark.IMapLoadCampaignTest.POLICY;
import static io.hydracache.imap.benchmark.IMapLoadCampaignTest.campaign;
import static io.hydracache.imap.benchmark.IMapPairedAnalysisTest.attempts;
import static io.hydracache.imap.benchmark.IMapResourceAnalysis.Value.*;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Metric.*;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Phase.*;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Scope.*;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Side.*;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Status.*;
import static org.junit.jupiter.api.Assertions.*;

import io.hydracache.imap.benchmark.IMapResourceAnalysis.Series;
import io.hydracache.imap.benchmark.IMapResourceAnalysis.Summary;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Delta;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Metric;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Observation;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Phase;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Reading;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Report;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Scope;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Side;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.OptionalDouble;
import java.util.OptionalLong;
import org.junit.jupiter.api.Test;

final class IMapResourceAnalysisTest {
  @Test void summarizesPhaseCountersGaugesAndWorkWithoutBreakingPairs() {
    var campaign = campaign(2, 100, 5, POLICY);
    var baseline = attempts(campaign, List.of(80), List.of(100), List.of(100), List.of(100));
    long[] leftCpu = {10, 20, 30, 40, 50};
    long[] rightCpu = {11, 70, 31, 42, 45};
    var recorded = new ArrayList<IMapLoadCampaign.Attempt>();
    for (int i = 0; i < baseline.size(); i++) {
      var reports = new ArrayList<>(List.of(
          phase(PAIR, PREFLIGHT, process("driver", 4, 1024)),
          phase(LEFT, PRELOAD, process("server", 6, 1024)),
          phase(RIGHT, PRELOAD, process("server", 10, 2048)),
          phase(LEFT, WARMUP, process("server", 8, 1024)),
          phase(RIGHT, WARMUP, process("server", 10, 2048)),
          phase(LEFT, MEASUREMENT, process("server", leftCpu[i], 1024), network(0, 1600), jvm(0, 1024)),
          phase(RIGHT, MEASUREMENT, process("server", rightCpu[i], 2048), network(300, 3000), jvm(4, 2048))));
      if (baseline.get(i).run().rightFirst()) Collections.swap(reports, 5, 6);
      recorded.add(withResources(baseline.get(i), reports));
    }
    var cell = IMapPairedAnalysis.analyze(campaign, recorded).get(0);
    var previous = IMapPairedAnalysis.analyze(campaign, baseline).get(0);
    assertEquals(previous.verdict(), cell.verdict());
    assertEquals(previous.statistics(), cell.statistics());
    assertTrue(previous.resources().isEmpty());
    assertEquals(cell, IMapPairedAnalysis.analyze(campaign, recorded).get(0));
    assertEquals(recorded, cell.attempts());
    assertTrue(cell.resources().stream().noneMatch(summary -> summary.series().phase() == PREFLIGHT));
    assertThrows(UnsupportedOperationException.class, () -> cell.resources().clear());

    var cpu = summary(cell, new Series(MEASUREMENT, "server", PROCESS, CPU_NANOS, DELTA));
    assertEquals(new IMapResourceAnalysis.Comparison(30, 42, 1, -5, 50), cpu.comparison().orElseThrow());
    assertEquals(List.of(1, 2, 3, 4, 5), cpu.samples().stream().map(IMapResourceAnalysis.Sample::pairNumber).toList());
    assertValues(cpu, 10, 11);
    assertThrows(UnsupportedOperationException.class, () -> cpu.samples().clear());
    assertValues(summary(cell, new Series(MEASUREMENT, "server", PROCESS, CPU_NANOS, PER_SUCCESS)), 10 / 80.0, .11);
    assertValues(summary(cell, new Series(WARMUP, "server", PROCESS, CPU_NANOS, PER_SUCCESS)), 4, 5);
    assertValues(summary(cell, new Series(PRELOAD, "server", PROCESS, CPU_NANOS, DELTA)), 6, 10);
    assertFalse(cell.resources().stream().anyMatch(summary -> summary.series().phase() == PRELOAD
        && summary.series().value() == PER_SUCCESS));

    assertValues(summary(cell, new Series(MEASUREMENT, "server", PROCESS, RSS_BYTES, DELTA)), -24, -24);
    assertValues(summary(cell, new Series(MEASUREMENT, "server", PROCESS, RSS_BYTES, PHASE_END)), 1000, 2024);
    assertValues(summary(cell, new Series(MEASUREMENT, "server", PROCESS, PROCESS_THREADS, PHASE_END)), 2, 2);
    assertValues(summary(cell, new Series(MEASUREMENT, "wire", INTERFACE, RX_BYTES, PER_SUCCESS)), 0, 3);
    assertValues(summary(cell, new Series(MEASUREMENT, "wire", INTERFACE, TX_BYTES, PER_SUCCESS)), 20, 30);
    assertValues(summary(cell, new Series(MEASUREMENT, "wire", INTERFACE, RX_PACKETS, PER_SUCCESS)), 0, .03);
    assertValues(summary(cell, new Series(MEASUREMENT, "wire", INTERFACE, TX_PACKETS, PER_SUCCESS)), .2, .3);
    assertValues(summary(cell, new Series(MEASUREMENT, "runtime", JVM, JVM_HEAP_USED_BYTES, DELTA)), -10, -10);
    assertValues(summary(cell, new Series(MEASUREMENT, "runtime", JVM, JVM_HEAP_COMMITTED_BYTES, PHASE_END)), 2048, 4096);
    assertValues(summary(cell, new Series(MEASUREMENT, "runtime", JVM, JVM_THREADS, PHASE_END)), 8, 8);
    assertValues(summary(cell, new Series(MEASUREMENT, "runtime", JVM, JVM_GC_COLLECTIONS, DELTA)), 0, 4);
    assertValues(summary(cell, new Series(MEASUREMENT, "runtime", JVM, JVM_GC_MILLIS, DELTA)), 0, 8);
  }

  @Test void keepsUnmatchedUnavailableAndDiscontinuousSamplesWithoutEstimatingEffects() {
    var campaign = campaign(0, 100, 5, POLICY);
    var baseline = attempts(campaign, List.of(100), List.of(100), List.of(100), List.of(100));
    var reset = new Delta(OptionalLong.of(10), OptionalLong.of(1), OptionalLong.empty(), COUNTER_RESET);
    var unavailable = new Delta(OptionalLong.empty(), OptionalLong.of(1), OptionalLong.empty(), UNAVAILABLE);
    var changed = new Delta(OptionalLong.of(10), OptionalLong.of(11), OptionalLong.empty(), SOURCE_CHANGED);
    var observations = List.of(process("server", 0, 100),
        observation("server", PROCESS, Map.of(CPU_NANOS, reset)),
        observation("server", PROCESS, Map.of(CPU_NANOS, unavailable)),
        observation("server", PROCESS, Map.of(CPU_NANOS, changed)),
        observation("server", JVM, Map.of(JVM_THREADS, available(8, 8))));
    var recorded = new ArrayList<IMapLoadCampaign.Attempt>();
    for (int i = 0; i < baseline.size(); i++) {
      var reports = new ArrayList<>(List.of(
          phase(LEFT, WARMUP, process("server", 1, 100)),
          phase(RIGHT, WARMUP, process("server", 1, 100)),
          phase(LEFT, MEASUREMENT, observations.get(i)),
          phase(RIGHT, MEASUREMENT, process("server", 1, 100), process("secondary", 1, 100))));
      if (i == 0) {
        reports.add(phase(LEFT, PRELOAD, process("server", 1, 100)));
        reports.add(phase(RIGHT, PRELOAD, process("server", 1, 100)));
      }
      recorded.add(withResources(baseline.get(i), reports));
    }
    var cell = IMapPairedAnalysis.analyze(campaign, recorded).get(0);
    assertEquals(IMapPairedAnalysis.Verdict.EQUIVALENT, cell.verdict());
    var cpu = summary(cell, new Series(MEASUREMENT, "server", PROCESS, CPU_NANOS, DELTA));
    assertEquals(OptionalDouble.of(0), cpu.samples().get(0).left());
    for (int i = 1; i < 5; i++) {
      assertTrue(cpu.samples().get(i).left().isEmpty());
      assertEquals(OptionalDouble.of(1), cpu.samples().get(i).right());
    }
    assertTrue(cpu.comparison().isEmpty());
    assertTrue(summary(cell, new Series(MEASUREMENT, "server", PROCESS, CPU_NANOS, PER_SUCCESS)).comparison().isEmpty());
    assertTrue(summary(cell, new Series(MEASUREMENT, "secondary", PROCESS, CPU_NANOS, DELTA)).comparison().isEmpty());
    assertTrue(summary(cell, new Series(MEASUREMENT, "server", JVM, JVM_THREADS, PHASE_END)).comparison().isEmpty());
    var sparse = summary(cell, new Series(PRELOAD, "server", PROCESS, CPU_NANOS, DELTA));
    assertEquals(1, sparse.samples().size());
    assertValues(sparse, 1, 1);
    assertTrue(sparse.comparison().isEmpty());
    var emptyWarmup = summary(cell, new Series(WARMUP, "server", PROCESS, CPU_NANOS, PER_SUCCESS));
    assertTrue(emptyWarmup.samples().stream().allMatch(sample -> sample.left().isEmpty() && sample.right().isEmpty()));
    assertTrue(emptyWarmup.comparison().isEmpty());
    assertEquals(recorded, cell.attempts());
  }

  @Test void failedIncompleteAndInvalidCellsKeepResourcesButSkipComparisons() {
    var campaign = campaign(0, 100, 5, POLICY);
    var measured = attempts(campaign, List.of(100), List.of(100), List.of(100), List.of(100));
    var left = phase(LEFT, MEASUREMENT, process("server", 4, 100));
    var right = phase(RIGHT, MEASUREMENT, process("server", 2, 100));
    var interrupted = new Report(RIGHT, MEASUREMENT, 10, false, right.observations());
    var completed = withResources(measured.get(0), List.of(left, right));
    var failed = new IMapLoadCampaign.Failed(measured.get(1).run(), IMapLoadCampaign.Failure.INFRASTRUCTURE,
        "drain timed out", List.of(left, interrupted));
    var invalid = attempts(campaign, List.of(0), List.of(100), List.of(100), List.of(100));
    for (var recorded : List.of(List.of(completed), List.of(completed, failed),
        List.of(withResources(invalid.get(0), List.of(left, right))))) {
      var cell = IMapPairedAnalysis.analyze(campaign, recorded).get(0);
      assertTrue(cell.statistics().isEmpty());
      assertEquals(recorded, cell.attempts());
      assertFalse(cell.resources().isEmpty());
      assertTrue(cell.resources().stream().allMatch(summary -> summary.comparison().isEmpty()));
      var cpu = summary(cell, new Series(MEASUREMENT, "server", PROCESS, CPU_NANOS, DELTA));
      assertValues(cpu, 4, 2);
    }
    var failure = IMapPairedAnalysis.analyze(campaign, List.of(completed, failed)).get(0);
    var delta = summary(failure, new Series(MEASUREMENT, "server", PROCESS, CPU_NANOS, DELTA));
    assertEquals(OptionalDouble.of(4), delta.samples().get(1).left());
    assertTrue(delta.samples().get(1).right().isEmpty());
    assertTrue(summary(failure, new Series(MEASUREMENT, "server", PROCESS, CPU_NANOS, PER_SUCCESS))
        .samples().get(1).left().isEmpty());
  }

  @Test void duplicateObservationsInTheSameSideAndPhaseAreRejected() {
    var campaign = campaign(0, 100, 5, POLICY);
    var first = attempts(campaign, List.of(100), List.of(100), List.of(100), List.of(100)).get(0);
    var phase = phase(LEFT, MEASUREMENT, process("server", 4, 100));
    var duplicate = withResources(first, List.of(phase, phase));
    assertThrows(IllegalArgumentException.class, () -> IMapPairedAnalysis.analyze(campaign, List.of(duplicate)));
  }

  private static IMapLoadCampaign.Attempt withResources(IMapLoadCampaign.Attempt attempt, List<Report> reports) {
    var measured = (IMapLoadCampaign.Measured) attempt;
    return new IMapLoadCampaign.Measured(measured.run(), measured.result(), reports);
  }

  private static Summary summary(IMapPairedAnalysis.CellReport cell, Series key) {
    return cell.resources().stream().filter(summary -> summary.series().equals(key)).findFirst().orElseThrow();
  }

  private static void assertValues(Summary summary, double left, double right) {
    assertEquals(OptionalDouble.of(left), summary.samples().get(0).left());
    assertEquals(OptionalDouble.of(right), summary.samples().get(0).right());
  }

  private static Report phase(Side side, Phase phase, Observation... observations) {
    return new Report(side, phase, 10, true, List.of(observations));
  }

  private static Observation process(String label, long cpu, long rss) {
    return observation(label, PROCESS, Map.of(CPU_NANOS, available(0, cpu),
        RSS_BYTES, available(rss, rss - 24), PROCESS_THREADS, available(3, 2)));
  }

  private static Observation network(long rx, long tx) {
    return observation("wire", INTERFACE, Map.of(RX_BYTES, available(0, rx), TX_BYTES, available(0, tx),
        RX_PACKETS, available(0, rx / 100), TX_PACKETS, available(0, tx / 100)));
  }

  private static Observation jvm(long gc, long heap) {
    return observation("runtime", JVM, Map.of(JVM_HEAP_USED_BYTES, available(heap, heap - 10),
        JVM_HEAP_COMMITTED_BYTES, available(heap * 2, heap * 2), JVM_THREADS, available(8, 8),
        JVM_GC_COLLECTIONS, available(0, gc), JVM_GC_MILLIS, available(0, gc * 2)));
  }

  private static Observation observation(String label, Scope scope, Map<Metric, Delta> metrics) {
    var reading = new Reading(0, Optional.empty(), "fixture uses the recorded deltas");
    return new Observation(label, scope, reading, reading, metrics);
  }

  private static Delta available(long before, long after) {
    return new Delta(OptionalLong.of(before), OptionalLong.of(after), OptionalLong.of(after - before), AVAILABLE);
  }
}
