package io.hydracache.imap.semantic;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.HexFormat;
import java.util.List;
import java.util.Locale;
import java.util.Objects;

/** Versioned development-fixture identity, separate from wire, SDK and durable identities. */
public final class SemanticScenarioFingerprint {
  public static final int VERSION = 2;
  public static final int MAX_STEPS = 10_000;
  public static final long MAX_CANONICAL_BYTES = 16L * 1024 * 1024;
  private static final int MAX_COMPONENT_BYTES = 1024 * 1024;
  private SemanticScenarioFingerprint() {}

  public static String fingerprint(ScenarioManifest scenario) { return digest(scenario, "SHA-256"); }

  static String digest(ScenarioManifest scenario, String algorithm) {
    Objects.requireNonNull(scenario, "scenario");
    validateSteps(scenario.steps());
    MessageDigest digest;
    try { digest = MessageDigest.getInstance(algorithm); }
    catch (NoSuchAlgorithmException failure) { throw new IllegalStateException("fixture digest unavailable", failure); }
    digest.update(new byte[] {'H', 'C', 'S', 'F', '0', '7', '5', 0});
    number(digest, VERSION, Integer.BYTES); number(digest, scenario.seed(), Long.BYTES);
    number(digest, scenario.steps().size(), Integer.BYTES);
    for (var step : scenario.steps()) {
      frame(digest, step.id().getBytes(StandardCharsets.UTF_8));
      frame(digest, operationName(step.operation()).getBytes(StandardCharsets.US_ASCII));
      number(digest, step.keys().size(), Integer.BYTES);
      for (var key : step.keys()) frame(digest, key.copy());
      number(digest, step.entries().size(), Integer.BYTES);
      for (var entry : step.entries()) { frame(digest, entry.key().copy()); frame(digest, entry.value().copy()); }
      digest.update((byte) (step.expected().isPresent() ? 1 : 0));
      step.expected().ifPresent(value -> frame(digest, value.copy()));
      frame(digest, step.ttl().kind().name().toLowerCase(Locale.ROOT).getBytes(StandardCharsets.US_ASCII));
      number(digest, step.ttl().ticks(), Long.BYTES); number(digest, step.advanceTicks(), Long.BYTES);
    }
    return "v2:" + HexFormat.of().formatHex(digest.digest());
  }

  public static boolean verify(ScenarioManifest scenario, String identity) {
    if (!Objects.requireNonNull(identity, "identity").matches("v2:[0-9a-f]{64}")) {
      throw new IllegalArgumentException("unsupported or malformed fixture fingerprint version");
    }
    return fingerprint(scenario).equals(identity);
  }

  /** Migrate only with the original bounded scenario; a legacy digest cannot identify its fields. */
  public static String migrateLegacy(ScenarioManifest scenario, String legacy) {
    if (!Objects.requireNonNull(legacy, "legacy").matches("[0-9a-f]{64}")) {
      throw new IllegalArgumentException("explicit unprefixed v1 fingerprint required");
    }
    validateSteps(scenario.steps());
    if (!SeededSemanticScenario.legacyFingerprintV1(scenario).equals(legacy)) {
      throw new IllegalArgumentException("legacy fixture fingerprint mismatch");
    }
    return fingerprint(scenario);
  }

  static String operationName(Operation operation) { return operation.name().toLowerCase(Locale.ROOT); }

  static void validateSteps(List<ScenarioStep> steps) {
    if (steps.size() > MAX_STEPS) throw new IllegalArgumentException("fixture step bound");
    long bytes = 24;
    for (var step : steps) {
      if (step.id().length() > 256 || !StandardCharsets.UTF_8.newEncoder().canEncode(step.id()) || step.id().getBytes(StandardCharsets.UTF_8).length > 256
          || step.keys().size() > 256 || step.entries().size() > 256) throw new IllegalArgumentException("fixture field bound");
      bytes = bounded(bytes + 4 + step.id().getBytes(StandardCharsets.UTF_8).length
          + 4 + operationName(step.operation()).length() + 4 + 4 + 1
          + 4 + step.ttl().kind().name().length() + 8 + 8);
      for (var key : step.keys()) bytes = component(bytes, key);
      for (var entry : step.entries()) { bytes = component(bytes, entry.key()); bytes = component(bytes, entry.value()); }
      if (step.expected().isPresent()) bytes = component(bytes, step.expected().orElseThrow());
    }
  }
  private static long component(long previous, BytesValue bytes) {
    if (bytes.size() > MAX_COMPONENT_BYTES) throw new IllegalArgumentException("fixture component byte bound");
    return bounded(previous + 4 + bytes.size());
  }
  private static long bounded(long bytes) {
    if (bytes > MAX_CANONICAL_BYTES) throw new IllegalArgumentException("fixture canonical byte bound");
    return bytes;
  }
  private static void frame(MessageDigest digest, byte[] bytes) {
    number(digest, bytes.length, Integer.BYTES); digest.update(bytes);
  }
  private static void number(MessageDigest digest, long value, int bytes) {
    for (int shift = (bytes - 1) * 8; shift >= 0; shift -= 8) digest.update((byte) (value >>> shift));
  }
}
