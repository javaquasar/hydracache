package io.hydracache.imap.benchmark;

import static io.hydracache.imap.benchmark.IMapLoadCampaignTest.POLICY;
import static io.hydracache.imap.benchmark.IMapLoadCampaignTest.campaign;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Metric.*;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Phase.*;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Scope.*;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Side.*;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Status.*;
import static org.junit.jupiter.api.Assertions.*;

import io.hydracache.imap.benchmark.IMapResourceMonitor.Probe;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Recorder;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Report;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Snapshot;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Targets;
import io.hydracache.imap.semantic.HydraCacheSemanticAdapter;
import io.hydracache.imap.semantic.InMemorySemanticAdapter;
import java.io.IOException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.OptionalLong;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.AtomicReference;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Timeout;

@Timeout(15)
final class IMapResourceMonitorTest {
  @Test void keepsGaugeAndCounterChangesSeparateFromMissingOrResetValues() throws Exception {
    var current = new AtomicReference<>(new Snapshot("pid:1", Map.of(CPU_NANOS, 100L, RSS_BYTES, 200L)));
    var network = new AtomicReference<>(new Snapshot("interface:1", Map.of(RX_BYTES, 99L, TX_BYTES, 20L)));
    var reports = new ArrayList<Report>();
    var monitor = new Recorder(new Targets(List.of(
        new Probe("client", PROCESS, current::get), new Probe("nic", INTERFACE, network::get)),
        List.of(), List.of()), reports);
    assertEquals("done", monitor.around(LEFT, MEASUREMENT, () -> {
      current.set(new Snapshot("pid:1", Map.of(CPU_NANOS, 125L, RSS_BYTES, 150L)));
      network.set(new Snapshot("interface:1", Map.of(RX_BYTES, 3L, TX_BYTES, 34L)));
      return "done";
    }));
    var report = reports.get(0);
    assertTrue(report.completed());
    assertTrue(report.elapsedNanos() >= 0);
    var process = report.observations().get(0);
    assertEquals(25, process.metrics().get(CPU_NANOS).change().orElseThrow());
    assertEquals(-50, process.metrics().get(RSS_BYTES).change().orElseThrow());
    assertEquals(UNAVAILABLE, process.metrics().get(PROCESS_THREADS).status());
    assertEquals(AVAILABLE, process.metrics().get(RSS_BYTES).status());
    assertEquals(OptionalLong.empty(), process.metrics().get(PROCESS_THREADS).before());
    assertTrue(process.before().snapshot().isPresent());
    assertTrue(process.after().sampledAtNanos() - process.before().sampledAtNanos() >= 0);
    var traffic = report.observations().get(1);
    assertEquals(COUNTER_RESET, traffic.metrics().get(RX_BYTES).status());
    assertEquals(OptionalLong.empty(), traffic.metrics().get(RX_BYTES).change());
    assertEquals(14, traffic.metrics().get(TX_BYTES).change().orElseThrow());
    assertFalse(process.metrics().containsKey(TX_BYTES));

    monitor.around(RIGHT, PRELOAD, () -> {
      current.set(new Snapshot("pid:2", Map.of(CPU_NANOS, 20L)));
      return null;
    });
    var replaced = reports.get(1).observations().get(0).metrics().get(CPU_NANOS);
    assertEquals(SOURCE_CHANGED, replaced.status());
    assertEquals(OptionalLong.of(125), replaced.before());
    assertEquals(OptionalLong.of(20), replaced.after());
    assertTrue(replaced.change().isEmpty());
  }

  @Test void reportsFailedSamplesAndRetainsFailedPhase() throws Exception {
    var calls = new AtomicLong();
    var bad = new Probe("broken", PROCESS, () -> {
      if (calls.getAndIncrement() == 0) throw new IOException("denied".repeat(100));
      return new Snapshot("wrong-scope", Map.of(JVM_THREADS, 10L));
    });
    var afterOnly = new AtomicLong();
    var intermittent = new Probe("intermittent", PROCESS, () -> {
      if (afterOnly.getAndIncrement() != 0) throw new IOException("owner exited");
      return new Snapshot("pid", Map.of(CPU_NANOS, 1L));
    });
    var reports = new ArrayList<Report>();
    var recorder = new Recorder(new Targets(List.of(bad, intermittent), List.of(), List.of()), reports);
    assertThrows(TimeoutException.class, () -> recorder.around(PAIR, PREFLIGHT, () -> {
      throw new TimeoutException("bounded preflight");
    }));
    var report = reports.get(0);
    assertFalse(report.completed());
    var reading = report.observations().get(0);
    assertEquals(256, reading.before().error().length());
    assertTrue(reading.after().error().contains("wrong scope"));
    assertEquals(UNAVAILABLE, reading.metrics().get(CPU_NANOS).status());
    var exit = report.observations().get(1).metrics().get(CPU_NANOS);
    assertEquals(UNAVAILABLE, exit.status());
    assertEquals(1, exit.before().orElseThrow());
    assertTrue(exit.after().isEmpty());
  }

  @Test void boundsProbeNamesCountsAndReadings() {
    assertThrows(IllegalArgumentException.class, () -> new Probe("../bad", PROCESS, () -> null));
    assertThrows(IllegalArgumentException.class, () -> new Snapshot("", Map.of()));
    assertThrows(IllegalArgumentException.class, () -> new Snapshot("a".repeat(193), Map.of()));
    assertThrows(IllegalArgumentException.class, () -> new Snapshot("p", Map.of(CPU_NANOS, -1L)));
    var one = new Probe("one", PROCESS, () -> new Snapshot("p", Map.of()));
    assertThrows(IllegalArgumentException.class,
        () -> new Targets(Collections.nCopies(9, one), List.of(), List.of()));
    assertThrows(IllegalArgumentException.class, () -> new Targets(List.of(one), List.of(one), List.of()));
  }

  @Test void campaignKeepsSnapshotsInActualPhaseAndSideOrder() {
    var campaign = campaign(0, 1, 5, POLICY);
    var targets = new Targets(List.of(ticking("driver")), List.of(ticking("left-server")),
        List.of(ticking("right-server")));
    var attempts = campaign.execute(run -> InMemorySemanticAdapter.strict("left"),
        run -> InMemorySemanticAdapter.strict("right"), () -> false, run -> targets);
    assertEquals(5, attempts.size());
    for (var attempt : attempts) {
      assertInstanceOf(IMapLoadCampaign.Measured.class, attempt);
      var reports = attempt.resources();
      assertEquals(List.of(PREFLIGHT, PRELOAD, PRELOAD, WARMUP, MEASUREMENT, CLEANUP,
          WARMUP, MEASUREMENT, CLEANUP), reports.stream().map(Report::phase).toList());
      var first = attempt.run().rightFirst() ? RIGHT : LEFT;
      var second = attempt.run().rightFirst() ? LEFT : RIGHT;
      assertEquals(List.of(PAIR, LEFT, RIGHT, first, first, first, second, second, second),
          reports.stream().map(Report::side).toList());
      assertTrue(reports.stream().allMatch(Report::completed));
      assertEquals(List.of("driver"), reports.get(0).observations().stream()
          .map(IMapResourceMonitor.Observation::label).toList());
      assertEquals(List.of("driver", "left-server"), reports.get(1).observations().stream()
          .map(IMapResourceMonitor.Observation::label).toList());
      assertEquals(List.of("driver", "right-server"), reports.get(2).observations().stream()
          .map(IMapResourceMonitor.Observation::label).toList());
      assertEquals(1, reports.get(4).observations().get(0).metrics().get(CPU_NANOS).change().orElseThrow());
      assertThrows(UnsupportedOperationException.class, reports::clear);
    }
    var cell = IMapPairedAnalysis.analyze(campaign, attempts).get(0);
    assertEquals(attempts, cell.attempts());
    var cpu = new IMapResourceAnalysis.Series(MEASUREMENT, "driver", PROCESS, CPU_NANOS,
        IMapResourceAnalysis.Value.PER_SUCCESS);
    var summary = cell.resources().stream().filter(item -> item.series().equals(cpu)).findFirst().orElseThrow();
    assertEquals(new IMapResourceAnalysis.Comparison(1, 1, 0, 0, 0), summary.comparison().orElseThrow());
  }

  @Test void campaignRetainsResourceEvidenceWhenPreflightRefuses() {
    var campaign = campaign(0, 1, 5, POLICY);
    var attempts = campaign.execute(run -> new HydraCacheSemanticAdapter(),
        run -> InMemorySemanticAdapter.strict("reference"), () -> false,
        run -> new Targets(List.of(ticking("driver")), List.of(), List.of()));
    for (var attempt : attempts) {
      assertEquals(IMapLoadCampaign.Failure.NOT_COMPARABLE,
          assertInstanceOf(IMapLoadCampaign.Failed.class, attempt).failure());
      assertEquals(1, attempt.resources().size());
      var phase = attempt.resources().get(0);
      assertEquals(PREFLIGHT, phase.phase());
      assertFalse(phase.completed());
      assertEquals(1, phase.observations().get(0).metrics().get(CPU_NANOS).change().orElseThrow());
    }
  }

  private static Probe ticking(String label) {
    var time = new AtomicLong();
    return new Probe(label, PROCESS, () -> new Snapshot(label + ":pid", Map.of(CPU_NANOS, time.getAndIncrement())));
  }
}
