package io.hydracache.imap.semantic;

import static io.hydracache.imap.semantic.ExtendedSemanticScenario.*;
import java.nio.ByteBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Objects;
import java.util.Optional;
import java.util.function.Predicate;

/** Closed, bounded development corpus codec. This format assigns no SDK or protocol IDs. */
public final class ExtendedSemanticCorpus {
  public static final String FORMAT = "imap-extended-semantic-v2";
  public static final int MAX_BYTES = 2 * 1024 * 1024;
  public static final int MAX_LINE_BYTES = 1024 * 1024;
  private ExtendedSemanticCorpus() {}

  public static ExtendedSemanticScenario parse(String input) {
    Objects.requireNonNull(input);
    if (input.length() > MAX_BYTES || input.chars().anyMatch(c -> c > 127 || c == '\r')) throw new IllegalArgumentException("corpus byte/ASCII bound");
    var lines = new ArrayList<>(List.of(input.split("\n", -1)));
    if (lines.size() > 1 && lines.get(lines.size() - 1).isEmpty()) lines.remove(lines.size() - 1);
    if (lines.size() > MAX_STEPS + 1) throw new IllegalArgumentException("corpus step bound");
    for (var line : lines) if (line.length() > MAX_LINE_BYTES) throw new IllegalArgumentException("corpus line bound");
    var header = fields(lines.get(0), "\t", 3);
    if (!header[0].equals(FORMAT)) throw new IllegalArgumentException("unsupported corpus version");
    long seed = number(header[1], true); long count = number(header[2], false);
    if (count != lines.size() - 1) throw new IllegalArgumentException("corpus declared count");
    var steps = new ArrayList<Step>();
    for (int index = 1; index < lines.size(); index++) {
      var field = fields(lines.get(index), "\t", 7);
      Op operation;
      try { operation = Op.valueOf(field[1].toUpperCase(Locale.ROOT)); }
      catch (IllegalArgumentException failure) { throw new IllegalArgumentException("unknown corpus operation", failure); }
      if (!operation.name().toLowerCase(Locale.ROOT).equals(field[1])) throw new IllegalArgumentException("noncanonical operation");
      var entries = new ArrayList<Entry>();
      if (!field[2].equals("~")) for (String encoded : boundedList(field[2])) {
        var entry = fields(encoded, ":", 4);
        entries.add(new Entry(hex(entry[0], 1024), entry[1].equals("~") ? Optional.empty() : Optional.of(hex(entry[1], 4096)),
            entry[2].equals("~") ? Optional.empty() : Optional.of(text(entry[2])), ttl(entry[3])));
      }
      var requests = new ArrayList<Request>();
      if (!field[3].equals("~")) for (String encoded : boundedList(field[3])) {
        var request = fields(encoded, ":", 2); requests.add(new Request(text(request[0]), number(request[1], false)));
      }
      Optional<Limits> limits = Optional.empty();
      if (!field[4].equals("~")) { var pair = fields(field[4], ":", 2); long maximum = number(pair[0], false); if (maximum > MAX_ITEMS) throw new IllegalArgumentException("corpus snapshot item bound"); limits = Optional.of(new Limits((int) maximum, number(pair[1], false))); }
      if (!field[6].equals("0") && !field[6].equals("1")) throw new IllegalArgumentException("corpus loss flag");
      steps.add(new Step(text(field[0]), operation, entries, requests, limits, number(field[5], false), field[6].equals("1")));
    }
    return new ExtendedSemanticScenario(VERSION, seed, steps);
  }

  public static String write(ExtendedSemanticScenario scenario) {
    Objects.requireNonNull(scenario); var output = new StringBuilder();
    output.append(FORMAT).append('\t').append(scenario.seed()).append('\t').append(scenario.steps().size()).append('\n');
    for (var step : scenario.steps()) {
      var entries = new ArrayList<String>();
      for (var entry : step.entries()) entries.add(entry.key().hex() + ":" + entry.value().map(BytesValue::hex).orElse("~") + ":"
          + entry.versionRef().map(ExtendedSemanticCorpus::textHex).orElse("~") + ":" + ttl(entry.ttl()));
      var requests = step.requests().stream().map(request -> textHex(request.client()) + ":" + request.sequence()).toList();
      String line = textHex(step.id()) + "\t" + step.operation().name().toLowerCase(Locale.ROOT) + "\t" + joined(entries) + "\t" + joined(requests)
          + "\t" + step.limits().map(limit -> limit.maximumItems() + ":" + limit.maximumBytes()).orElse("~") + "\t" + step.ticks() + "\t" + (step.loseResponse() ? "1" : "0");
      if (line.length() > MAX_LINE_BYTES || output.length() + line.length() + 1 > MAX_BYTES) throw new IllegalArgumentException("encoded corpus bound");
      output.append(line).append('\n');
    }
    return output.toString();
  }
  static String textHex(String text) {
    try { var bytes = StandardCharsets.UTF_8.newEncoder().encode(java.nio.CharBuffer.wrap(text)); var copy = new byte[bytes.remaining()]; bytes.get(copy); return BytesValue.copyOf(copy).hex(); }
    catch (CharacterCodingException failure) { throw new IllegalArgumentException("invalid UTF-16 corpus text", failure); }
  }
  private static String text(String encoded) {
    try { return StandardCharsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT).onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(hex(encoded, 96).copy())).toString(); }
    catch (CharacterCodingException failure) { throw new IllegalArgumentException("invalid UTF-8 corpus text", failure); }
  }
  private static BytesValue hex(String encoded, int maximum) {
    if (encoded.length() > maximum * 2 || (encoded.length() & 1) != 0 || !encoded.matches("[0-9a-f]*")) throw new IllegalArgumentException("corpus hex bound/canonical form");
    return BytesValue.fromHex(encoded);
  }
  private static String[] fields(String input, String delimiter, int count) { var fields = input.split(delimiter, -1); if (fields.length != count) throw new IllegalArgumentException("corpus field count"); return fields; }
  private static String[] boundedList(String input) { var parts = input.split(",", -1); if (parts.length > MAX_ITEMS) throw new IllegalArgumentException("corpus item bound"); return parts; }
  private static String joined(List<String> values) { return values.isEmpty() ? "~" : String.join(",", values); }
  private static long number(String input, boolean signed) {
    if (!input.matches(signed ? "(?:0|[1-9][0-9]*|-[1-9][0-9]*)" : "(?:0|[1-9][0-9]*)")) throw new IllegalArgumentException("corpus canonical number");
    try { return Long.parseLong(input); } catch (NumberFormatException failure) { throw new IllegalArgumentException("corpus number overflow", failure); }
  }
  private static TtlDirective ttl(String input) { return switch (input) { case "p" -> TtlDirective.preserve(); case "e" -> TtlDirective.eternal(); default -> { if (!input.startsWith("x")) throw new IllegalArgumentException("corpus TTL kind"); yield TtlDirective.expireAfter(number(input.substring(1), false)); } }; }
  private static String ttl(TtlDirective ttl) { return switch (ttl.kind()) { case PRESERVE -> "p"; case ETERNAL -> "e"; case EXPIRE_AFTER -> "x" + ttl.ticks(); }; }

  public record ShrinkResult(ExtendedSemanticScenario scenario, int replays, long work, boolean budgetExhausted) {}
  public static ShrinkResult shrink(ExtendedSemanticScenario original, SeededSemanticScenario.ShrinkBudget budget, Predicate<ExtendedSemanticScenario> stillFails) {
    Objects.requireNonNull(original); Objects.requireNonNull(budget); Objects.requireNonNull(stillFails);
    if (original.steps().size() > budget.maxWork()) throw new IllegalArgumentException("initial replay work bound");
    if (!stillFails.test(original)) throw new IllegalArgumentException("original corpus does not fail");
    var minimized = new ArrayList<>(original.steps()); int replays = 1; long work = minimized.size();
    for (int index = 0; index < minimized.size();) {
      if (replays >= budget.maxReplays() || minimized.size() - 1 > budget.maxWork() - work) return new ShrinkResult(new ExtendedSemanticScenario(VERSION, original.seed(), minimized), replays, work, true);
      var candidate = new ArrayList<>(minimized); candidate.remove(index); work += candidate.size();
      ExtendedSemanticScenario scenario;
      try { scenario = new ExtendedSemanticScenario(VERSION, original.seed(), candidate); }
      catch (IllegalArgumentException invalid) { index++; continue; }
      replays++;
      if (stillFails.test(scenario)) minimized = candidate; else index++;
    }
    return new ShrinkResult(new ExtendedSemanticScenario(VERSION, original.seed(), minimized), replays, work, false);
  }

  /** Seeded recipe covers all actions when sufficiently long; shorter prefixes remain valid. */
  public static ExtendedSemanticScenario generate(long seed, int count) {
    if (count <= 0 || count > MAX_STEPS) throw new IllegalArgumentException("generated step bound");
    var recipe = new Recipe(seed); recipe.populate();
    while (recipe.steps.size() < count) recipe.control(Op.RESOURCES);
    return new ExtendedSemanticScenario(VERSION, seed, recipe.steps.subList(0, count));
  }
  private static final class Recipe {
    final List<Step> steps = new ArrayList<>(); final long seed;
    final BytesValue a = BytesValue.fromHex(""), b = BytesValue.fromHex("00ff"), c = BytesValue.fromHex("c3a9");
    Recipe(long seed) { this.seed = seed; }
    Entry value(BytesValue key, int salt, TtlDirective ttl) { return new Entry(key, Optional.of(BytesValue.copyOf(ByteBuffer.allocate(10).putLong(seed ^ salt).put((byte) 0).put((byte) 255).array())), Optional.empty(), ttl); }
    Entry token(BytesValue key, String label, Optional<BytesValue> value, TtlDirective ttl) { return new Entry(key, value, Optional.of(label), ttl); }
    Request request(long sequence) { return new Request("seeded", sequence); }
    void add(Op op, List<Entry> entries, List<Request> requests, Optional<Limits> limits, long ticks, boolean lost) { steps.add(new Step("s" + steps.size(), op, entries, requests, limits, ticks, lost)); }
    void mutate(Op op, long identity, Entry... entries) { add(op, List.of(entries), List.of(request(identity)), Optional.empty(), 0, false); }
    void control(Op op) { add(op, List.of(), List.of(), Optional.empty(), 0, false); }
    void capture(BytesValue key, String label) { add(Op.CAPTURE_VERSION, List.of(token(key, label, Optional.empty(), TtlDirective.preserve())), List.of(), Optional.empty(), 0, false); }
    void query(long... identities) { add(Op.QUERY_OUTCOMES, List.of(), java.util.Arrays.stream(identities).mapToObj(this::request).toList(), Optional.empty(), 0, false); }
    void snapshot(boolean hold) { add(hold ? Op.HOLD_LISTENER_SNAPSHOT : Op.LISTENER_RESYNC, List.of(), List.of(), Optional.of(new Limits(MAX_ITEMS, 1024 * 1024)), 0, false); }
    void populate() {
      mutate(Op.PUT, 1, value(a, 1, TtlDirective.eternal())); capture(a, "a0");
      mutate(Op.RESERVE_PUT, 2, value(b, 2, TtlDirective.eternal())); query(2); mutate(Op.DISPATCH_PUT, 2, value(b, 2, TtlDirective.eternal())); capture(b, "b0");
      Entry lost = value(a, 3, TtlDirective.eternal()); add(Op.PUT, List.of(lost), List.of(request(3)), Optional.empty(), 0, true); query(3); mutate(Op.PUT, 3, lost); capture(a, "a1");
      mutate(Op.REPLACE_IF_VERSION, 4, token(a, "a0", value(a, 4, TtlDirective.eternal()).value(), TtlDirective.preserve()));
      mutate(Op.REPLACE_IF_VERSION, 5, token(a, "a1", value(a, 5, TtlDirective.eternal()).value(), TtlDirective.preserve())); capture(a, "a2");
      var bulk = List.of(token(a, "a1", value(a, 6, TtlDirective.eternal()).value(), TtlDirective.preserve()), token(b, "b0", value(b, 7, TtlDirective.eternal()).value(), TtlDirective.preserve()));
      if ((seed & 1) != 0) bulk = List.of(bulk.get(1), bulk.get(0));
      add(Op.BULK_REPLACE, bulk, List.of(request(6)), Optional.empty(), 0, false); capture(b, "b1");
      mutate(Op.SET_TTL_IF_VERSION, 7, token(a, "a2", Optional.empty(), TtlDirective.expireAfter(2))); capture(a, "a3");
      mutate(Op.BULK_SET_TTL, 8, token(a, "a2", Optional.empty(), TtlDirective.eternal()), token(b, "b1", Optional.empty(), TtlDirective.expireAfter(3))); capture(b, "b2");
      mutate(Op.BULK_REMOVE, 9, token(a, "a3", Optional.empty(), TtlDirective.preserve()), token(b, "b1", Optional.empty(), TtlDirective.preserve()));
      mutate(Op.REMOVE_IF_VERSION, 10, token(b, "b1", Optional.empty(), TtlDirective.preserve())); mutate(Op.REMOVE_IF_VERSION, 11, token(b, "b2", Optional.empty(), TtlDirective.preserve()));
      mutate(Op.PUT, 12, value(a, 8, TtlDirective.expireAfter(1))); capture(a, "a4"); add(Op.ADVANCE, List.of(), List.of(), Optional.empty(), 1, false); capture(a, "absent");
      mutate(Op.EVICT, 13, new Entry(a, Optional.empty(), Optional.empty(), TtlDirective.preserve()));
      mutate(Op.PUT, 14, value(a, 9, TtlDirective.eternal())); capture(a, "pre_restore"); snapshot(true); control(Op.CHECKPOINT_RESTORE); control(Op.CLOSE_LISTENER_SNAPSHOT); control(Op.CHECKPOINT_RESTORE);
      mutate(Op.REPLACE_IF_VERSION, 15, token(a, "pre_restore", value(a, 10, TtlDirective.eternal()).value(), TtlDirective.preserve())); capture(a, "fresh_instance");
      control(Op.SIGNAL_GAP); mutate(Op.PUT, 16, value(c, 11, TtlDirective.eternal())); snapshot(false); control(Op.RESOURCES);
      mutate(Op.RESERVE_PUT, 17, value(b, 12, TtlDirective.eternal())); add(Op.CANCEL_RESERVED, List.of(), List.of(request(17)), Optional.empty(), 0, false); query(17);
      add(Op.RETIRE_OUTCOMES, List.of(), List.of(request(6)), Optional.empty(), 0, false); query(1, 3, 7); mutate(Op.PUT, 1, value(a, 1, TtlDirective.eternal()));
      mutate(Op.CLEAR, 18); mutate(Op.CLEAR, 19); mutate(Op.PUT, 20, value(a, 13, TtlDirective.eternal())); mutate(Op.DESTROY, 21);
      control(Op.CLOSE_LISTENER_SNAPSHOT); add(Op.CANCEL_RESERVED, List.of(), List.of(request(17)), Optional.empty(), 0, false); query(1, 20, 21); control(Op.RESOURCES);
    }
  }
}
