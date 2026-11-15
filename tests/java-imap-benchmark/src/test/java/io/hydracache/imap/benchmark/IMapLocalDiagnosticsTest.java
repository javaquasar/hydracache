package io.hydracache.imap.benchmark;

import static org.junit.jupiter.api.Assertions.*;

import io.hydracache.imap.benchmark.IMapLocalDiagnostics.Controls;
import io.hydracache.imap.benchmark.IMapPairedAnalysis.Verdict;
import io.hydracache.imap.semantic.BytesValue;
import io.hydracache.imap.semantic.ScenarioStep;
import java.nio.file.FileAlreadyExistsException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.concurrent.CancellationException;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

final class IMapLocalDiagnosticsTest {
  @TempDir Path directory;

  @Test void ladderRepeatsTheSameAdapterAndRetainsAnActualSlowdownCapture() throws Exception {
    var contract = Files.writeString(directory.resolve("contract.toml"), "provisional");
    var output = directory.resolve("controls");
    IMapLocalCampaign.launch(new String[] {contract.toString(), output.toString(), "117", "25", "1", "2",
        "5", "1", "2", "0", "25,50", "15"}, (linux, process) -> IMapResourceMonitor.Targets.NONE);
    String aa = Files.readString(output.resolve("aa/manifest.properties"));
    String slowdown = Files.readString(output.resolve("slowdown/manifest.properties"));
    assertTrue(aa.contains("pairs.retained=10"), aa);
    assertTrue(slowdown.contains("pairs.retained=5"), slowdown);
    assertTrue(Files.readString(output.resolve("aa/report.txt")).contains("CONTRACT_NOT_ADMITTED"));
    var histograms = Files.readAllLines(output.resolve("slowdown/histograms.tsv"));
    var rightService = histograms.stream().map(line -> line.split("\t"))
        .filter(row -> row[1].equals("right") && row[2].equals("measurement") && row[3].equals("service"))
        .mapToLong(row -> Long.parseLong(row[4])).toArray();
    assertTrue(rightService.length >= 5);
    assertTrue(java.util.Arrays.stream(rightService).allMatch(bound -> bound >= 15_000_000));
    String report = Files.readString(output.resolve("controls.txt"));
    assertTrue(report.contains("A/A reads-r25-w1-i2"), report);
    assertTrue(report.contains("A/A reads-r50-w1-i2"), report);
    assertTrue(report.contains("Slowdown 15 ms reads-r25-w1-i2"), report);
    assertTrue(report.contains("p99 Metric"), report);
    assertTrue(report.contains("p999 Metric"), report);
  }

  @Test void controlsValidateRatesAndDelayedSideBeforeCollectingEvidence() throws Exception {
    for (String rates : List.of("25", "1,2,3,4,5,6,7,8,9", "0,25", "25,25", "50,25", "25,")) {
      assertThrows(IllegalArgumentException.class, () -> Controls.parse(rates, "15"));
    }
    assertThrows(IllegalArgumentException.class, () -> Controls.parse("25,50", "0"));
    assertThrows(IllegalArgumentException.class, () -> Controls.parse("25,50", "251"));
    var contract = Files.writeString(directory.resolve("contract.toml"), "provisional");
    assertThrows(FileAlreadyExistsException.class, () -> IMapLocalCampaign.main(
        new String[] {contract.toString(), directory.toString(), "117", "25", "1", "2",
            "5", "1", "2", "0", "25,50", "15"}));
  }

  @Test void equivalenceAloneSupportsRepeatabilityAndInterruptCancelsDelayWithoutAnOperation() {
    assertEquals(Verdict.EQUIVALENT, IMapLocalDiagnostics.repeatability(Verdict.EQUIVALENT));
    for (var verdict : List.of(Verdict.INCONCLUSIVE, Verdict.LEFT_ADVANTAGE, Verdict.RIGHT_ADVANTAGE,
        Verdict.INVALID_INFRASTRUCTURE, Verdict.NOT_COMPARABLE)) {
      assertEquals(Verdict.INCONCLUSIVE, IMapLocalDiagnostics.repeatability(verdict));
    }
    try (var adapter = new IMapLocalDiagnostics.SlowAdapter(IMapLocalCampaign.reference(), 15)) {
      Thread.currentThread().interrupt();
      try {
        assertThrows(CancellationException.class, () ->
            adapter.execute(ScenarioStep.get("cancelled", BytesValue.fromHex("01"))));
      } finally {
        assertTrue(Thread.interrupted());
      }
      assertEquals(0, adapter.snapshot().liveCardinality());
    }
  }
}
