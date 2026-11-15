package io.hydracache.imap.benchmark;

import io.hydracache.imap.benchmark.IMapResourceMonitor.Metric;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Phase;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Scope;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Side;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Status;
import io.hydracache.imap.semantic.AdapterIdentity;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.Set;

/** Fail-closed publication boundary for a preregistered fixed-rate IMap campaign. */
public final class IMapCampaignPublication {
  public static final int MANIFEST_VERSION = 2;

  private IMapCampaignPublication() {}

  public enum Admission { PROVISIONAL, ADMITTED }

  /** Fingerprints identify each server host and the separate load generator host. */
  public record Hosts(List<String> servers, String generator, boolean dedicated) {
    public Hosts {
      servers = List.copyOf(servers);
      if (servers.isEmpty() || new HashSet<>(servers).size() != servers.size()) {
        throw new IllegalArgumentException("IMap server hosts must be present and distinct");
      }
      servers.forEach(IMapCampaignPublication::requireDigest);
      requireDigest(generator);
    }
  }

  public record Calibration(long bitsPerSecond, long packetsPerSecond) {
    boolean valid() { return bitsPerSecond > 0 && packetsPerSecond > 0; }
  }

  public record CalibrationPolicy(Calibration baseline, int maxDriftBasisPoints) {
    public CalibrationPolicy {
      if (!baseline.valid() || maxDriftBasisPoints < 0 || maxDriftBasisPoints > 10_000) {
        throw new IllegalArgumentException("invalid IMap calibration policy");
      }
    }
  }

  public record ResourceRequirement(Side side, Phase phase, String label, Scope scope, Metric metric) {
    public ResourceRequirement {
      Objects.requireNonNull(side);
      Objects.requireNonNull(phase);
      IMapResourceMonitor.requireLabel(label);
      if (metric.scope() != scope) throw new IllegalArgumentException("IMap resource requirement scope mismatch");
      if ((side == Side.PAIR) != (phase == Phase.PREFLIGHT)) {
        throw new IllegalArgumentException("IMap resource requirement phase mismatch");
      }
    }
  }

  /** Trusted verifier input, obtained from the reviewed comparison contract before measurement. */
  public record Registration(String contractSha256, Admission admission, IMapLoadCampaign campaign,
                             Hosts hosts, AdapterIdentity leftAdapter, AdapterIdentity rightAdapter,
                             CalibrationPolicy calibration, List<ResourceRequirement> requiredResources) {
    public Registration {
      requireDigest(contractSha256);
      Objects.requireNonNull(admission);
      Objects.requireNonNull(campaign);
      Objects.requireNonNull(hosts);
      Objects.requireNonNull(leftAdapter);
      Objects.requireNonNull(rightAdapter);
      Objects.requireNonNull(calibration);
      requiredResources = List.copyOf(requiredResources);
      if (new HashSet<>(requiredResources).size() != requiredResources.size()) {
        throw new IllegalArgumentException("duplicate IMap resource requirement");
      }
    }

    public String sha256() { return IMapCampaignEvidenceDigest.registration(this); }
  }

  public record CalibrationSample(long sampledAtNanos, Calibration reading) {
    public CalibrationSample { Objects.requireNonNull(reading); }
  }

  /** All captures in the campaign use the same monotonic coordinator clock. */
  public record TrialCapture(String startupSha256, long measurementStartNanos, long measurementEndNanos,
                             Optional<CalibrationSample> before, Optional<CalibrationSample> after) {
    public TrialCapture {
      requireDigest(startupSha256);
      Objects.requireNonNull(before);
      Objects.requireNonNull(after);
    }
  }

  public record Capture(IMapLoadCampaign.Bindings observedBindings, Hosts hosts,
                        TrialCapture left, TrialCapture right) {
    public Capture {
      Objects.requireNonNull(observedBindings);
      Objects.requireNonNull(hosts);
      Objects.requireNonNull(left);
      Objects.requireNonNull(right);
    }
  }

  public record PairEvidence(IMapLoadCampaign.Attempt attempt, Optional<Capture> capture) {
    public PairEvidence {
      Objects.requireNonNull(attempt);
      Objects.requireNonNull(capture);
    }
  }

  public record Manifest(int version, String registrationSha256, List<String> pairSha256) {
    public Manifest {
      requireDigest(registrationSha256);
      pairSha256 = List.copyOf(pairSha256);
      pairSha256.forEach(IMapCampaignPublication::requireDigest);
    }

    public String sha256() { return IMapCampaignEvidenceDigest.manifest(this); }
  }

  public enum Refusal {
    CONTRACT_NOT_ADMITTED, MANIFEST, INCOMPLETE_CAMPAIGN, UNPLANNED_PAIR, FAILED_PAIR, MISSING_CAPTURE,
    ENVIRONMENT, ADAPTER, STARTUP_REUSED, TRIAL_ORDER, CALIBRATION, RESOURCE, HISTOGRAM, ANALYSIS
  }

  public record Blocker(Refusal reason, String subject, String detail) {}

  /** Fractional changes from the before to the after calibration. */
  public record Drift(double bitsPerSecond, double packetsPerSecond) {}
  public record PairReport(PairEvidence evidence, Drift left, Drift right) {}

  /** A publication payload can only be created after evidence checks and independent analysis. */
  public static final class Publication {
    private final Registration registration;
    private final Manifest manifest;
    private final List<IMapPairedAnalysis.CellReport> cells;
    private final List<PairReport> pairs;

    private Publication(Registration registration, Manifest manifest, List<IMapPairedAnalysis.CellReport> cells,
        List<PairReport> pairs) {
      this.registration = registration;
      this.manifest = manifest;
      this.cells = List.copyOf(cells);
      this.pairs = List.copyOf(pairs);
    }

    public Registration registration() { return registration; }
    public Manifest manifest() { return manifest; }
    public List<IMapPairedAnalysis.CellReport> cells() { return cells; }
    public List<PairReport> pairs() { return pairs; }
  }

  public record Assessment(List<Blocker> blockers, Optional<Publication> publication) {
    public Assessment { blockers = List.copyOf(blockers); }

    public Publication requirePublishable() {
      return publication.orElseThrow(() -> new IllegalStateException("IMap publication blocked: " + blockers));
    }
  }

  /** Seal retained evidence, including failed or partial campaigns. Sealing does not qualify it. */
  public static Manifest seal(Registration registration, List<PairEvidence> pairs) {
    if (pairs.size() > registration.campaign().runs().size()) {
      throw new IllegalArgumentException("too many IMap pair manifests");
    }
    return new Manifest(MANIFEST_VERSION, registration.sha256(),
        pairs.stream().map(IMapCampaignEvidenceDigest::pair).toList());
  }

  /** The retained root SHA-256 and registration come from the verifier's trusted inputs. */
  public static Assessment assess(Registration registration, String retainedRootSha256, Manifest manifest,
      List<PairEvidence> pairs) {
    pairs = List.copyOf(pairs);
    var blockers = new ArrayList<Blocker>();
    var runs = registration.campaign().runs();
    if (pairs.size() != runs.size()) {
      blockers.add(new Blocker(Refusal.INCOMPLETE_CAMPAIGN, "campaign",
          "expected " + runs.size() + " attempts, received " + pairs.size()));
    }
    if (!verifyManifest(registration, retainedRootSha256, manifest, pairs)) {
      blockers.add(new Blocker(Refusal.MANIFEST, "campaign", "version, retained root or nested SHA-256 mismatch"));
    }
    if (blockers.isEmpty()) {
      var startups = new HashSet<String>();
      long previousEnd = Long.MIN_VALUE;
      for (int i = 0; i < runs.size(); i++) {
        previousEnd = verifyPair(registration, runs.get(i), pairs.get(i), previousEnd, startups, blockers);
      }
    }
    if (registration.admission() != Admission.ADMITTED) {
      blockers.add(new Blocker(Refusal.CONTRACT_NOT_ADMITTED, "campaign", "comparison contract is provisional"));
    }
    if (!blockers.isEmpty()) return refused(blockers);

    List<IMapPairedAnalysis.CellReport> cells;
    try {
      cells = IMapPairedAnalysis.analyze(registration.campaign(), pairs.stream().map(PairEvidence::attempt).toList());
    } catch (IllegalArgumentException | NullPointerException invalid) {
      return refused(List.of(new Blocker(Refusal.ANALYSIS, "campaign", invalid.getMessage())));
    }
    for (var cell : cells) {
      if (cell.statistics().isEmpty()) blockers.add(new Blocker(Refusal.ANALYSIS, cell.cell(), cell.reason()));
    }
    if (!blockers.isEmpty()) return refused(blockers);
    var pairReports = pairs.stream().map(pair -> {
      var capture = pair.capture().orElseThrow();
      return new PairReport(pair, drift(capture.left()), drift(capture.right()));
    }).toList();
    return new Assessment(List.of(), Optional.of(new Publication(registration, manifest, cells, pairReports)));
  }

  private static boolean verifyManifest(Registration registration, String root, Manifest manifest,
      List<PairEvidence> pairs) {
    try {
      return manifest.version() == MANIFEST_VERSION && manifest.pairSha256().size() == pairs.size()
          && manifest.sha256().equals(root)
          && manifest.equals(seal(registration, pairs));
    } catch (IllegalArgumentException | NullPointerException malformed) {
      return false;
    }
  }

  private static Assessment refused(List<Blocker> blockers) { return new Assessment(blockers, Optional.empty()); }

  private static long verifyPair(Registration registration, IMapLoadCampaign.Run run, PairEvidence pair,
      long previousEnd, Set<String> startups, List<Blocker> blockers) {
    String subject = run.cell() + "/" + run.pairNumber();
    if (!run.equals(pair.attempt().run())) {
      blockers.add(new Blocker(Refusal.UNPLANNED_PAIR, subject, "attempt differs from the planned slot"));
      return previousEnd;
    }
    if (pair.attempt() instanceof IMapLoadCampaign.Failed failed) {
      blockers.add(new Blocker(Refusal.FAILED_PAIR, subject, failed.failure() + " " + failed.detail()));
      return previousEnd;
    }
    if (pair.capture().isEmpty()) {
      blockers.add(new Blocker(Refusal.MISSING_CAPTURE, subject, "independent start and calibration capture missing"));
      return previousEnd;
    }
    var capture = pair.capture().orElseThrow();
    if (!run.bindings().equals(capture.observedBindings()) || !registration.hosts().equals(capture.hosts())) {
      blockers.add(new Blocker(Refusal.ENVIRONMENT, subject, "observed campaign bindings differ"));
    }
    if (!capture.hosts().dedicated() || capture.hosts().servers().contains(capture.hosts().generator())) {
      blockers.add(new Blocker(Refusal.ENVIRONMENT, subject, "dedicated host placement required"));
    }
    var result = ((IMapLoadCampaign.Measured) pair.attempt()).result();
    if (java.util.stream.Stream.of(result.left(), result.right())
        .flatMap(trial -> java.util.stream.Stream.of(trial.warmup(), trial.measured()))
        .anyMatch(phase -> phase.histograms().isEmpty())) {
      blockers.add(new Blocker(Refusal.HISTOGRAM, subject, "raw warm-up or measurement histogram is missing"));
    }
    if (!registration.leftAdapter().equals(result.left().adapter())
        || !registration.rightAdapter().equals(result.right().adapter())
        || !result.admission().leftSnapshot().equals(result.admission().rightSnapshot())) {
      blockers.add(new Blocker(Refusal.ADAPTER, subject, "adapter identity or semantic endpoint differs"));
    }
    var first = run.rightFirst() ? capture.right() : capture.left();
    var second = run.rightFirst() ? capture.left() : capture.right();
    if (previousEnd > first.measurementStartNanos() || first.measurementEndNanos() > second.measurementStartNanos()) {
      blockers.add(new Blocker(Refusal.TRIAL_ORDER, subject, "measurement windows overlap or violate the planned order"));
    }
    verifyTrial(registration.calibration(), capture.left(), result.left().measured().elapsedNanos(),
        subject + "/left", startups, blockers);
    verifyTrial(registration.calibration(), capture.right(), result.right().measured().elapsedNanos(),
        subject + "/right", startups, blockers);
    verifyResources(pair.attempt().resources(), registration.requiredResources(), subject, blockers);
    return second.measurementEndNanos();
  }

  private record PhaseKey(Side side, Phase phase) {}

  private static void verifyResources(List<IMapResourceMonitor.Report> reports, List<ResourceRequirement> required,
      String subject, List<Blocker> blockers) {
    var available = new HashSet<ResourceRequirement>();
    var seenPhases = new HashSet<PhaseKey>();
    for (var report : reports) {
      var phase = new PhaseKey(report.side(), report.phase());
      if (!report.completed() || report.elapsedNanos() < 0
          || (phase.side() == Side.PAIR) != (phase.phase() == Phase.PREFLIGHT) || !seenPhases.add(phase)) {
        blockers.add(new Blocker(Refusal.RESOURCE, subject, "invalid or repeated phase " + phase));
        return;
      }
      var seenLabels = new HashSet<String>();
      for (var observation : report.observations()) {
        if (!seenLabels.add(observation.label()) || !IMapResourceMonitor.consistent(observation, report.elapsedNanos())) {
          blockers.add(new Blocker(Refusal.RESOURCE, subject, "invalid or repeated observation " + observation.label()));
          return;
        }
        observation.metrics().forEach((metric, delta) -> {
          if (delta.status() == Status.AVAILABLE) {
            available.add(new ResourceRequirement(phase.side(), phase.phase(), observation.label(), observation.scope(), metric));
          }
        });
      }
    }
    for (var requirement : required) {
      if (!available.contains(requirement)) {
        blockers.add(new Blocker(Refusal.RESOURCE, subject, "missing valid observation " + requirement));
      }
    }
  }

  private static void verifyTrial(CalibrationPolicy policy, TrialCapture trial, long elapsedNanos, String subject,
      Set<String> startups, List<Blocker> blockers) {
    if (!startups.add(trial.startupSha256())) {
      blockers.add(new Blocker(Refusal.STARTUP_REUSED, subject, "startup identity already used"));
    }
    if (trial.before().isEmpty() || trial.after().isEmpty()) {
      blockers.add(new Blocker(Refusal.CALIBRATION, subject, "before or after sample missing"));
      return;
    }
    var before = trial.before().orElseThrow();
    var after = trial.after().orElseThrow();
    if (before.sampledAtNanos() > trial.measurementStartNanos()
        || trial.measurementStartNanos() >= trial.measurementEndNanos()
        || trial.measurementEndNanos() - trial.measurementStartNanos() < elapsedNanos
        || trial.measurementEndNanos() > after.sampledAtNanos()
        || !before.reading().valid() || !after.reading().valid()) {
      blockers.add(new Blocker(Refusal.CALIBRATION, subject, "invalid samples or measurement window"));
    } else if (drifted(policy.baseline(), before.reading(), policy.maxDriftBasisPoints())
        || drifted(policy.baseline(), after.reading(), policy.maxDriftBasisPoints())
        || drifted(before.reading(), after.reading(), policy.maxDriftBasisPoints())) {
      blockers.add(new Blocker(Refusal.CALIBRATION, subject, "calibration drift exceeds preregistered limit"));
    }
  }

  private static boolean drifted(Calibration before, Calibration after, int limit) {
    return drifted(before.bitsPerSecond(), after.bitsPerSecond(), limit)
        || drifted(before.packetsPerSecond(), after.packetsPerSecond(), limit);
  }

  private static boolean drifted(long before, long after, int limit) {
    return BigInteger.valueOf(Math.abs(after - before)).multiply(BigInteger.valueOf(10_000))
        .compareTo(BigInteger.valueOf(before).multiply(BigInteger.valueOf(limit))) > 0;
  }

  private static Drift drift(TrialCapture trial) {
    var before = trial.before().orElseThrow().reading();
    var after = trial.after().orElseThrow().reading();
    return new Drift((after.bitsPerSecond() - before.bitsPerSecond()) / (double) before.bitsPerSecond(),
        (after.packetsPerSecond() - before.packetsPerSecond()) / (double) before.packetsPerSecond());
  }

  private static void requireDigest(String digest) {
    if (!digest.matches("[0-9a-f]{64}")) throw new IllegalArgumentException("IMap evidence requires SHA-256");
  }
}
