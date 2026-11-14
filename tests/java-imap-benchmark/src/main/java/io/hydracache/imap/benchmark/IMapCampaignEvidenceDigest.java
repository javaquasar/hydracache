package io.hydracache.imap.benchmark;

import io.hydracache.imap.semantic.AdapterIdentity;
import io.hydracache.imap.semantic.OracleReport;
import io.hydracache.imap.semantic.StateSnapshot;
import java.io.DataOutputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.security.DigestOutputStream;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.HexFormat;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.TreeMap;

/** Canonical SHA-256 inputs for the versioned, nested campaign manifest. */
final class IMapCampaignEvidenceDigest {
  private IMapCampaignEvidenceDigest() {}

  static String registration(IMapCampaignPublication.Registration registration) {
    return digest("imap-campaign-registration-v1", out -> {
      out.writeUTF(registration.contractSha256());
      out.writeUTF(registration.admission().name());
      list(out, registration.campaign().runs(), IMapCampaignEvidenceDigest::run);
      hosts(out, registration.hosts());
      adapter(out, registration.leftAdapter());
      adapter(out, registration.rightAdapter());
      calibration(out, registration.calibration().baseline());
      out.writeInt(registration.calibration().maxDriftBasisPoints());
      list(out, registration.requiredResources(), (stream, requirement) -> {
        stream.writeUTF(requirement.side().name());
        stream.writeUTF(requirement.phase().name());
        stream.writeUTF(requirement.label());
        stream.writeUTF(requirement.scope().name());
        stream.writeUTF(requirement.metric().name());
      });
    });
  }

  static String manifest(IMapCampaignPublication.Manifest manifest) {
    return digest("imap-campaign-manifest", out -> {
      out.writeInt(manifest.version());
      out.writeUTF(manifest.registrationSha256());
      list(out, manifest.pairSha256(), DataOutputStream::writeUTF);
    });
  }

  static String pair(IMapCampaignPublication.PairEvidence pair) {
    return digest("imap-campaign-pair-v2", out -> {
      var attempt = pair.attempt();
      run(out, attempt.run());
      if (attempt instanceof IMapLoadCampaign.Measured measured) {
        out.writeBoolean(true);
        result(out, measured.result());
      } else {
        out.writeBoolean(false);
        var failed = (IMapLoadCampaign.Failed) attempt;
        out.writeUTF(failed.failure().name());
        out.writeUTF(failed.detail());
      }
      list(out, attempt.resources(), IMapCampaignEvidenceDigest::resourceReport);
      optional(out, pair.capture(), (stream, capture) -> {
        bindings(stream, capture.observedBindings());
        hosts(stream, capture.hosts());
        trialCapture(stream, capture.left());
        trialCapture(stream, capture.right());
      });
    });
  }

  private static void run(DataOutputStream out, IMapLoadCampaign.Run run) throws IOException {
    out.writeUTF(run.cell());
    out.writeInt(run.pairNumber());
    out.writeBoolean(run.rightFirst());
    out.writeUTF(run.workloadFingerprint());
    settings(out, run.settings());
    bindings(out, run.bindings());
    var policy = run.analysisPolicy();
    out.writeLong(policy.bootstrapSeed());
    out.writeInt(policy.bootstrapIterations());
    out.writeInt(policy.confidencePermille());
    out.writeLong(policy.p99LimitNanos());
    out.writeLong(policy.p999LimitNanos());
  }

  private static void settings(DataOutputStream out, FixedRateIMapLoad.Settings settings) throws IOException {
    out.writeLong(settings.seed());
    out.writeInt(settings.requestsPerSecond());
    out.writeInt(settings.warmupRequests());
    out.writeInt(settings.measuredRequests());
    out.writeInt(settings.workers());
    out.writeInt(settings.maxInFlight());
    out.writeInt(settings.drainTimeoutMillis());
  }

  private static void bindings(DataOutputStream out, IMapLoadCampaign.Bindings bindings) throws IOException {
    out.writeUTF(bindings.leftArtifact());
    out.writeUTF(bindings.rightArtifact());
    out.writeUTF(bindings.tooling());
    out.writeUTF(bindings.harness());
    out.writeUTF(bindings.configuration());
    out.writeUTF(bindings.host());
  }

  private static void result(DataOutputStream out, FixedRateIMapLoad.Pair result) throws IOException {
    settings(out, result.settings());
    out.writeUTF(result.workloadFingerprint());
    out.writeBoolean(result.rightFirst());
    admission(out, result.admission());
    snapshot(out, result.preloaded());
    trial(out, result.left());
    trial(out, result.right());
  }

  private static void admission(DataOutputStream out, OracleReport report) throws IOException {
    out.writeLong(report.seed());
    adapter(out, report.leftIdentity());
    adapter(out, report.rightIdentity());
    list(out, report.differences(), (stream, difference) -> {
      stream.writeUTF(difference.path());
      stream.writeUTF(difference.left());
      stream.writeUTF(difference.right());
    });
    snapshot(out, report.leftSnapshot());
    snapshot(out, report.rightSnapshot());
  }

  private static void adapter(DataOutputStream out, AdapterIdentity identity) throws IOException {
    out.writeUTF(identity.product());
    out.writeUTF(identity.implementation());
    out.writeUTF(identity.readiness());
  }

  private static void snapshot(DataOutputStream out, StateSnapshot snapshot) throws IOException {
    out.writeUTF(snapshot.digest());
    out.writeInt(snapshot.liveCardinality());
    out.writeLong(snapshot.logicalTick());
  }

  private static void trial(DataOutputStream out, FixedRateIMapLoad.Trial trial) throws IOException {
    adapter(out, trial.adapter());
    phase(out, trial.warmup());
    phase(out, trial.measured());
  }

  private static void phase(DataOutputStream out, FixedRateIMapLoad.Phase phase) throws IOException {
    out.writeInt(phase.scheduled());
    out.writeInt(phase.completed());
    out.writeInt(phase.successful());
    out.writeInt(phase.lateSlots());
    out.writeInt(phase.capacityRejected());
    out.writeLong(phase.offeredWindowNanos());
    out.writeLong(phase.elapsedNanos());
    metrics(out, phase.errors(), DataOutputStream::writeInt);
    optional(out, phase.endToEnd(), IMapCampaignEvidenceDigest::latencies);
    optional(out, phase.queue(), IMapCampaignEvidenceDigest::latencies);
    optional(out, phase.service(), IMapCampaignEvidenceDigest::latencies);
    optional(out, phase.histograms(), (stream, raw) -> {
      histogram(stream, raw.endToEnd());
      histogram(stream, raw.queue());
      histogram(stream, raw.service());
    });
  }

  private static void histogram(DataOutputStream out, FixedRateIMapLoad.Histogram histogram) throws IOException {
    out.writeInt(histogram.samples());
    list(out, histogram.buckets(), (stream, bucket) -> {
      stream.writeLong(bucket.upperNanos());
      stream.writeInt(bucket.count());
    });
  }

  private static void latencies(DataOutputStream out, FixedRateIMapLoad.Latencies latencies) throws IOException {
    out.writeLong(latencies.p50());
    out.writeLong(latencies.p95());
    out.writeLong(latencies.p99());
    out.writeLong(latencies.p999());
  }

  private static void hosts(DataOutputStream out, IMapCampaignPublication.Hosts hosts) throws IOException {
    list(out, hosts.servers(), DataOutputStream::writeUTF);
    out.writeUTF(hosts.generator());
    out.writeBoolean(hosts.dedicated());
  }

  private static void trialCapture(DataOutputStream out, IMapCampaignPublication.TrialCapture trial) throws IOException {
    out.writeUTF(trial.startupSha256());
    out.writeLong(trial.measurementStartNanos());
    out.writeLong(trial.measurementEndNanos());
    optional(out, trial.before(), IMapCampaignEvidenceDigest::sample);
    optional(out, trial.after(), IMapCampaignEvidenceDigest::sample);
  }

  private static void sample(DataOutputStream out, IMapCampaignPublication.CalibrationSample sample) throws IOException {
    out.writeLong(sample.sampledAtNanos());
    calibration(out, sample.reading());
  }

  private static void calibration(DataOutputStream out, IMapCampaignPublication.Calibration reading) throws IOException {
    out.writeLong(reading.bitsPerSecond());
    out.writeLong(reading.packetsPerSecond());
  }

  private static void resourceReport(DataOutputStream out, IMapResourceMonitor.Report report) throws IOException {
    out.writeUTF(report.side().name());
    out.writeUTF(report.phase().name());
    out.writeLong(report.elapsedNanos());
    out.writeBoolean(report.completed());
    list(out, report.observations(), (stream, observation) -> {
      stream.writeUTF(observation.label());
      stream.writeUTF(observation.scope().name());
      reading(stream, observation.before());
      reading(stream, observation.after());
      metrics(stream, observation.metrics(), IMapCampaignEvidenceDigest::delta);
    });
  }

  private static void reading(DataOutputStream out, IMapResourceMonitor.Reading reading) throws IOException {
    out.writeLong(reading.sampledAtNanos());
    out.writeUTF(reading.error());
    optional(out, reading.snapshot(), (stream, snapshot) -> {
      stream.writeUTF(snapshot.identity());
      metrics(stream, snapshot.values(), DataOutputStream::writeLong);
    });
  }

  private static void delta(DataOutputStream out, IMapResourceMonitor.Delta delta) throws IOException {
    out.writeUTF(delta.status().name());
    optionalLong(out, delta.before());
    optionalLong(out, delta.after());
    optionalLong(out, delta.change());
  }

  private static void optionalLong(DataOutputStream out, OptionalLong value) throws IOException {
    out.writeBoolean(value.isPresent());
    if (value.isPresent()) out.writeLong(value.orElseThrow());
  }

  private static <K extends Enum<K>, V> void metrics(DataOutputStream out, Map<K, V> values, Writer<V> writer)
      throws IOException {
    out.writeInt(values.size());
    for (var entry : new TreeMap<>(values).entrySet()) {
      out.writeUTF(entry.getKey().name());
      writer.write(out, entry.getValue());
    }
  }

  private static <T> void optional(DataOutputStream out, Optional<T> value, Writer<T> writer) throws IOException {
    out.writeBoolean(value.isPresent());
    if (value.isPresent()) writer.write(out, value.orElseThrow());
  }

  private static <T> void list(DataOutputStream out, List<T> values, Writer<T> writer) throws IOException {
    out.writeInt(values.size());
    for (var value : values) writer.write(out, value);
  }

  private static String digest(String domain, Encoder encoder) {
    try {
      var sha = MessageDigest.getInstance("SHA-256");
      var out = new DataOutputStream(new DigestOutputStream(OutputStream.nullOutputStream(), sha));
      out.writeUTF(domain);
      encoder.write(out);
      return HexFormat.of().formatHex(sha.digest());
    } catch (IOException | NoSuchAlgorithmException invalid) {
      throw new IllegalArgumentException("cannot encode canonical IMap evidence", invalid);
    }
  }

  @FunctionalInterface private interface Encoder { void write(DataOutputStream out) throws IOException; }
  @FunctionalInterface private interface Writer<T> { void write(DataOutputStream out, T value) throws IOException; }
}
