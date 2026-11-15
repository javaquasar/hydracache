package io.hydracache.imap.benchmark;

import static io.hydracache.imap.benchmark.IMapResourceMonitor.Metric.*;

import io.hydracache.imap.benchmark.IMapCampaignPublication.CalibrationPolicy;
import io.hydracache.imap.benchmark.IMapCampaignPublication.Registration;
import io.hydracache.imap.benchmark.IMapCampaignPublication.ResourceRequirement;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Metric;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Phase;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Side;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Targets;
import io.hydracache.imap.semantic.BytesValue;
import io.hydracache.imap.semantic.InMemorySemanticAdapter;
import io.hydracache.imap.semantic.MapSemanticAdapter;
import io.hydracache.imap.semantic.Operation;
import io.hydracache.imap.semantic.ScenarioEntry;
import io.hydracache.imap.semantic.ScenarioManifest;
import io.hydracache.imap.semantic.ScenarioStep;
import io.hydracache.imap.semantic.TtlDirective;
import io.hydracache.imap.semantic.UnifiedIMapServiceSemanticAdapter;
import java.io.IOException;
import java.net.URISyntaxException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.function.BiFunction;
import java.util.function.BooleanSupplier;
import java.util.function.Supplier;

/** Single-host diagnostic using the local reference owner and the in-memory oracle adapter. */
public final class IMapLocalCampaign {
  private IMapLocalCampaign() {}

  record Options(FixedRateIMapLoad.Settings settings, int pairs, long idleMillis) {
    Options {
      if (idleMillis < 0 || idleMillis > 60_000) throw new IllegalArgumentException("IMap post-idle interval out of bounds");
    }
  }

  public static void main(String[] args) throws IOException, URISyntaxException {
    launch(args, IMapLocalCampaign::targets);
  }

  static void launch(String[] args, BiFunction<Boolean, ProcessHandle, Targets> probesFor)
      throws IOException, URISyntaxException {
    boolean controls = args.length == 12;
    if (!controls && args.length != 10) {
      throw new IllegalArgumentException(
          "usage: IMapLocalCampaign CONTRACT OUTPUT SEED RATE WARMUP MEASURED PAIRS WORKERS MAX_IN_FLIGHT IDLE_MS"
              + " [LADDER_RATES DELAY_MS]");
    }
    var options = new Options(new FixedRateIMapLoad.Settings(Long.parseLong(args[2]), Integer.parseInt(args[3]),
        Integer.parseInt(args[4]), Integer.parseInt(args[5]), Integer.parseInt(args[7]),
        Integer.parseInt(args[8]), 5000), Integer.parseInt(args[6]), Long.parseLong(args[9]));
    var process = ProcessHandle.current();
    String os = System.getProperty("os.name");
    var environment = System.getenv();
    String host = environment.getOrDefault("COMPUTERNAME", environment.getOrDefault("HOSTNAME", ""));
    var reference = location(UnifiedIMapServiceSemanticAdapter.class);
    var artifacts = new IMapLocalCapture.Artifacts(reference,
        controls ? reference : location(InMemorySemanticAdapter.class), location(IMapLocalCampaign.class),
        Path.of(process.info().command().orElseThrow()), Path.of(args[0]));
    String configuration = options + (controls ? " ladder=" + args[10] + " delayMillis=" + args[11] : "");
    var source = new IMapLocalCapture(artifacts, configuration,
        String.join("\n", host, os, System.getProperty("os.arch"), System.getProperty("os.version")),
        IMapLocalCapture::loopback);
    var output = Path.of(args[1]);
    var probes = probesFor.apply(os.startsWith("Linux"), process);
    BooleanSupplier cancelled = () -> false;
    if (controls) {
      var settings = IMapLocalDiagnostics.Controls.parse(args[10], args[11]);
      IMapLocalDiagnostics.run(output, options, settings, source, probes, cancelled);
    } else {
      run(output, options, source, probes, cancelled);
    }
  }

  static Targets targets(boolean linux, ProcessHandle process) {
    var probes = new ArrayList<IMapResourceMonitor.Probe>();
    probes.add(IMapResourceProbes.currentJvm("load-jvm"));
    if (linux) {
      probes.add(IMapResourceProbes.linuxProcess("load-process", process));
      probes.add(IMapResourceProbes.linuxPss("load-pss", process));
      probes.add(IMapResourceProbes.linuxInterface("loopback", "lo"));
    } else {
      probes.add(IMapResourceProbes.windowsProcess("load-process", process.pid()));
    }
    return new Targets(probes, List.of(), List.of());
  }

  private static Path location(Class<?> type) throws URISyntaxException {
    return Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI());
  }

  static void run(Path output, Options options, IMapLocalCapture capture, Targets targets,
      BooleanSupplier cancelled) throws IOException {
    var campaign = plan(options, List.of(options.settings().requestsPerSecond()), capture);
    execute(output, options, campaign, capture, targets, cancelled, IMapLocalCampaign::oracle);
  }

  static IMapLoadCampaign plan(Options options, List<Integer> rates, IMapLocalCapture capture) {
    var settings = options.settings();
    return IMapLoadCampaign.plan(List.of(new IMapLoadCampaign.Variant("reads", reads(settings.seed()))),
        rates, List.of(new IMapLoadCampaign.Parallelism(settings.workers(),
            settings.maxInFlight())), settings, options.pairs(), capture.observedBindings(),
        new IMapPairedAnalysis.Policy(settings.seed(), 1000, 950, 5_000_000, 20_000_000));
  }

  static List<IMapPairedAnalysis.CellReport> execute(Path output, Options options, IMapLoadCampaign campaign,
      IMapLocalCapture capture, Targets targets, BooleanSupplier cancelled,
      Supplier<? extends MapSemanticAdapter> rightAdapter) throws IOException {
    var calibration = capture.calibrate(Side.PAIR)
        .orElseThrow(() -> new IOException("local IMap loopback calibration failed before measurement"));
    Registration registration;
    try (var left = reference(); var right = rightAdapter.get()) {
      registration = new Registration(capture.contractSha256(), IMapCampaignPublication.Admission.PROVISIONAL,
          campaign, capture.hosts(), left.identity(), right.identity(), new CalibrationPolicy(calibration, 10_000),
          requiredResources(options.idleMillis()));
    }
    Files.createDirectory(output);
    var pairs = campaign.executeCaptured(run -> capture.started(reference()), run -> capture.started(rightAdapter.get()),
        cancelled, run -> targets, run -> capture, options.idleMillis());
    return IMapLocalCampaignOutput.write(output, options, registration, pairs);
  }

  static UnifiedIMapServiceSemanticAdapter reference() {
    return UnifiedIMapServiceSemanticAdapter.fresh("local-campaign");
  }

  private static InMemorySemanticAdapter oracle() { return InMemorySemanticAdapter.strict("local-campaign"); }

  private static IMapLoadWorkload reads(long seed) {
    var key = BytesValue.copyOf(new byte[] {1});
    var value = BytesValue.copyOf(new byte[] {2});
    var put = new ScenarioStep("preload", Operation.PUT, List.of(), List.of(new ScenarioEntry(key, value)),
        Optional.empty(), TtlDirective.eternal(), 0);
    var hit = ScenarioStep.get("hit", key);
    var miss = ScenarioStep.get("miss", BytesValue.copyOf(new byte[] {3}));
    return new IMapLoadWorkload(new ScenarioManifest(seed, List.of(put, hit, miss)), List.of(put), List.of(hit, miss));
  }

  private static List<ResourceRequirement> requiredResources(long idleMillis) {
    var requirements = new ArrayList<ResourceRequirement>();
    require(requirements, Side.PAIR, Phase.PREFLIGHT, "load-jvm", JVM_HEAP_USED_BYTES);
    for (Side side : List.of(Side.LEFT, Side.RIGHT)) {
      for (Phase phase : List.of(Phase.PRELOAD, Phase.WARMUP, Phase.MEASUREMENT, Phase.CLEANUP)) {
        require(requirements, side, phase, "load-jvm", JVM_HEAP_USED_BYTES);
      }
      if (idleMillis > 0) require(requirements, side, Phase.POST_IDLE, "load-jvm", JVM_HEAP_USED_BYTES);
      require(requirements, side, Phase.MEASUREMENT, "load-process", CPU_NANOS, RSS_BYTES, PROCESS_THREADS);
      require(requirements, side, Phase.MEASUREMENT, "load-jvm", JVM_GC_COLLECTIONS, JVM_GC_MILLIS, JVM_THREADS);
      require(requirements, side, Phase.MEASUREMENT, "loopback", RX_BYTES, TX_BYTES, RX_PACKETS, TX_PACKETS);
    }
    return requirements;
  }

  private static void require(List<ResourceRequirement> requirements, Side side, Phase phase, String probe,
      Metric... metrics) {
    for (Metric metric : metrics) requirements.add(new ResourceRequirement(side, phase, probe, metric.scope(), metric));
  }
}
