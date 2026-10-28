package io.hydracache.imap.semantic;

import java.io.IOException;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.TreeSet;
import java.util.stream.Collectors;

/** Bounded development-only process peer for the independent Rust/Java differential gate. */
public final class IMapTraceRunner {
  private static final int MAX_STEPS = 512;
  private static final int MAX_BYTES = 512 * 1024;
  private static final int MAX_LINE_BYTES = 8 * 1024;
  private static final int MAX_OUTPUT_BYTES = 4 * 1024 * 1024;
  private static final int MAX_OBSERVATION_BYTES = 64 * 1024;

  private IMapTraceRunner() {}

  public static void main(String[] args) throws IOException {
    boolean defect = args.length == 1 && args[0].equals("--canary-drop-set");
    if (args.length > 0 && !defect) throw new IllegalArgumentException("unknown trace-runner option");
    List<String> output = execute(new InputStreamReader(System.in, StandardCharsets.US_ASCII), defect);
    for (String line : output) System.out.println(line);
  }

  public static List<String> execute(Reader source, boolean canaryDropSet) throws IOException {
    List<Step> steps = parse(source);
    var bounds = new BoundedIMapApi.Bounds(1024 * 1024, 4 * 1024 * 1024, 4096,
        256, 4096, 8 * 1024 * 1024, 4096);
    var service = new UnifiedIMapService(bounds, 1);
    var remembered = new HashMap<BytesValue, UnifiedIMapService.VersionToken>();
    var cursors = new HashMap<String, BoundedIMapApi.Cursor>();
    var listeners = new TreeSet<Long>();
    var output = new ArrayList<String>(steps.size());
    int outputBytes = 0;
    for (Step step : steps) {
      var command = command(step, remembered, cursors);
      String status;
      try {
        UnifiedIMapService.Result result;
        if (step.operation().equals("reset_limits")) {
          var limits = UnifiedIMapService.BudgetLimits.defaults();
          service = new UnifiedIMapService(bounds, 1, new UnifiedIMapService.BudgetLimits(
              number(step.detail()), limits.outcomes(), limits.pending(), limits.listeners(), limits.streams(),
              limits.resync(), limits.maxClients(), limits.maxBatch(), Math.toIntExact(number(step.extra())), limits.queryBytes()));
          remembered.clear(); cursors.clear(); listeners.clear();
          result = developmentResult("reset", List.of(), 0, false, OptionalLong.empty());
        } else if (step.operation().equals("retire_outcomes")) {
          var receipt = service.retireOutcomes("trace", 1, step.identity());
          result = developmentResult("retired", List.of(), receipt.removed(), false, OptionalLong.of(receipt.through()));
        } else if (step.operation().equals("query_outcomes")) {
          var identities = java.util.Arrays.stream(step.extra().split(",", -1))
              .map(value -> new BoundedIMapApi.MutationIdentity("trace", number(value))).toList();
          var outcomes = service.queryOutcomes(identities, command.limit);
          var items = new ArrayList<UnifiedIMapService.Item>();
          for (var outcome : outcomes) {
            var receipt = outcome.status().result();
            items.add(new UnifiedIMapService.Item(outcome.index(), BytesValue.copyOf(Long.toString(outcome.identity().sequence()).getBytes(StandardCharsets.US_ASCII)),
                outcome.status().kind() + receipt.map(value -> ":" + value.kind()).orElse(""),
                receipt.flatMap(UnifiedIMapService.Result::value), receipt.map(UnifiedIMapService.Result::version).orElse(OptionalLong.empty()),
                receipt.map(UnifiedIMapService.Result::remainingTtl).orElse(OptionalLong.empty())));
          }
          result = developmentResult("outcomes", items, items.size(), false, OptionalLong.empty());
        } else if (step.operation().equals("resync_listener")) {
          try (var snapshot = service.snapshotListener(step.identity(), command.limit, number(step.extra()), step.now())) {
            var items = new ArrayList<UnifiedIMapService.Item>();
            for (var entry : snapshot.entries()) items.add(new UnifiedIMapService.Item(items.size(), entry.key(), "present", entry.value(), OptionalLong.of(entry.version()), entry.remainingTtl()));
            result = developmentResult("resynced", items, items.size(), service.resourceUsage().resync() > 0, OptionalLong.of(snapshot.watermark()));
          }
        } else if (step.operation().equals("expire_step")) {
          var progress = service.expireStep(step.now(), command.limit);
          result = developmentResult("expiry_progress", List.of(), progress.expired(), progress.remaining(), OptionalLong.of(progress.work()));
        } else if (step.operation().equals("checkpoint_restore")) {
          service = UnifiedIMapService.restore(service.checkpoint());
          result = developmentResult("restored", List.of(), 0, false, OptionalLong.empty());
        } else if (step.operation().equals("resource_usage")) {
          result = developmentResult("resources", List.of(), service.resourceUsage().values(), false, OptionalLong.empty());
        } else if (java.util.Set.of("version_bulk_replace", "version_bulk_remove", "version_bulk_set_ttl").contains(step.operation())) {
          var entries = versionEntries(step, remembered, command.ttl);
          var operation = switch (step.operation()) { case "version_bulk_replace" -> UnifiedIMapService.VersionAction.REPLACE; case "version_bulk_remove" -> UnifiedIMapService.VersionAction.REMOVE; default -> UnifiedIMapService.VersionAction.SET_TTL; };
          result = service.versionBulk(operation, entries, new BoundedIMapApi.MutationIdentity("trace", step.identity()), step.now(), step.deadline(), command.loseResponse);
        } else if (step.operation().equals("reserve_operation")) {
          service.beginOperation(command);
          result = new UnifiedIMapService.Result("reserved", Optional.empty(), OptionalLong.empty(),
              OptionalLong.empty(), List.of(), 0, false, Optional.empty());
        } else if (step.operation().equals("cancel_reserved")) {
          boolean cancelled = service.cancelReserved(new BoundedIMapApi.MutationIdentity("trace", step.identity()));
          result = new UnifiedIMapService.Result("cancelled", Optional.empty(), OptionalLong.empty(),
              OptionalLong.empty(), List.of(), 0, cancelled, Optional.empty());
        } else if (canaryDropSet && step.operation().equals("set")) {
          result = service.execute(new UnifiedIMapService.DomainCommand("get")
              .key(step.key()).now(step.now()));
        } else {
          result = service.execute(command);
        }
        status = canonicalResult(result);
        result.token().ifPresent(token -> remembered.put(step.key(), token));
        if (result.cursor().isPresent()) cursors.put(step.operation(), result.cursor().orElseThrow());
        else if (java.util.Set.of("key_set_page", "values_page", "entry_set_page", "contains_value_bounded")
            .contains(step.operation())) cursors.remove(step.operation());
        if (step.operation().equals("entry_listener")) listeners.add(step.identity());
        if (step.operation().equals("remove_entry_listener")) listeners.remove(step.identity());
      } catch (IMapDomainException failure) {
        status = "error:" + failure.kind() + "|~|~|~|0|0|[]";
      }
      String state = service.diagnosticSnapshot(step.now()).stream().map(entry ->
          entry.key().hex() + "/" + entry.value().hex() + "/" + entry.version() + "/"
              + number(entry.remainingTtl())).collect(Collectors.joining(","));
      var events = new ArrayList<String>();
      for (long listener : listeners) {
        for (var event : service.drainListener(listener)) {
          events.add(listener + "/" + event.kind() + "/" + event.key().hex() + "/"
              + bytes(event.oldValue()) + "/" + bytes(event.value()) + "/" + event.version()
              + "/" + event.watermark());
        }
      }
      var stats = service.diagnosticStats(step.now());
      String observation = status + "|[" + state + "]|[" + String.join(",", events) + "]|["
          + stats.liveEntries() + "," + stats.valueBytes() + "," + stats.retainedOutcomes() + ","
          + stats.listeners() + "," + stats.queuedEvents() + "," + stats.inflight() + ","
          + stats.rejections() + "," + stats.cancellations() + "," + stats.requests() + ","
          + stats.activeStreams() + "," + stats.activeCursors() + "]";
      var usage = service.resourceUsage();
      observation += "|[" + usage.values() + "," + usage.outcomes() + "," + usage.pending() + ","
          + usage.listeners() + "," + usage.streams() + "," + usage.resync() + "]";
      outputBytes = Math.addExact(outputBytes, observation.length() + 1);
      if (observation.length() > MAX_OBSERVATION_BYTES || outputBytes > MAX_OUTPUT_BYTES) {
        throw new IllegalArgumentException("trace observations exceed output bound");
      }
      output.add(observation);
    }
    return List.copyOf(output);
  }

  private static UnifiedIMapService.DomainCommand command(Step step,
      Map<BytesValue, UnifiedIMapService.VersionToken> remembered,
      Map<String, BoundedIMapApi.Cursor> cursors) {
    var command = new UnifiedIMapService.DomainCommand(step.operation().equals("reserve_operation") ? "put" : step.operation())
        .key(step.key()).now(step.now()).deadline(step.deadline())
        .listenerId(step.identity()).includeValue(step.detail().equals("values"))
        .loseResponse(step.detail().equals("lost"));
    step.value().ifPresent(command::value);
    step.expected().ifPresent(command::expected);
    if (step.identity() > 0) command.identity(new BoundedIMapApi.MutationIdentity("trace", step.identity()));
    if (step.detail().matches("[0-9]+")) command.limit(Math.toIntExact(number(step.detail())));
    if (step.operation().equals("entry_listener") && !step.extra().isEmpty()) command.limit(Math.toIntExact(number(step.extra())));
    if (step.detail().equals("resume")) {
      if (cursors.containsKey(step.operation())) command.cursor(cursors.get(step.operation()));
      command.limit(1);
    }
    if (step.detail().equals("wrong_cursor")) {
      if (cursors.containsKey("key_set_page")) command.cursor(cursors.get("key_set_page"));
      command.limit(1);
    }
    command.ttl(switch (step.ttl()) {
      case "eternal" -> TtlDirective.eternal();
      case "preserve" -> TtlDirective.preserve();
      default -> TtlDirective.expireAfter(number(step.ttl().substring("after:".length())));
    });
    var token = remembered.get(step.key());
    if (token != null) {
      switch (step.detail()) {
        case "remembered" -> command.token(token);
        case "stale" -> command.token(new UnifiedIMapService.VersionToken(
            token.namespaceGeneration(), token.mapId(), token.version() > 1
                ? token.version() - 1 : token.version() + 1));
        case "wrong_generation" -> command.token(new UnifiedIMapService.VersionToken(
            token.namespaceGeneration() + 1, token.mapId(), token.version()));
        default -> { }
      }
    }
    if (!step.extra().isEmpty()) {
      if (step.operation().equals("get_all") || step.operation().equals("remove_all_detailed")) {
        command.keys(java.util.Arrays.stream(step.extra().split(",", -1))
            .map(IMapTraceRunner::hex).toList());
      } else if (step.operation().equals("put_all_detailed")
          || step.operation().equals("replace_all_detailed")) {
        command.entries(java.util.Arrays.stream(step.extra().split(",", -1)).map(entry -> {
          String[] parts = entry.split(":", -1);
          if (parts.length != 2) throw new IllegalArgumentException("invalid bulk trace entry");
          return new BoundedIMapApi.ReplaceInput(hex(parts[0]), hex(parts[1]));
        }).toList());
      }
    }
    return command;
  }

  private static UnifiedIMapService.Result developmentResult(String kind, List<UnifiedIMapService.Item> items,
      long number, boolean flag, OptionalLong version) {
    return new UnifiedIMapService.Result(kind, Optional.empty(), version, OptionalLong.empty(), items, number, flag, Optional.empty());
  }

  private static List<UnifiedIMapService.VersionInput> versionEntries(Step step,
      Map<BytesValue, UnifiedIMapService.VersionToken> remembered, TtlDirective ttl) {
    var entries = new ArrayList<UnifiedIMapService.VersionInput>();
    for (String source : step.extra().split(",", -1)) {
      String[] parts = source.split(":", -1);
      boolean replace = step.operation().equals("version_bulk_replace");
      if (parts.length != (replace ? 2 : 1)) throw new IllegalArgumentException("invalid version bulk entry");
      BytesValue key = hex(parts[0]);
      var token = remembered.getOrDefault(key, new UnifiedIMapService.VersionToken(1, Long.MAX_VALUE, 1));
      entries.add(new UnifiedIMapService.VersionInput(key, token, replace ? Optional.of(hex(parts[1])) : Optional.empty(), ttl));
    }
    return List.copyOf(entries);
  }

  private static String canonicalResult(UnifiedIMapService.Result result) {
    String items = result.items().stream().map(item -> item.index() + "/" + item.key().hex()
        + "/" + item.kind() + "/" + bytes(item.value()) + "/" + number(item.version())
        + "/" + number(item.remainingTtl())).collect(Collectors.joining(","));
    return result.kind() + "|" + bytes(result.value()) + "|" + number(result.version()) + "|"
        + number(result.remainingTtl()) + "|" + result.number() + "|" + (result.flag() ? 1 : 0)
        + "|[" + items + "]";
  }

  private record Step(String operation, BytesValue key, Optional<BytesValue> value,
      Optional<BytesValue> expected, String ttl, long identity, long now, long deadline,
      String detail, String extra) {}

  private static List<Step> parse(Reader source) throws IOException {
    var lines = new ArrayList<String>();
    var line = new StringBuilder();
    int count = 0;
    for (int value; (value = source.read()) != -1;) {
      if (++count > MAX_BYTES) throw new IllegalArgumentException("trace exceeds total byte bound");
      if (value > 127 || value == '\r') throw new IllegalArgumentException("trace must use ASCII LF lines");
      if (value == '\n') {
        lines.add(line.toString()); line.setLength(0);
        if (lines.size() > MAX_STEPS + 1) throw new IllegalArgumentException("trace exceeds step bound");
      } else {
        if (line.length() == MAX_LINE_BYTES) throw new IllegalArgumentException("trace line exceeds bound");
        line.append((char) value);
      }
    }
    if (!line.isEmpty()) lines.add(line.toString());
    if (lines.isEmpty() || !lines.get(0).startsWith("imap-api-trace-v1\t")) {
      throw new IllegalArgumentException("unsupported trace header");
    }
    number(lines.remove(0).substring("imap-api-trace-v1\t".length()));
    if (lines.isEmpty() || lines.size() > MAX_STEPS) throw new IllegalArgumentException("invalid trace step count");
    var result = new ArrayList<Step>(lines.size());
    for (String text : lines) {
      String[] fields = text.split("\t", -1);
      if (fields.length != 10 || !fields[0].matches("[a-z_]+")) {
        throw new IllegalArgumentException("trace requires operation and exactly ten fields");
      }
      if (!fields[4].equals("eternal") && !fields[4].equals("preserve")) {
        if (!fields[4].startsWith("after:") || number(fields[4].substring(6)) == 0) {
          throw new IllegalArgumentException("invalid trace TTL");
        }
      }
      for (int index : new int[] {8, 9}) {
        if (fields[index].chars().anyMatch(value -> value < 33 || value > 126
            || value == ';' || value == '[' || value == ']')) {
          throw new IllegalArgumentException("invalid trace detail");
        }
      }
      if (fields[8].matches("[0-9]+") && number(fields[8]) > Integer.MAX_VALUE) {
        throw new IllegalArgumentException("trace page limit exceeds common integer range");
      }
      result.add(new Step(fields[0], hex(fields[1]), optionalHex(fields[2]),
          optionalHex(fields[3]), fields[4], number(fields[5]), number(fields[6]),
          number(fields[7]), fields[8], fields[9]));
    }
    return List.copyOf(result);
  }

  private static BytesValue hex(String encoded) {
    if (encoded.length() > 2048) throw new IllegalArgumentException("oversized trace hex");
    return BytesValue.fromHex(encoded);
  }
  private static Optional<BytesValue> optionalHex(String encoded) {
    return encoded.equals("~") ? Optional.empty() : Optional.of(hex(encoded));
  }
  private static String bytes(Optional<BytesValue> value) {
    return value.map(BytesValue::hex).orElse("~");
  }
  private static String number(OptionalLong value) {
    return value.isPresent() ? Long.toString(value.getAsLong()) : "~";
  }
  private static long number(String value) {
    long parsed = Long.parseLong(value);
    if (parsed < 0) throw new IllegalArgumentException("negative trace integer");
    return parsed;
  }
}
