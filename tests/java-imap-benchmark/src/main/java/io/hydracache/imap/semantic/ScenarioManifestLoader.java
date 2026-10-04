package io.hydracache.imap.semantic;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.Reader;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.Set;

/** Strict, dependency-free loader for the provisional {@code imap-semantic-v1} trace format. */
public final class ScenarioManifestLoader {
  private static final String FORMAT = "imap-semantic-v1";

  private ScenarioManifestLoader() {}

  public static ScenarioManifest load(Reader source, ManifestLimits limits)
      throws IOException, ManifestException {
    if (source == null) throw new NullPointerException("source");
    if (limits == null) throw new NullPointerException("limits");

    var steps = new ArrayList<ScenarioStep>();
    long seed = 0;
    boolean sawFormat = false;
    boolean sawSeed = false;
    int characters = 0;
    int payloadBytes = 0;
    int lineNumber = 0;
    var stepIds = new HashSet<String>();

    try (var reader = new BufferedReader(source)) {
      String line;
      while ((line = reader.readLine()) != null) {
        lineNumber++;
        characters = checkedAdd(characters, line.length() + 1, "manifest character count", lineNumber);
        if (characters > limits.maxCharacters()) {
          throw error(lineNumber, "manifest exceeds maxCharacters=" + limits.maxCharacters());
        }
        if (line.length() > limits.maxLineCharacters()) {
          throw error(lineNumber, "line exceeds maxLineCharacters=" + limits.maxLineCharacters());
        }
        line = line.trim();
        if (line.isEmpty() || line.startsWith("#")) continue;

        if (line.startsWith("format=")) {
          if (sawFormat || !line.equals("format=" + FORMAT)) {
            throw error(lineNumber, "expected exactly one format=" + FORMAT);
          }
          sawFormat = true;
          continue;
        }
        if (line.startsWith("seed=")) {
          if (sawSeed) throw error(lineNumber, "duplicate seed");
          seed = parseLong(line.substring("seed=".length()), "seed", lineNumber);
          sawSeed = true;
          continue;
        }
        if (!line.startsWith("step=")) throw error(lineNumber, "unknown top-level field");
        if (!sawFormat || !sawSeed) {
          throw error(lineNumber, "format and seed must precede steps");
        }
        if (steps.size() >= limits.maxSteps()) {
          throw error(lineNumber, "manifest exceeds maxSteps=" + limits.maxSteps());
        }
        ParsedStep parsed = parseStep(line, limits, lineNumber);
        if (!stepIds.add(parsed.step().id())) {
          throw error(lineNumber, "duplicate step id: " + parsed.step().id());
        }
        payloadBytes = checkedAdd(payloadBytes, parsed.payloadBytes(), "payload byte count", lineNumber);
        if (payloadBytes > limits.maxTotalPayloadBytes()) {
          throw error(lineNumber,
              "manifest exceeds maxTotalPayloadBytes=" + limits.maxTotalPayloadBytes());
        }
        steps.add(parsed.step());
      }
    }

    if (!sawFormat) throw new ManifestException("missing format=" + FORMAT);
    if (!sawSeed) throw new ManifestException("missing seed");
    if (steps.isEmpty()) throw new ManifestException("manifest must contain at least one step");
    return new ScenarioManifest(seed, steps);
  }

  private static ParsedStep parseStep(String line, ManifestLimits limits, int lineNumber)
      throws ManifestException {
    String[] tokens = line.split(" +");
    if (tokens.length < 2) throw error(lineNumber, "step requires id and operation");
    String id = tokens[0].substring("step=".length());
    if (!id.matches("[A-Za-z0-9._-]{1,64}")) throw error(lineNumber, "invalid step id");

    Operation operation;
    try {
      operation = Operation.valueOf(tokens[1].toUpperCase(Locale.ROOT));
    } catch (IllegalArgumentException error) {
      throw error(lineNumber, "unsupported operation: " + tokens[1], error);
    }

    Map<String, String> fields = new HashMap<>();
    for (int index = 2; index < tokens.length; index++) {
      int equals = tokens[index].indexOf('=');
      if (equals <= 0 || equals == tokens[index].length() - 1) {
        throw error(lineNumber, "invalid step field: " + tokens[index]);
      }
      String name = tokens[index].substring(0, equals);
      String value = tokens[index].substring(equals + 1);
      if (fields.put(name, value) != null) throw error(lineNumber, "duplicate field: " + name);
    }

    try {
      return switch (operation) {
        case GET, GET_AND_REMOVE -> pointRead(id, operation, fields, limits, lineNumber);
        case PUT, PUT_IF_ABSENT, GET_AND_PUT ->
            pointWrite(id, operation, fields, false, limits, lineNumber);
        case REPLACE -> pointWrite(id, operation, fields, true, limits, lineNumber);
        case GET_ALL, REMOVE_ALL -> bulkKeys(id, operation, fields, limits, lineNumber);
        case PUT_ALL -> bulkEntries(id, fields, limits, lineNumber);
        case ADVANCE -> advance(id, fields, lineNumber);
      };
    } catch (IllegalArgumentException error) {
      throw error(lineNumber, error.getMessage(), error);
    }
  }

  private static ParsedStep pointRead(
      String id, Operation operation, Map<String, String> fields, ManifestLimits limits, int line)
      throws ManifestException {
    requireFields(fields, Set.of("key"), line);
    BytesValue key = decode(fields.get("key"), limits.maxKeyBytes(), "key", line);
    return new ParsedStep(new ScenarioStep(id, operation, List.of(key), List.of(), Optional.empty(),
        TtlDirective.preserve(), 0), key.size());
  }

  private static ParsedStep pointWrite(
      String id,
      Operation operation,
      Map<String, String> fields,
      boolean requiresExpected,
      ManifestLimits limits,
      int line) throws ManifestException {
    Set<String> expectedFields = requiresExpected
        ? Set.of("key", "value", "expected", "ttl")
        : Set.of("key", "value", "ttl");
    requireFields(fields, expectedFields, line);
    BytesValue key = decode(fields.get("key"), limits.maxKeyBytes(), "key", line);
    BytesValue value = decode(fields.get("value"), limits.maxValueBytes(), "value", line);
    Optional<BytesValue> expected = requiresExpected
        ? Optional.of(decode(fields.get("expected"), limits.maxValueBytes(), "expected", line))
        : Optional.empty();
    int payload = checkedAdd(key.size(), value.size(), "step payload", line);
    if (expected.isPresent()) payload = checkedAdd(payload, expected.get().size(), "step payload", line);
    ScenarioStep step = new ScenarioStep(id, operation, List.of(),
        List.of(new ScenarioEntry(key, value)), expected, parseTtl(fields.get("ttl"), line), 0);
    return new ParsedStep(step, payload);
  }

  private static ParsedStep bulkKeys(
      String id, Operation operation, Map<String, String> fields, ManifestLimits limits, int line)
      throws ManifestException {
    requireFields(fields, Set.of("keys"), line);
    String[] encoded = fields.get("keys").split(",", -1);
    requireBulkCount(encoded.length, limits, line);
    var keys = new ArrayList<BytesValue>(encoded.length);
    int payload = 0;
    for (String item : encoded) {
      BytesValue key = decode(item, limits.maxKeyBytes(), "bulk key", line);
      keys.add(key);
      payload = checkedAdd(payload, key.size(), "step payload", line);
    }
    return new ParsedStep(new ScenarioStep(id, operation, keys, List.of(), Optional.empty(),
        TtlDirective.preserve(), 0), payload);
  }

  private static ParsedStep bulkEntries(
      String id, Map<String, String> fields, ManifestLimits limits, int line)
      throws ManifestException {
    requireFields(fields, Set.of("entries", "ttl"), line);
    String[] encoded = fields.get("entries").split(",", -1);
    requireBulkCount(encoded.length, limits, line);
    var entries = new ArrayList<ScenarioEntry>(encoded.length);
    int payload = 0;
    for (String item : encoded) {
      int separator = item.indexOf(':');
      if (separator < 0) throw error(line, "bulk entry must be keyHex:valueHex");
      BytesValue key = decode(item.substring(0, separator), limits.maxKeyBytes(), "bulk key", line);
      BytesValue value = decode(
          item.substring(separator + 1), limits.maxValueBytes(), "bulk value", line);
      entries.add(new ScenarioEntry(key, value));
      payload = checkedAdd(payload, key.size(), "step payload", line);
      payload = checkedAdd(payload, value.size(), "step payload", line);
    }
    return new ParsedStep(new ScenarioStep(id, Operation.PUT_ALL, List.of(), entries,
        Optional.empty(), parseTtl(fields.get("ttl"), line), 0), payload);
  }

  private static ParsedStep advance(String id, Map<String, String> fields, int line)
      throws ManifestException {
    requireFields(fields, Set.of("ticks"), line);
    long ticks = parseLong(fields.get("ticks"), "ticks", line);
    if (ticks <= 0) throw error(line, "advance ticks must be positive");
    return new ParsedStep(new ScenarioStep(id, Operation.ADVANCE, List.of(), List.of(),
        Optional.empty(), TtlDirective.preserve(), ticks), 0);
  }

  private static TtlDirective parseTtl(String encoded, int line) throws ManifestException {
    if (encoded.equals("preserve")) return TtlDirective.preserve();
    if (encoded.equals("eternal")) return TtlDirective.eternal();
    if (encoded.startsWith("expire_after:")) {
      long ticks = parseLong(encoded.substring("expire_after:".length()), "ttl ticks", line);
      if (ticks <= 0) throw error(line, "expire-after ticks must be positive");
      return TtlDirective.expireAfter(ticks);
    }
    throw error(line, "unknown ttl directive: " + encoded);
  }

  private static BytesValue decode(String encoded, int maxBytes, String field, int line)
      throws ManifestException {
    if ((encoded.length() & 1) != 0) throw error(line, field + " hex length must be even");
    int bytes = encoded.length() / 2;
    if (bytes > maxBytes) throw error(line, field + " exceeds byte limit=" + maxBytes);
    try {
      return BytesValue.fromHex(encoded);
    } catch (IllegalArgumentException error) {
      throw error(line, "invalid " + field + " hex", error);
    }
  }

  private static void requireFields(Map<String, String> fields, Set<String> expected, int line)
      throws ManifestException {
    if (!fields.keySet().equals(expected)) {
      throw error(line, "fields must be exactly " + expected + ", got " + fields.keySet());
    }
  }

  private static void requireBulkCount(int count, ManifestLimits limits, int line)
      throws ManifestException {
    if (count <= 0 || count > limits.maxBulkItems()) {
      throw error(line, "bulk item count exceeds limit=" + limits.maxBulkItems());
    }
  }

  private static long parseLong(String value, String field, int line) throws ManifestException {
    try {
      return Long.parseLong(value);
    } catch (NumberFormatException error) {
      throw error(line, "invalid " + field, error);
    }
  }

  private static int checkedAdd(int left, int right, String field, int line)
      throws ManifestException {
    try {
      return Math.addExact(left, right);
    } catch (ArithmeticException error) {
      throw error(line, field + " overflow", error);
    }
  }

  private static ManifestException error(int line, String message) {
    return new ManifestException("line " + line + ": " + message);
  }

  private static ManifestException error(int line, String message, Throwable cause) {
    return new ManifestException("line " + line + ": " + message, cause);
  }

  private record ParsedStep(ScenarioStep step, int payloadBytes) {}
}
