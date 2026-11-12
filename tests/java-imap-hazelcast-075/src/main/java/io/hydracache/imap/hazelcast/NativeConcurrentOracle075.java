package io.hydracache.imap.hazelcast;

import io.hydracache.imap.semantic.BytesValue;
import java.util.*;
import java.util.function.Function;
import static io.hydracache.imap.hazelcast.NativeConcurrentScenario075.*;
import static io.hydracache.imap.hazelcast.NativeConcurrentHistory075.*;

/** Independent bounded sequential map specification; native callbacks do not constrain this order. */
public final class NativeConcurrentOracle075 {
  private NativeConcurrentOracle075() {}
  public enum Verdict { LINEARIZABLE, NON_LINEARIZABLE, EXHAUSTED }
  public record Report(Verdict verdict, long work, List<Integer> witness) { public Report { witness = List.copyOf(witness); } }
  public record Shrink(History history, int trials, boolean exhausted) {}
  public static Report check(History history, long maxWork) {
    Objects.requireNonNull(history); if (maxWork < 1 || maxWork > 1_000_000) throw new IllegalArgumentException("native search work bound");
    var search = new Search(history, maxWork); boolean passed = search.visit(new TreeMap<>(), 0, new ArrayList<>());
    return new Report(passed ? Verdict.LINEARIZABLE : search.exhausted ? Verdict.EXHAUSTED : Verdict.NON_LINEARIZABLE, search.work, search.witness);
  }
  private static final class Search {
    final History history; final long maxWork; long work; boolean exhausted; List<Integer> witness = List.of(); final int[] predecessors;
    Search(History history, long maxWork) {
      this.history = history; this.maxWork = maxWork; predecessors = new int[history.calls().size()];
      for (var a : history.calls()) for (var b : history.calls()) {
        if (a.returned() < b.invoked() || a.command().worker() == b.command().worker() && a.command().position() < b.command().position()) predecessors[b.command().id()] |= 1 << a.command().id();
      }
    }
    boolean visit(TreeMap<BytesValue, BytesValue> state, int done, List<Integer> order) {
      if (work == maxWork) { exhausted = true; return false; } work++;
      if (done == (1 << history.calls().size()) - 1) {
        var entries = state.entrySet().stream().map(e -> new NativeIMapObservation075.Entry(e.getKey(), e.getValue())).toList();
        if (!NativeIMapObservation075.digestEntries(entries).equals(history.finalState().digest())) return false;
        witness = List.copyOf(order); return true;
      }
      for (var call : history.calls()) {
        int bit = 1 << call.command().id();
        if ((done & bit) != 0 || (predecessors[call.command().id()] & done) != predecessors[call.command().id()]) continue;
        var next = new TreeMap<>(state); var result = apply(next, call.command()); if (!result.equals(call.result())) continue;
        order.add(call.command().id()); if (visit(next, done | bit, order)) return true; order.remove(order.size() - 1);
        if (exhausted) return false;
      }
      return false;
    }
  }
  static RawResult apply(Map<BytesValue, BytesValue> state, Command command) {
    var old = state.get(command.key()); var value = command.value().orElse(null);
    return switch (command.operation()) {
      case GET -> new RawResult(Optional.ofNullable(old), Optional.empty());
      case CONTAINS_KEY -> RawResult.bool(old != null);
      case PUT, GET_AND_PUT -> { state.put(command.key(), value); yield new RawResult(Optional.ofNullable(old), Optional.empty()); }
      case PUT_IF_ABSENT -> { if (old == null) state.put(command.key(), value); yield new RawResult(Optional.ofNullable(old), Optional.empty()); }
      case REPLACE_IF_PRESENT -> { if (old != null) state.put(command.key(), value); yield new RawResult(Optional.ofNullable(old), Optional.empty()); }
      case GET_AND_REMOVE -> { state.remove(command.key()); yield new RawResult(Optional.ofNullable(old), Optional.empty()); }
      case REPLACE_IF_VALUE, REMOVE_IF_VALUE -> {
        boolean matches = old != null && command.expected().orElseThrow().equals(old);
        if (matches) { if (command.operation() == Operation.REPLACE_IF_VALUE) state.put(command.key(), value); else state.remove(command.key()); }
        yield RawResult.bool(matches);
      }
    };
  }
  /** Candidates are freshly executed by a trusted caller; editing observations is not replay. */
  public static Shrink shrink(History original, int maxTrials, long maxSearch, Function<Program, History> replay) {
    if (maxTrials < 1 || maxTrials > 32 || check(original, maxSearch).verdict() != Verdict.NON_LINEARIZABLE) throw new IllegalArgumentException("native shrink requires a counterexample and finite bounds");
    History best = original; int trials = 0; boolean reduced;
    do {
      reduced = false;
      for (var command : best.program().commands()) {
        if (best.program().commands().stream().filter(c -> c.worker() == command.worker()).count() == 1) continue;
        if (trials == maxTrials) return new Shrink(best, trials, true);
        var candidate = best.program().without(command.id()); var history = replay.apply(candidate); trials++;
        if (!history.program().equals(candidate)) throw new IllegalArgumentException("native shrink replay changed program");
        var report = check(history, maxSearch);
        if (report.verdict() == Verdict.EXHAUSTED) return new Shrink(best, trials, true);
        if (report.verdict() == Verdict.NON_LINEARIZABLE) { best = history; reduced = true; break; }
      }
    } while (reduced);
    return new Shrink(best, trials, false);
  }
}
