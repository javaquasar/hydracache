package io.hydracache.imap.benchmark;

import static io.hydracache.imap.benchmark.IMapCampaignPublication.Refusal.*;
import static io.hydracache.imap.benchmark.IMapLoadCampaignTest.*;
import static io.hydracache.imap.benchmark.IMapPairedAnalysisTest.attempts;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Metric.CPU_NANOS;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Phase.*;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Scope.*;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Side.*;
import static io.hydracache.imap.benchmark.IMapResourceMonitor.Status.*;
import static org.junit.jupiter.api.Assertions.*;

import io.hydracache.imap.benchmark.IMapCampaignPublication.Admission;
import io.hydracache.imap.benchmark.IMapCampaignPublication.Assessment;
import io.hydracache.imap.benchmark.IMapCampaignPublication.Calibration;
import io.hydracache.imap.benchmark.IMapCampaignPublication.CalibrationPolicy;
import io.hydracache.imap.benchmark.IMapCampaignPublication.CalibrationSample;
import io.hydracache.imap.benchmark.IMapCampaignPublication.Capture;
import io.hydracache.imap.benchmark.IMapCampaignPublication.Drift;
import io.hydracache.imap.benchmark.IMapCampaignPublication.Hosts;
import io.hydracache.imap.benchmark.IMapCampaignPublication.Manifest;
import io.hydracache.imap.benchmark.IMapCampaignPublication.PairEvidence;
import io.hydracache.imap.benchmark.IMapCampaignPublication.Refusal;
import io.hydracache.imap.benchmark.IMapCampaignPublication.Registration;
import io.hydracache.imap.benchmark.IMapCampaignPublication.ResourceRequirement;
import io.hydracache.imap.benchmark.IMapCampaignPublication.TrialCapture;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Delta;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Observation;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Reading;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Report;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Side;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Snapshot;
import io.hydracache.imap.semantic.AdapterIdentity;
import io.hydracache.imap.semantic.ErrorClass;
import io.hydracache.imap.semantic.OracleReport;
import io.hydracache.imap.semantic.SemanticDifference;
import io.hydracache.imap.semantic.StateSnapshot;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.function.UnaryOperator;
import org.junit.jupiter.api.Test;

final class IMapCampaignPublicationTest {
  private static final long SECOND = 1_000_000_000L;
  private static final Hosts DEDICATED = new Hosts(List.of("a".repeat(64)), "b".repeat(64), true);
  private static final Calibration BASELINE = new Calibration(1000, 100);
  private static final CalibrationPolicy CALIBRATION_POLICY = new CalibrationPolicy(BASELINE, 500);
  private static final ResourceRequirement LEFT_CPU = new ResourceRequirement(LEFT, MEASUREMENT, "server", PROCESS, CPU_NANOS);
  private static final ResourceRequirement RIGHT_CPU = new ResourceRequirement(RIGHT, MEASUREMENT, "server", PROCESS, CPU_NANOS);

  @Test void qualifiesAllCellsAndRetainsInconclusiveResultsAndRawEvidence() {
    var campaign = IMapLoadCampaign.plan(List.of(READS), List.of(2, 4), List.of(SERIAL),
        new FixedRateIMapLoad.Settings(117, 2, 0, 100, 1, 1, 1000), 5, BINDINGS, POLICY);
    var attempts = attempts(campaign, List.of(80), List.of(100), List.of(100), List.of(100));
    var adjusted = new ArrayList<>(attempts);
    adjusted.set(5, changeRightMeasurement(adjusted.get(5), phase -> copyPhase(phase, phase.scheduled(),
        phase.completed(), phase.successful(), 0, Map.of(), Optional.of(new FixedRateIMapLoad.Latencies(1, 2, 1100, 1300)))));
    var fixture = fixture(campaign, adjusted);
    var result = fixture.assess();
    assertTrue(result.blockers().isEmpty());
    var publication = result.requirePublishable();
    assertEquals(fixture.registration(), publication.registration());
    assertEquals(fixture.seal(), publication.manifest());
    assertEquals(fixture.evidence(), publication.pairs().stream().map(IMapCampaignPublication.PairReport::evidence).toList());
    assertEquals(new Drift(.05, -.05), publication.pairs().get(0).left());
    assertEquals(new Drift(0, 0), publication.pairs().get(0).right());
    assertEquals(List.of(IMapPairedAnalysis.Verdict.RIGHT_ADVANTAGE, IMapPairedAnalysis.Verdict.INCONCLUSIVE),
        publication.cells().stream().map(IMapPairedAnalysis.CellReport::verdict).toList());
    assertTrue(publication.cells().stream().allMatch(cell -> cell.statistics().isPresent() && !cell.resources().isEmpty()));
    assertThrows(UnsupportedOperationException.class, () -> publication.cells().clear());
    assertThrows(UnsupportedOperationException.class, () -> publication.pairs().clear());
    assertThrows(UnsupportedOperationException.class, () -> publication.manifest().pairSha256().clear());

    var prefix = new Fixture(fixture.registration(), fixture.evidence().subList(0, 9));
    blocked(prefix.assess(), INCOMPLETE_CAMPAIGN);
  }

  @Test void rejectsProvisionalApprovalMissingSlotsFailedAttemptsAndSubstitutedRuns() {
    var fixture = fixture();
    var provisional = fixture.register(Admission.PROVISIONAL, DEDICATED, CALIBRATION_POLICY, List.of(LEFT_CPU, RIGHT_CPU));
    blocked(provisional.assess(), CONTRACT_NOT_ADMITTED);
    var reordered = new ArrayList<>(fixture.evidence());
    Collections.swap(reordered, 0, 1);
    blocked(new Fixture(fixture.registration(), reordered).assess(), UNPLANNED_PAIR);
    var reused = fixture.replace(1, ignored -> fixture.evidence().get(0));
    blocked(reused.assess(), UNPLANNED_PAIR);
    var failed = fixture.replace(1, pair -> new PairEvidence(new IMapLoadCampaign.Failed(pair.attempt().run(),
        IMapLoadCampaign.Failure.CANCELLED, "cancelled", pair.attempt().resources()), Optional.empty()));
    assertEquals("CANCELLED cancelled", blocked(failed.assess(), FAILED_PAIR).detail());
    assertFalse(failed.evidence().get(1).attempt().resources().isEmpty());
    blocked(fixture.replace(1, pair -> new PairEvidence(pair.attempt(), Optional.empty())).assess(), MISSING_CAPTURE);
  }

  @Test void checksRootVersionRegistrationAndNestedPairBytes() {
    var formatFixture = new Manifest(1, "a".repeat(64), List.of("b".repeat(64), "c".repeat(64)));
    assertEquals("e0a2d2b196dd8910a2d72fd2ed8402a229126e5030699820b8c5aa5aff9925b7", formatFixture.sha256());
    var fixture = fixture();
    var original = fixture.seal();
    assertEquals(original, fixture.seal());
    assertTrue(original.sha256().matches("[0-9a-f]{64}"));
    assertEquals(fixture.registration().sha256(), original.registrationSha256());
    assertEquals(5, original.pairSha256().stream().distinct().count());
    var future = new Manifest(3, original.registrationSha256(), original.pairSha256());
    blocked(fixture.assess(future.sha256(), future), MANIFEST);
    blocked(fixture.assess("0".repeat(64), original), MANIFEST);
    blocked(fixture.assess(original.sha256(), null), MANIFEST);
    var thresholdChanged = fixture.register(Admission.ADMITTED, DEDICATED,
        new CalibrationPolicy(BASELINE, 600), List.of(LEFT_CPU, RIGHT_CPU));
    blocked(thresholdChanged.assess(original.sha256(), original), MANIFEST);

    var changedLatency = fixture.replace(1, pair -> new PairEvidence(changeRightMeasurement(pair.attempt(),
        phase -> copyPhase(phase, phase.scheduled(), phase.completed(), phase.successful(), phase.lateSlots(),
            phase.errors(), Optional.of(new FixedRateIMapLoad.Latencies(1, 2, 101, 111)))), pair.capture()));
    blocked(changedLatency.assess(original.sha256(), original), MANIFEST);
    blocked(changedLatency.assess(original.sha256(), changedLatency.seal()), MANIFEST);
    var truncatedNested = new Manifest(1, original.registrationSha256(), original.pairSha256().subList(0, 4));
    blocked(fixture.assess(truncatedNested.sha256(), truncatedNested), MANIFEST);
    var changedResource = fixture.replace(1, pair -> withReports(pair, pair.attempt().resources().subList(1, 3)));
    blocked(changedResource.assess(original.sha256(), original), MANIFEST);
    var changedCapture = fixture.replace(1, pair -> withLeft(pair, trial -> calibration(trial, 999, 100, 1000, 100)));
    blocked(changedCapture.assess(original.sha256(), original), MANIFEST);

    var tooMany = new ArrayList<>(fixture.evidence());
    tooMany.add(tooMany.get(0));
    var overflow = new Fixture(fixture.registration(), tooMany);
    assertThrows(IllegalArgumentException.class, overflow::seal);
    blocked(overflow.assess(original.sha256(), original), INCOMPLETE_CAMPAIGN);

    var huge = new AdapterIdentity("z".repeat(70_000), "test", "fixture");
    var unencodable = fixture.replace(0, pair -> withResult(pair, result -> new FixedRateIMapLoad.Pair(result.settings(),
        result.workloadFingerprint(), result.rightFirst(), result.admission(), result.preloaded(),
        new FixedRateIMapLoad.Trial(huge, result.left().warmup(), result.left().measured()), result.right())));
    assertThrows(IllegalArgumentException.class, unencodable::seal);
    blocked(unencodable.assess(original.sha256(), original), MANIFEST);
  }

  @Test void verifiesPlacementAdapterIdentityAndIndependentOrderedStarts() {
    var fixture = fixture();
    blocked(fixture.replace(0, pair -> withCapture(pair, capture ->
        new Capture(bindings("c"), capture.hosts(), capture.left(), capture.right()))).assess(), ENVIRONMENT);
    blocked(fixture.replace(0, pair -> withCapture(pair, capture ->
        new Capture(capture.observedBindings(), new Hosts(List.of("c".repeat(64)), DEDICATED.generator(), true),
            capture.left(), capture.right()))).assess(), ENVIRONMENT);
    for (var hosts : List.of(new Hosts(DEDICATED.servers(), DEDICATED.generator(), false),
        new Hosts(List.of(DEDICATED.servers().get(0), DEDICATED.generator()), DEDICATED.generator(), true))) {
      var changed = fixture.register(Admission.ADMITTED, hosts, CALIBRATION_POLICY, List.of(LEFT_CPU, RIGHT_CPU));
      for (int i = 0; i < changed.evidence().size(); i++) {
        changed = changed.replace(i, pair -> withCapture(pair, capture ->
            new Capture(capture.observedBindings(), hosts, capture.left(), capture.right())));
      }
      blocked(changed.assess(), ENVIRONMENT);
    }
    var other = new AdapterIdentity("other", "test", "fixture");
    blocked(fixture.replace(0, pair -> withResult(pair, result -> new FixedRateIMapLoad.Pair(result.settings(),
        result.workloadFingerprint(), result.rightFirst(), result.admission(), result.preloaded(),
        new FixedRateIMapLoad.Trial(other, result.left().warmup(), result.left().measured()), result.right())))
        .assess(), ADAPTER);
    blocked(fixture.replace(0, pair -> withResult(pair, result -> new FixedRateIMapLoad.Pair(result.settings(),
        result.workloadFingerprint(), result.rightFirst(), result.admission(), result.preloaded(), result.left(),
        new FixedRateIMapLoad.Trial(other, result.right().warmup(), result.right().measured())))).assess(), ADAPTER);
    blocked(fixture.replace(0, pair -> withAdmission(pair, report -> new OracleReport(report.seed(),
        report.leftIdentity(), report.rightIdentity(), List.of(), report.leftSnapshot(),
        new StateSnapshot("f".repeat(64), 1, 0)))).assess(), ADAPTER);

    var first = fixture.evidence().get(0).capture().orElseThrow();
    blocked(fixture.replace(1, pair -> withLeft(pair, left -> new TrialCapture(first.right().startupSha256(),
        left.measurementStartNanos(), left.measurementEndNanos(), left.before(), left.after()))).assess(), STARTUP_REUSED);
    blocked(fixture.replace(0, pair -> withCapture(pair, capture ->
        new Capture(capture.observedBindings(), capture.hosts(), capture.right(), capture.left())))
        .assess(), TRIAL_ORDER);
    blocked(fixture.replace(1, pair -> withCapture(pair, capture ->
        new Capture(capture.observedBindings(), capture.hosts(), first.left(), first.right())))
        .assess(), TRIAL_ORDER);
  }

  @Test void rejectsMissingInvalidAndDriftingCalibrationOnEitherSide() {
    var fixture = fixture();
    List<UnaryOperator<TrialCapture>> invalid = List.of(
        trial -> new TrialCapture(trial.startupSha256(), trial.measurementStartNanos(), trial.measurementEndNanos(),
            Optional.empty(), trial.after()),
        trial -> new TrialCapture(trial.startupSha256(), trial.measurementStartNanos(), trial.measurementEndNanos(),
            trial.before(), Optional.empty()),
        trial -> samples(trial, trial.measurementStartNanos() + 1, trial.measurementEndNanos() + 1),
        trial -> new TrialCapture(trial.startupSha256(), trial.measurementStartNanos(), trial.measurementStartNanos(),
            trial.before(), trial.after()),
        trial -> new TrialCapture(trial.startupSha256(), trial.measurementStartNanos(), trial.measurementStartNanos() + 1,
            trial.before(), trial.after()),
        trial -> samples(trial, trial.measurementStartNanos() - 1, trial.measurementEndNanos() - 1),
        trial -> calibration(trial, 0, 100, 1000, 100),
        trial -> calibration(trial, 1000, 100, 1000, 0),
        trial -> calibration(trial, 1051, 100, 1000, 100),
        trial -> calibration(trial, 1000, 100, 1000, 94),
        trial -> calibration(trial, 950, 100, 1050, 100));
    for (var bad : invalid) blocked(fixture.replace(0, pair -> withLeft(pair, bad)).assess(), CALIBRATION);
    var rightMissing = fixture.replace(0, pair -> withCapture(pair, capture -> new Capture(capture.observedBindings(),
        capture.hosts(), capture.left(), invalid.get(0).apply(capture.right()))));
    assertTrue(blocked(rightMissing.assess(), CALIBRATION).subject().endsWith("/right"));
  }

  @Test void requiresEachPreregisteredResourceAndVerifiesItAgainstBothSnapshots() {
    var fixture = fixture();
    var evidence = fixture.evidence().get(0);
    var measurement = evidence.attempt().resources().get(1);
    var resource = measurement.observations().get(0);
    var good = resource.metrics().get(CPU_NANOS);
    var badValue = new Delta(good.before(), good.after(), OptionalLong.of(500), AVAILABLE);
    var reset = new Delta(good.before(), good.after(), OptionalLong.empty(), COUNTER_RESET);
    var missing = new Reading(resource.after().sampledAtNanos(), Optional.empty(), "unavailable");
    var early = new Reading(resource.before().sampledAtNanos(), resource.after().snapshot(), "");
    var corrupted = List.of(new Observation("unregistered", PROCESS, resource.before(), resource.after(), resource.metrics()),
        new Observation("server", JVM, resource.before(), resource.after(), resource.metrics()),
        new Observation("server", PROCESS, resource.before(), resource.after(), Map.of()),
        new Observation("server", PROCESS, resource.before(), resource.after(), Map.of(CPU_NANOS, reset)),
        new Observation("server", PROCESS, resource.before(), resource.after(), Map.of(CPU_NANOS, badValue)),
        new Observation("server", PROCESS, resource.before(), missing, resource.metrics()),
        new Observation("server", PROCESS, resource.before(), early, resource.metrics()));
    for (var invalid : corrupted) {
      var phase = new Report(LEFT, MEASUREMENT, measurement.elapsedNanos(), true, List.of(invalid));
      blocked(fixture.replace(0, pair -> replaceLeftReport(pair, phase)).assess(), RESOURCE);
    }
    for (var phase : List.of(new Report(LEFT, MEASUREMENT, -1, true, List.of(resource)),
        new Report(LEFT, MEASUREMENT, measurement.elapsedNanos(), false, List.of(resource)),
        new Report(PAIR, MEASUREMENT, measurement.elapsedNanos(), true, List.of(resource)),
        new Report(LEFT, MEASUREMENT, measurement.elapsedNanos(), true, List.of(resource, resource)))) {
      blocked(fixture.replace(0, pair -> replaceLeftReport(pair, phase)).assess(), RESOURCE);
    }
    blocked(fixture.replace(0, pair -> withReports(pair, List.of(measurement, measurement))).assess(), RESOURCE);
    blocked(fixture.replace(0, pair -> withReports(pair, List.of())).assess(), RESOURCE);
    var withoutRequiredResources = fixture.register(Admission.ADMITTED, DEDICATED, CALIBRATION_POLICY, List.of());
    assertNotNull(withoutRequiredResources.replace(0, pair -> withReports(pair, List.of())).assess().requirePublishable());
    var unavailable = new Observation("server", PROCESS, resource.before(), missing,
        Map.of(CPU_NANOS, new Delta(good.before(), OptionalLong.empty(), OptionalLong.empty(), UNAVAILABLE)));
    var unavailablePhase = new Report(LEFT, MEASUREMENT, measurement.elapsedNanos(), true, List.of(unavailable));
    blocked(fixture.replace(0, pair -> replaceLeftReport(pair, unavailablePhase)).assess(), RESOURCE);
    var optional = withoutRequiredResources.replace(0, pair -> replaceLeftReport(pair, unavailablePhase));
    assertNotNull(optional.assess().requirePublishable());
    var wrongDelta = new Report(LEFT, MEASUREMENT, measurement.elapsedNanos(), true,
        List.of(new Observation("server", PROCESS, resource.before(), resource.after(), Map.of(CPU_NANOS, badValue))));
    blocked(withoutRequiredResources.replace(0, pair -> replaceLeftReport(pair, wrongDelta)).assess(), RESOURCE);
  }

  @Test void recomputesAnalysisAndRejectsMalformedOrIneligibleStatistics() {
    var fixture = fixture();
    var malformed = fixture.replace(0, pair -> new PairEvidence(changeRightMeasurement(pair.attempt(),
        phase -> copyPhase(phase, phase.scheduled() + 1, phase.completed(), phase.successful(), 0,
            phase.errors(), phase.endToEnd())), pair.capture()));
    assertTrue(blocked(malformed.assess(), ANALYSIS).detail().contains("phase accounting"));
    var errors = fixture.replace(1, pair -> new PairEvidence(changeRightMeasurement(pair.attempt(),
        phase -> copyPhase(phase, phase.scheduled(), phase.completed(), phase.successful() - 1, 0,
            Map.of(ErrorClass.INTERNAL, 1), phase.endToEnd())), pair.capture()));
    assertEquals("operation errors in the run", blocked(errors.assess(), ANALYSIS).detail());
    var divergence = fixture.replace(0, pair -> withAdmission(pair, report -> new OracleReport(report.seed(),
        report.leftIdentity(), report.rightIdentity(), List.of(new SemanticDifference("get", "present", "absent")),
        report.leftSnapshot(), report.rightSnapshot())));
    assertEquals("semantic preflight failed", blocked(divergence.assess(), ANALYSIS).detail());
  }

  @Test void refusesMalformedPoliciesAndDigests() {
    assertThrows(IllegalArgumentException.class, () -> new Hosts(List.of("not-sha256"), DEDICATED.generator(), true));
    assertThrows(IllegalArgumentException.class, () -> new Hosts(List.of(), DEDICATED.generator(), true));
    assertThrows(IllegalArgumentException.class, () -> new Hosts(
        Collections.nCopies(2, DEDICATED.servers().get(0)), DEDICATED.generator(), true));
    assertThrows(IllegalArgumentException.class, () -> new CalibrationPolicy(new Calibration(0, 100), 500));
    assertThrows(IllegalArgumentException.class, () -> new CalibrationPolicy(BASELINE, -1));
    assertThrows(IllegalArgumentException.class, () -> new CalibrationPolicy(BASELINE, 10_001));
    assertThrows(IllegalArgumentException.class, () -> new ResourceRequirement(LEFT, MEASUREMENT, "server", JVM, CPU_NANOS));
    assertThrows(IllegalArgumentException.class, () -> new ResourceRequirement(PAIR, MEASUREMENT, "server", PROCESS, CPU_NANOS));
    assertThrows(IllegalArgumentException.class, () -> new ResourceRequirement(LEFT, PREFLIGHT, "server", PROCESS, CPU_NANOS));
    assertThrows(IllegalArgumentException.class, () -> new Manifest(1, "0".repeat(64), List.of("not-sha256")));
    var fixture = fixture();
    assertThrows(IllegalArgumentException.class,
        () -> fixture.register(Admission.ADMITTED, DEDICATED, CALIBRATION_POLICY, List.of(LEFT_CPU, LEFT_CPU)));
    assertNotNull(new ResourceRequirement(PAIR, PREFLIGHT, "driver", PROCESS, CPU_NANOS));
  }

  private static IMapCampaignPublication.Blocker blocked(Assessment assessment, Refusal reason) {
    assertTrue(assessment.publication().isEmpty());
    assertThrows(IllegalStateException.class, assessment::requirePublishable);
    return assessment.blockers().stream().filter(blocker -> blocker.reason() == reason).findFirst()
        .orElseThrow(() -> new AssertionError("expected " + reason + ": " + assessment.blockers()));
  }

  private record Fixture(Registration registration, List<PairEvidence> evidence) {
    Manifest seal() { return IMapCampaignPublication.seal(registration, evidence); }

    Assessment assess() {
      var manifest = seal();
      return assess(manifest.sha256(), manifest);
    }

    Assessment assess(String sha256, Manifest manifest) {
      return IMapCampaignPublication.assess(registration, sha256, manifest, evidence);
    }

    Fixture register(Admission admission, Hosts hosts, CalibrationPolicy calibration, List<ResourceRequirement> required) {
      return new Fixture(new Registration(registration.contractSha256(), admission, registration.campaign(), hosts,
          registration.leftAdapter(), registration.rightAdapter(), calibration, required), evidence);
    }

    Fixture replace(int index, UnaryOperator<PairEvidence> replacement) {
      var changed = new ArrayList<>(evidence);
      changed.set(index, replacement.apply(changed.get(index)));
      return new Fixture(registration, changed);
    }
  }

  private static Fixture fixture() {
    var campaign = campaign(0, 100, 5, POLICY);
    return fixture(campaign, attempts(campaign, List.of(80), List.of(100), List.of(100), List.of(100)));
  }

  @Test void rawHistogramsAreRequiredForPublication() {
    var fixture = fixture();
    var missing = fixture.replace(0, evidence -> new PairEvidence(changeRightMeasurement(evidence.attempt(),
        phase -> new FixedRateIMapLoad.Phase(phase.scheduled(), phase.completed(), phase.successful(),
            phase.lateSlots(), phase.capacityRejected(), phase.offeredWindowNanos(), phase.elapsedNanos(),
            phase.errors(), phase.endToEnd(), phase.queue(), phase.service())), evidence.capture()));
    blocked(missing.assess(), HISTOGRAM);
  }

  private static Fixture fixture(IMapLoadCampaign campaign, List<IMapLoadCampaign.Attempt> attempts) {
    var pairs = new ArrayList<PairEvidence>();
    var first = (IMapLoadCampaign.Measured) attempts.get(0);
    var registration = new Registration("c".repeat(64), Admission.ADMITTED, campaign, DEDICATED,
        first.result().left().adapter(), first.result().right().adapter(), CALIBRATION_POLICY, List.of(LEFT_CPU, RIGHT_CPU));
    for (int i = 0; i < attempts.size(); i++) {
      var attempt = (IMapLoadCampaign.Measured) attempts.get(i);
      long start = 200 * SECOND * (i + 1);
      boolean rightFirst = attempt.run().rightFirst();
      var left = trial(2 * i + 1, start + (rightFirst ? 70 * SECOND : 0), new Calibration(1050, 95));
      var right = trial(2 * i + 2, start + (rightFirst ? 0 : 70 * SECOND), BASELINE);
      var capture = new Capture(attempt.run().bindings(), DEDICATED, left, right);
      var reports = List.of(new Report(LEFT, PRELOAD, 1, true, List.of()),
          measurement(LEFT, left, attempt.result().left().measured().elapsedNanos()),
          measurement(RIGHT, right, attempt.result().right().measured().elapsedNanos()),
          new Report(PAIR, PREFLIGHT, 1, true, List.of()));
      var result = attempt.result();
      var raw = new FixedRateIMapLoad.Pair(result.settings(), result.workloadFingerprint(), result.rightFirst(),
          result.admission(), result.preloaded(), withHistograms(result.left()), withHistograms(result.right()));
      pairs.add(new PairEvidence(new IMapLoadCampaign.Measured(attempt.run(), raw, reports), Optional.of(capture)));
    }
    return new Fixture(registration, List.copyOf(pairs));
  }

  private static FixedRateIMapLoad.Trial withHistograms(FixedRateIMapLoad.Trial trial) {
    return new FixedRateIMapLoad.Trial(trial.adapter(), withHistograms(trial.warmup()), withHistograms(trial.measured()));
  }

  private static FixedRateIMapLoad.Phase withHistograms(FixedRateIMapLoad.Phase phase) {
    var raw = new FixedRateIMapLoad.Histograms(histogram(phase.endToEnd(), phase.completed()),
        histogram(phase.queue(), phase.completed()), histogram(phase.service(), phase.completed()));
    return new FixedRateIMapLoad.Phase(phase.scheduled(), phase.completed(), phase.successful(), phase.lateSlots(),
        phase.capacityRejected(), phase.offeredWindowNanos(), phase.elapsedNanos(), phase.errors(),
        raw.endToEnd().quantiles(), raw.queue().quantiles(), raw.service().quantiles(), Optional.of(raw));
  }

  private static FixedRateIMapLoad.Histogram histogram(Optional<FixedRateIMapLoad.Latencies> latency, int count) {
    if (latency.isEmpty()) return new FixedRateIMapLoad.Histogram(0, List.of());
    var value = latency.orElseThrow();
    var histogram = new LoadLatencyHistogram();
    for (int i = 0; i < count; i++) histogram.record(i + 1 == count ? value.p999() : value.p99());
    return histogram.snapshot();
  }

  private static TrialCapture trial(int id, long start, Calibration after) {
    long end = start + 60 * SECOND;
    return new TrialCapture(String.format("%064x", id), start, end,
        Optional.of(new CalibrationSample(start - 1, BASELINE)), Optional.of(new CalibrationSample(end + 1, after)));
  }

  private static TrialCapture samples(TrialCapture trial, long before, long after) {
    return new TrialCapture(trial.startupSha256(), trial.measurementStartNanos(), trial.measurementEndNanos(),
        Optional.of(new CalibrationSample(before, BASELINE)), Optional.of(new CalibrationSample(after, BASELINE)));
  }

  private static TrialCapture calibration(TrialCapture trial, long beforeBits, long beforePackets, long afterBits, long afterPackets) {
    return new TrialCapture(trial.startupSha256(), trial.measurementStartNanos(), trial.measurementEndNanos(),
        Optional.of(new CalibrationSample(trial.measurementStartNanos() - 1, new Calibration(beforeBits, beforePackets))),
        Optional.of(new CalibrationSample(trial.measurementEndNanos() + 1, new Calibration(afterBits, afterPackets))));
  }

  private static Report measurement(Side side, TrialCapture trial, long elapsed) {
    var before = new Reading(trial.measurementStartNanos(), Optional.of(new Snapshot("owner", Map.of(CPU_NANOS, 10L))), "");
    var after = new Reading(trial.measurementEndNanos(), Optional.of(new Snapshot("owner", Map.of(CPU_NANOS, 110L))), "");
    var delta = new Delta(OptionalLong.of(10), OptionalLong.of(110), OptionalLong.of(100), AVAILABLE);
    var observation = new Observation("server", PROCESS, before, after, Map.of(CPU_NANOS, delta));
    return new Report(side, MEASUREMENT, elapsed, true, List.of(observation));
  }

  private static PairEvidence withCapture(PairEvidence pair, UnaryOperator<Capture> change) {
    return new PairEvidence(pair.attempt(), pair.capture().map(change));
  }

  private static PairEvidence withLeft(PairEvidence pair, UnaryOperator<TrialCapture> change) {
    return withCapture(pair, capture -> new Capture(capture.observedBindings(), capture.hosts(), change.apply(capture.left()), capture.right()));
  }

  private static PairEvidence replaceLeftReport(PairEvidence pair, Report replacement) {
    var reports = new ArrayList<>(pair.attempt().resources());
    reports.set(1, replacement);
    return withReports(pair, reports);
  }

  private static PairEvidence withReports(PairEvidence pair, List<Report> reports) {
    var measured = (IMapLoadCampaign.Measured) pair.attempt();
    return new PairEvidence(new IMapLoadCampaign.Measured(measured.run(), measured.result(), reports), pair.capture());
  }

  private static PairEvidence withResult(PairEvidence pair, UnaryOperator<FixedRateIMapLoad.Pair> change) {
    var measured = (IMapLoadCampaign.Measured) pair.attempt();
    return new PairEvidence(new IMapLoadCampaign.Measured(measured.run(), change.apply(measured.result()), measured.resources()), pair.capture());
  }

  private static PairEvidence withAdmission(PairEvidence pair, UnaryOperator<OracleReport> change) {
    return withResult(pair, result -> new FixedRateIMapLoad.Pair(result.settings(), result.workloadFingerprint(),
        result.rightFirst(), change.apply(result.admission()), result.preloaded(), result.left(), result.right()));
  }

  private static IMapLoadCampaign.Attempt changeRightMeasurement(IMapLoadCampaign.Attempt attempt,
      UnaryOperator<FixedRateIMapLoad.Phase> change) {
    return withResult(new PairEvidence(attempt, Optional.empty()), result -> new FixedRateIMapLoad.Pair(result.settings(),
        result.workloadFingerprint(), result.rightFirst(), result.admission(), result.preloaded(), result.left(),
        new FixedRateIMapLoad.Trial(result.right().adapter(), result.right().warmup(), change.apply(result.right().measured())))).attempt();
  }

  private static FixedRateIMapLoad.Phase copyPhase(FixedRateIMapLoad.Phase phase, int scheduled, int completed,
      int successful, int late, Map<ErrorClass, Integer> errors, Optional<FixedRateIMapLoad.Latencies> endToEnd) {
    return withHistograms(new FixedRateIMapLoad.Phase(scheduled, completed, successful, late, phase.capacityRejected(),
        phase.offeredWindowNanos(), phase.elapsedNanos(), errors, endToEnd, phase.queue(), phase.service()));
  }
}
