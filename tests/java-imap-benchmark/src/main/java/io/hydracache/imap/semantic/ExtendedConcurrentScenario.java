package io.hydracache.imap.semantic;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashSet;
import java.util.List;
import java.util.Objects;

/** Separate bounded development schedules; public SDK and wire operation IDs are unaffected. */
public record ExtendedConcurrentScenario(long seed, List<Round> rounds) {
  public static final String SCHEDULE = "imap-extended-schedule-v1", HISTORY = "imap-extended-history-v1";
  public static final int MAX_ROUNDS = 32, MAX_SCHEDULE_BYTES = 16 * 1024, MAX_HISTORY_BYTES = 256 * 1024;
  public static final List<String> FAMILIES = List.of("bulk_aba", "bulk_expiry", "retire_pending", "listener_cut", "checkpoint_lease");
  public static final List<String> ORDERS = List.of("overlap", "left_first", "right_first");
  public ExtendedConcurrentScenario {
    if (rounds.isEmpty() || rounds.size() > MAX_ROUNDS) throw new IllegalArgumentException("concurrent round bound"); rounds = List.copyOf(rounds);
    var ids = new HashSet<Integer>(); for (var round : rounds) if (!ids.add(round.id())) throw new IllegalArgumentException("duplicate round");
  }
  public record Round(int id, String family, String order) {
    public Round { if (id <= 0 || id > MAX_ROUNDS || !FAMILIES.contains(family) || !ORDERS.contains(order)) throw new IllegalArgumentException("concurrent round shape"); }
    public List<String> program(int worker) {
      if (worker < 0 || worker > 1) throw new IllegalArgumentException("worker");
      return switch (family) {
        case "bulk_aba" -> worker == 0 ? List.of("bulk") : List.of("remove", "put");
        case "bulk_expiry" -> worker == 0 ? List.of("bulk") : List.of("expire");
        case "retire_pending" -> worker == 0 ? List.of("reserve", "query", "finish") : List.of("retire", "query");
        case "listener_cut" -> worker == 0 ? List.of("request") : List.of("snapshot", "put");
        default -> worker == 0 ? List.of("close") : List.of("checkpoint");
      };
    }
  }
  public record Call(int worker, int ordinal, int start, int end, String result) {
    public Call { if (worker < 0 || worker > 1 || ordinal < 0 || ordinal > 2 || start <= 0 || end <= start || end > 64) throw new IllegalArgumentException("invocation interval"); ascii(result, 1024); }
  }
  public record History(Round round, List<Call> calls, String finalState) {
    public History {
      Objects.requireNonNull(round); ascii(finalState, 8192);
      if (calls.size() != round.program(0).size() + round.program(1).size()) throw new IllegalArgumentException("invocation count");
      calls = List.copyOf(calls);
      var stamps = new HashSet<Integer>(); int index = 0;
      for (int worker = 0; worker < 2; worker++) {
        int previous = 0;
        for (int ordinal = 0; ordinal < round.program(worker).size(); ordinal++) {
          Call call = calls.get(index++);
          if (call.worker() != worker || call.ordinal() != ordinal || call.start() <= previous || !stamps.add(call.start()) || !stamps.add(call.end())) throw new IllegalArgumentException("invocation ordering");
          previous = call.end();
        }
      }
      if (round.order().equals("left_first") && calls.get(round.program(0).size() - 1).end() >= calls.get(round.program(0).size()).start()
          || round.order().equals("right_first") && calls.get(calls.size() - 1).end() >= calls.get(0).start()) throw new IllegalArgumentException("explicit worker order");
      if (round.order().equals("overlap") && (calls.get(0).end() <= calls.get(round.program(0).size()).start() || calls.get(round.program(0).size()).end() <= calls.get(0).start())) throw new IllegalArgumentException("declared overlap is absent");
    }
  }
  public static ExtendedConcurrentScenario generate(long seed, int count) {
    if (count <= 0 || count > MAX_ROUNDS) throw new IllegalArgumentException("concurrent generator bound");
    var rounds = new ArrayList<Round>();
    for (int index = 0; index < count; index++) rounds.add(new Round(index + 1, FAMILIES.get((Math.floorMod(seed, 5) + index) % 5), ORDERS.get((Math.floorMod(seed, 3) + index / 5) % 3)));
    return new ExtendedConcurrentScenario(seed, rounds);
  }
  public String encode() {
    var text = new StringBuilder(SCHEDULE + "\t" + seed + "\n");
    for (var round : rounds) text.append(round.id()).append('\t').append(round.family()).append('\t').append(round.order()).append('\n');
    return text.toString();
  }
  public static ExtendedConcurrentScenario parse(String input) {
    var lines = lines(input, MAX_SCHEDULE_BYTES); var head = fields(lines.get(0), "\t", 2);
    if (!head[0].equals(SCHEDULE)) throw new IllegalArgumentException("concurrent schedule version");
    long seed = number(head[1]); var rounds = new ArrayList<Round>();
    for (int index = 1; index < lines.size(); index++) { var field = fields(lines.get(index), "\t", 3); rounds.add(new Round(small(field[0]), field[1], field[2])); }
    return new ExtendedConcurrentScenario(seed, rounds);
  }
  public static String histories(long seed, List<History> histories) {
    if (histories.isEmpty() || histories.size() > MAX_ROUNDS) throw new IllegalArgumentException("history count bound");
    var scenario = new ExtendedConcurrentScenario(seed, histories.stream().map(History::round).toList());
    var text = new StringBuilder(HISTORY + "\t" + scenario.seed() + "\t" + histories.size() + "\n");
    for (var history : histories) {
      var calls = history.calls().stream().map(call -> call.worker() + "." + call.ordinal() + "." + call.start() + "." + call.end() + "." + hex(call.result())).toList();
      text.append(history.round().id()).append('\t').append(history.round().family()).append('\t').append(history.round().order()).append('\t').append(String.join(",", calls)).append('\t').append(hex(history.finalState())).append('\n');
    }
    ascii(text.toString(), MAX_HISTORY_BYTES); return text.toString();
  }
  public record Histories(long seed, List<History> values) { public Histories { values = List.copyOf(values); } }
  public static Histories parseHistories(String input) {
    var lines = lines(input, MAX_HISTORY_BYTES); var head = fields(lines.get(0), "\t", 3);
    if (!head[0].equals(HISTORY)) throw new IllegalArgumentException("concurrent history version");
    long seed = number(head[1]); if (small(head[2]) != lines.size() - 1) throw new IllegalArgumentException("history declared count");
    var result = new ArrayList<History>();
    for (int index = 1; index < lines.size(); index++) {
      var field = fields(lines.get(index), "\t", 5); var round = new Round(small(field[0]), field[1], field[2]); var calls = new ArrayList<Call>();
      if (field[3].split(",", -1).length > 5) throw new IllegalArgumentException("call bound");
      for (var encoded : field[3].split(",", -1)) { var call = fields(encoded, "\\.", 5); calls.add(new Call(small(call[0]), small(call[1]), small(call[2]), small(call[3]), unhex(call[4], 1024))); }
      result.add(new History(round, calls, unhex(field[4], 8192)));
    }
    new ExtendedConcurrentScenario(seed, result.stream().map(History::round).toList());
    return new Histories(seed, result);
  }
  static List<Call> sorted(List<Call> calls) { return calls.stream().sorted(Comparator.comparingInt(Call::worker).thenComparingInt(Call::ordinal)).toList(); }
  private static List<String> lines(String input, int maximum) {
    ascii(input, maximum); if (input.indexOf('\r') >= 0) throw new IllegalArgumentException("noncanonical newline");
    var lines = new ArrayList<>(List.of(input.split("\n", -1))); if (lines.size() > 1 && lines.get(lines.size() - 1).isEmpty()) lines.remove(lines.size() - 1);
    if (lines.size() > MAX_ROUNDS + 1) throw new IllegalArgumentException("line bound"); return lines;
  }
  private static String[] fields(String value, String separator, int count) { var fields = value.split(separator, -1); if (fields.length != count) throw new IllegalArgumentException("field count"); return fields; }
  private static long number(String value) { if (!value.matches("0|[1-9][0-9]*|-[1-9][0-9]*")) throw new IllegalArgumentException("canonical number"); try { return Long.parseLong(value); } catch (NumberFormatException failure) { throw new IllegalArgumentException("number range", failure); } }
  private static int small(String value) { long number = number(value); if (number < 0 || number > 64) throw new IllegalArgumentException("small integer range"); return (int) number; }
  private static String hex(String value) { return BytesValue.copyOf(value.getBytes(StandardCharsets.US_ASCII)).hex(); }
  private static String unhex(String value, int maximum) { if (value.length() > maximum * 2 || !value.matches("(?:[0-9a-f]{2})*")) throw new IllegalArgumentException("canonical hex"); var bytes = BytesValue.fromHex(value).copy(); for (byte b : bytes) if (b < 0) throw new IllegalArgumentException("history ASCII"); return new String(bytes, StandardCharsets.US_ASCII); }
  private static void ascii(String value, int maximum) { if (Objects.requireNonNull(value).length() > maximum || value.chars().anyMatch(c -> c > 127)) throw new IllegalArgumentException("ASCII byte bound"); }
}
