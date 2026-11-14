package io.hydracache.imap.benchmark;

import io.hydracache.imap.semantic.BytesValue;
import io.hydracache.imap.semantic.ManifestLimits;
import io.hydracache.imap.semantic.Operation;
import io.hydracache.imap.semantic.ScenarioManifest;
import io.hydracache.imap.semantic.ScenarioStep;
import io.hydracache.imap.semantic.SemanticScenarioFingerprint;
import java.nio.charset.StandardCharsets;
import java.util.HashSet;
import java.util.List;
import java.util.Objects;

/** Bounded immutable request pool and its required semantic preflight. */
public record IMapLoadWorkload(
    ScenarioManifest preflight, List<ScenarioStep> preload, List<ScenarioStep> requests) {
  private static final ManifestLimits LIMITS = ManifestLimits.defaults();

  public IMapLoadWorkload {
    Objects.requireNonNull(preflight, "preflight");
    if (preflight.steps().size() > LIMITS.maxSteps()
        || requests.isEmpty() || requests.size() > LIMITS.maxSteps()
        || preload.size() > preflight.steps().size()) {
      throw new IllegalArgumentException("IMap load workload step bound");
    }
    preload = List.copyOf(preload);
    requests = List.copyOf(requests);
    if (!preflight.steps().subList(0, preload.size()).equals(preload)
        || !new HashSet<>(preflight.steps()).containsAll(requests)) {
      throw new IllegalArgumentException("preflight must cover preload and every timed request");
    }
    long payload = 0;
    for (ScenarioStep step : preflight.steps()) {
      payload += payloadBytes(step);
      if (payload > LIMITS.maxTotalPayloadBytes()) {
        throw new IllegalArgumentException("IMap load workload payload bound");
      }
    }
    preload.forEach(IMapLoadWorkload::requireMapRequest);
    requests.forEach(IMapLoadWorkload::requireMapRequest);
  }

  /** Binds the trace, its preload prefix and the ordered, weighted request pool. */
  public String fingerprint() {
    var identity = new StringBuilder(SemanticScenarioFingerprint.fingerprint(preflight))
        .append(':').append(preload.size());
    // The trace fingerprint binds each ordinal to its complete request, including payload and TTL.
    for (var request : requests) {
      identity.append(':').append(preflight.steps().indexOf(request));
    }
    var bytes = BytesValue.copyOf(identity.toString().getBytes(StandardCharsets.US_ASCII));
    var projection = ScenarioStep.get("imap-load-workload-v1", bytes);
    return SemanticScenarioFingerprint.fingerprint(new ScenarioManifest(0, List.of(projection)));
  }

  private static long payloadBytes(ScenarioStep step) {
    if (step.id().length() > 128
        || step.keys().size() > LIMITS.maxBulkItems()
        || step.entries().size() > LIMITS.maxBulkItems()) {
      throw new IllegalArgumentException("IMap load step shape bound");
    }
    long bytes = 0;
    for (var key : step.keys()) {
      bytes += bounded(key.size(), LIMITS.maxKeyBytes());
    }
    for (var entry : step.entries()) {
      bytes += bounded(entry.key().size(), LIMITS.maxKeyBytes());
      bytes += bounded(entry.value().size(), LIMITS.maxValueBytes());
    }
    if (step.expected().isPresent()) {
      bytes += bounded(step.expected().orElseThrow().size(), LIMITS.maxValueBytes());
    }
    return bytes;
  }

  private static int bounded(int size, int limit) {
    if (size > limit) throw new IllegalArgumentException("IMap load key or value bound");
    return size;
  }

  private static void requireMapRequest(ScenarioStep step) {
    if (step.operation() == Operation.ADVANCE || step.operation() == Operation.LISTENER_GAP) {
      throw new IllegalArgumentException("semantic controls cannot execute in the load workload");
    }
    var keys = new HashSet<>(step.keys());
    for (var entry : step.entries()) keys.add(entry.key());
    if (keys.size() != step.keys().size() + step.entries().size()) {
      throw new IllegalArgumentException("load request contains duplicate keys");
    }
  }
}
