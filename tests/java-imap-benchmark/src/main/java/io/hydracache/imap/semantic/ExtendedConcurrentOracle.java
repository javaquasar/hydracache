package io.hydracache.imap.semantic;

import static io.hydracache.imap.semantic.ExtendedConcurrentScenario.*;
import java.util.ArrayList;
import java.util.List;
import java.util.TreeMap;

/** Independent bounded legal-history search. It never calls either implementation owner. */
public final class ExtendedConcurrentOracle {
  public enum Verdict { VALID, INVALID, SEARCH_EXHAUSTED }
  public record Report(Verdict verdict, int explored) {}
  public Report validate(History history, int maximumStates) {
    if (maximumStates <= 0 || maximumStates > 100000) throw new IllegalArgumentException("history search bound");
    var search = new Search(history, maximumStates); boolean valid = search.visit(new State(history));
    return new Report(valid ? Verdict.VALID : search.exhausted ? Verdict.SEARCH_EXHAUSTED : Verdict.INVALID, search.explored);
  }
  private static final class Search {
    final History history; final int maximum; int explored; boolean exhausted;
    Search(History history, int maximum) { this.history = history; this.maximum = maximum; }
    boolean visit(State state) {
      if (explored == maximum) { exhausted = true; return false; } explored++;
      boolean finished = true; for (boolean done : state.done) finished &= done;
      if (finished) return state.signature().equals(history.finalState());
      for (int index = 0; index < history.calls().size(); index++) {
        if (state.done[index] || !eligible(state, index)) continue;
        var call = history.calls().get(index); String operation = history.round().program(call.worker()).get(call.ordinal());
        var stages = operation.equals("bulk") ? !state.bulkStarted ? List.of(-1) : state.bulkError || state.items[0] != null && state.items[1] != null ? List.of(2) : state.items[0] == null && state.items[1] == null ? List.of(0, 1) : List.of(state.items[0] == null ? 0 : 1) : List.of(-2);
        for (int stage : stages) {
          var next = new State(state); String result = next.apply(index, operation, stage);
          if (result != null) { if (!result.equals(call.result())) continue; next.done[index] = true; }
          if (visit(next)) return true;
        }
      }
      return false;
    }
    boolean eligible(State state, int index) {
      var call = history.calls().get(index);
      for (int other = 0; other < state.done.length; other++) if (history.calls().get(other).end() < call.start() && !state.done[other]) return false;
      return true;
    }
  }
  private record Value(String bytes, long version, Long expiry) {}
  private static final class State {
    final String family; final boolean[] done; final TreeMap<String, Value> data = new TreeMap<>();
    final List<String> events = new ArrayList<>(), flow = new ArrayList<>(); final String[] items = new String[2];
    long revision, watermark, clock; int outcomes, demand;
    boolean pending, completed, retired, bulkStarted, bulkError, held = true, bufferB, bufferC;
    State(History history) {
      family = history.round().family(); done = new boolean[history.calls().size()];
      if (!family.equals("retire_pending")) {
        data.put("61", new Value("00", ++revision, family.equals("bulk_expiry") ? 1L : null)); outcomes++; watermark++;
        if (!family.equals("checkpoint_lease")) { data.put("62", new Value("00", ++revision, null)); outcomes++; watermark++; }
      }
      bufferB = family.equals("listener_cut");
    }
    State(State source) {
      family = source.family; done = source.done.clone(); data.putAll(source.data); events.addAll(source.events); flow.addAll(source.flow); System.arraycopy(source.items, 0, items, 0, 2);
      revision = source.revision; watermark = source.watermark; clock = source.clock; outcomes = source.outcomes; demand = source.demand;
      pending = source.pending; completed = source.completed; retired = source.retired; bulkStarted = source.bulkStarted; bulkError = source.bulkError; held = source.held; bufferB = source.bufferB; bufferC = source.bufferC;
    }
    String apply(int index, String operation, int stage) {
      if (stage == -1) { bulkStarted = true; bulkError = clock != 0; return null; }
      if (stage == 0 || stage == 1) {
        String key = stage == 0 ? "61" : "62"; var old = data.get(key);
        String kind = old == null ? "absent" : old.version() != stage + 1 ? "mismatch" : "replaced";
        if (kind.equals("replaced")) put(key, stage == 0 ? "01" : "02", old.expiry());
        items[stage] = kind; return null;
      }
      if (stage == 2) { if (bulkError) return "error:clock_regressed"; outcomes++; return "bulk:" + items[0] + "," + items[1]; }
      return switch (operation) {
        case "remove" -> { var old = data.remove("61"); revision++; emit("removed", "61", old.bytes(), "~"); outcomes++; yield "removed"; }
        case "put" -> { String key = family.equals("listener_cut") ? "63" : "61"; String value = family.equals("listener_cut") ? "03" : "00"; String kind = data.containsKey(key) ? "replaced" : "inserted"; put(key, value, null); outcomes++; yield kind; }
        case "expire" -> { clock = 1; var old = data.remove("61"); revision++; emit("expired", "61", old.bytes(), "~"); yield "expiry:1:1:0"; }
        case "reserve" -> { if (retired) yield "error:retired_identity"; pending = true; yield "pending"; }
        case "query" -> pending ? "pending" : completed ? "completed:inserted" : "unknown";
        case "finish" -> { if (!pending) yield "skipped"; put("61", "01", null); pending = false; completed = true; outcomes++; yield "inserted"; }
        case "retire" -> { if (pending) yield "error:pending_retirement"; retired = true; int removed = completed ? 1 : 0; completed = false; outcomes -= removed; yield "retired:" + removed; }
        case "request" -> { demand++; pump(); yield "requested"; }
        case "snapshot" -> { bufferB = false; bufferC = false; yield "snapshot:2:61,62"; }
        case "close" -> { held = false; yield "closed"; }
        default -> held ? "error:active_owners" : "checkpoint";
      };
    }
    void put(String key, String value, Long expiry) { var old = data.put(key, new Value(value, ++revision, expiry)); emit(old == null ? "added" : "updated", key, old == null ? "~" : old.bytes(), value); }
    void emit(String kind, String key, String old, String value) {
      watermark++; String event = kind + "/" + key + "/" + old + "/" + value + "/" + watermark;
      if (family.equals("listener_cut")) { bufferC = true; pump(); } else events.add(event);
    }
    void pump() {
      if (demand > 0 && bufferB) { flow.add("added/62/~/00/2"); bufferB = false; demand--; }
      if (demand > 0 && bufferC) { flow.add("added/63/~/03/3"); bufferC = false; demand--; }
    }
    String signature() {
      var rows = new ArrayList<String>();
      data.forEach((key, value) -> { long captured = key.equals("61") ? 1 : key.equals("62") ? 2 : -1; boolean fresh = !family.equals("retire_pending") && value.version() == captured; rows.add(key + "/" + value.bytes() + "/" + (value.expiry() == null ? "e" : "x" + (value.expiry() - clock)) + "/" + (fresh ? "1" : "0")); });
      return String.join(",", rows) + "|" + String.join(",", events) + "|" + String.join(",", flow) + "|" + outcomes + "/" + (pending ? 1 : 0) + "/0/0/0";
    }
  }
}
