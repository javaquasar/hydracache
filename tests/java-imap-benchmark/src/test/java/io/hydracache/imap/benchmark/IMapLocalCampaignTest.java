package io.hydracache.imap.benchmark;

import static org.junit.jupiter.api.Assertions.*;

import io.hydracache.imap.benchmark.IMapCampaignPublication.Calibration;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Side;
import io.hydracache.imap.semantic.InMemorySemanticAdapter;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.FileAlreadyExistsException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;
import java.util.Properties;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

final class IMapLocalCampaignTest {
  private static final Calibration CALIBRATION = new Calibration(1_000, 100);
  private static final FixedRateIMapLoad.Settings SETTINGS =
      new FixedRateIMapLoad.Settings(117, 100, 0, 1, 1, 1, 5000);

  @TempDir Path directory;

  @Test void commandRetainsHashBoundRawEvidenceAndDiagnosticReport() throws Exception {
    var contract = Files.writeString(directory.resolve("contract.toml"), "status = \"provisional\"\n");
    var output = directory.resolve("capture");
    IMapLocalCampaign.main(new String[] {contract.toString(), output.toString(), "117", "100", "1", "2",
        "5", "1", "1", "0"});
    var index = readIndex(output);
    assertEquals("local-diagnostic", index.getProperty("scope"));
    assertEquals("2", index.getProperty("manifest.version"));
    assertEquals("5", index.getProperty("pairs.expected"));
    assertEquals("5", index.getProperty("pairs.retained"));
    checkHashes(output, index);
    String histogram = Files.readString(output.resolve("histograms.tsv"));
    assertTrue(histogram.contains("1\tleft\twarmup\tend-to-end\t"));
    assertTrue(histogram.contains("5\tright\tmeasurement\tservice\t"));
    String report = Files.readString(output.resolve("report.txt"));
    assertTrue(report.contains("CONTRACT_NOT_ADMITTED"), report);
    assertTrue(report.contains("dedicated host placement required"), report);
    assertTrue(report.contains("JVM_HEAP_USED_BYTES"), report);
    assertEquals(4, IMapLocalCampaign.targets(true, ProcessHandle.current()).shared().size());
    assertEquals(2, IMapLocalCampaign.targets(false, ProcessHandle.current()).shared().size());
  }

  @Test void provisionalDiagnosticsRetainMissingCalibrationChangedArtifactAndAbsentPhases() throws Exception {
    var left = Files.writeString(directory.resolve("left.jar"), "original");
    var samples = new AtomicInteger();
    var capture = source(left, () -> {
      if (samples.incrementAndGet() == 2) {
        Files.writeString(left, "changed during the first measurement");
        throw new IOException("calibration unavailable");
      }
      return CALIBRATION;
    });
    var output = directory.resolve("changed");
    IMapLocalCampaign.run(output, new IMapLocalCampaign.Options(SETTINGS, 5, 1), capture,
        IMapResourceMonitor.Targets.NONE, () -> false);
    assertEquals(21, samples.get());
    var report = Files.readString(output.resolve("report.txt"));
    assertTrue(report.contains("observed campaign bindings differ"), report);
    assertTrue(report.contains("before or after sample missing"), report);
    assertTrue(report.contains("phase=POST_IDLE"), report);
    assertTrue(report.contains("phase=MEASUREMENT"), report);
    assertTrue(report.contains("CONTRACT_NOT_ADMITTED"), report);
    checkHashes(output, readIndex(output));
  }

  @Test void cancellationRetainsItsFailedSlotAndNeverOverwritesEvidence() throws Exception {
    var input = Files.writeString(directory.resolve("fixture"), "bytes");
    var capture = source(input, () -> CALIBRATION);
    var options = new IMapLocalCampaign.Options(SETTINGS, 5, 0);
    var output = directory.resolve("cancelled");
    IMapLocalCampaign.run(output, options, capture, IMapResourceMonitor.Targets.NONE, () -> true);
    var index = readIndex(output);
    assertEquals("1", index.getProperty("pairs.retained"));
    var report = Files.readString(output.resolve("report.txt"));
    assertTrue(report.contains("INCOMPLETE_CAMPAIGN"), report);
    assertTrue(report.contains("INCONCLUSIVE"), report);
    assertFalse(Files.exists(output.resolve("pair-2.bin")));
    assertThrows(FileAlreadyExistsException.class,
        () -> IMapLocalCampaign.run(output, options, capture, IMapResourceMonitor.Targets.NONE, () -> false));
    checkHashes(output, index);
  }

  @Test void invalidInputsOrMissingBaselineFailBeforeCreatingOutput() throws Exception {
    assertThrows(IllegalArgumentException.class, () -> IMapLocalCampaign.main(new String[0]));
    assertThrows(IllegalArgumentException.class, () -> new IMapLocalCampaign.Options(SETTINGS, 5, -1));
    assertThrows(IllegalArgumentException.class, () -> new IMapLocalCampaign.Options(SETTINGS, 5, 60_001));
    var output = directory.resolve("failed");
    var input = Files.writeString(directory.resolve("fixture"), "bytes");
    var capture = source(input, () -> { throw new IOException("probe failure"); });
    assertThrows(IOException.class, () -> IMapLocalCampaign.run(output, new IMapLocalCampaign.Options(SETTINGS, 5, 0),
        capture, IMapResourceMonitor.Targets.NONE, () -> false));
    assertThrows(UncheckedIOException.class, () -> source(directory.resolve("absent"), () -> CALIBRATION)
        .observedBindings());
    assertFalse(Files.exists(output));
  }

  @Test void adapterIdentityIsBoundToTheObservedInstance() throws Exception {
    var capture = source(Files.writeString(directory.resolve("fixture"), "bytes"), () -> CALIBRATION);
    try (var one = InMemorySemanticAdapter.strict("same"); var two = InMemorySemanticAdapter.strict("same")) {
      assertThrows(NullPointerException.class, () -> capture.startupSha256(Side.LEFT, one));
      assertSame(one, capture.started(one));
      assertSame(one, capture.started(one));
      capture.started(two);
      assertEquals(capture.startupSha256(Side.LEFT, one), capture.startupSha256(Side.RIGHT, one));
      assertNotEquals(capture.startupSha256(Side.LEFT, one), capture.startupSha256(Side.RIGHT, two));
    }
  }

  private IMapLocalCapture source(Path left, IMapLocalCapture.CalibrationProbe calibration) throws IOException {
    var other = Files.writeString(directory.resolve("other"), "fixed");
    return new IMapLocalCapture(new IMapLocalCapture.Artifacts(left, other, other, other, other),
        "config", "host", calibration);
  }

  private static Properties readIndex(Path output) throws IOException {
    var properties = new Properties();
    try (var reader = Files.newBufferedReader(output.resolve("manifest.properties"))) {
      properties.load(reader);
    }
    return properties;
  }

  private static void checkHashes(Path output, Properties index) throws IOException {
    for (Map.Entry<Object, Object> field : index.entrySet()) {
      String name = (String) field.getKey();
      if (name.endsWith(".bin.sha256")) {
        assertEquals(field.getValue(), IMapCampaignEvidenceDigest.file(output.resolve(name.replace(".sha256", ""))));
      }
    }
  }
}
