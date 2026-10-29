package io.hydracache.imap.hazelcast;

import io.hydracache.imap.semantic.BytesValue;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.*;
import static io.hydracache.imap.hazelcast.NativeConcurrentScenario075.*;

/** Raw invocation/completion stream and strictly derived normalized calls. Stamps are logical. */
public final class NativeConcurrentHistory075 {
  private NativeConcurrentHistory075() {}
  public record RawResult(Optional<BytesValue> value, Optional<Boolean> booleanValue) {
    public RawResult { Objects.requireNonNull(value); Objects.requireNonNull(booleanValue); if (value.isPresent() && booleanValue.isPresent() || value.filter(v -> v.size() > 64).isPresent()) throw new IllegalArgumentException("native return shape"); }
    public static RawResult value(byte[] bytes) { return new RawResult(Optional.ofNullable(bytes).map(BytesValue::copyOf), Optional.empty()); }
    public static RawResult bool(boolean value) { return new RawResult(Optional.empty(), Optional.of(value)); }
  }
  public sealed interface Event permits Invoke, Return { long stamp(); int callId(); int worker(); }
  public record Invoke(long stamp, int callId, int worker) implements Event {}
  public record Return(long stamp, int callId, int worker, RawResult result) implements Event { public Return { Objects.requireNonNull(result); } }
  public record Call(Command command, long invoked, long returned, RawResult result) {}
  public record History(Program program, List<Event> raw, List<Call> calls, NativeIMapObservation075.Snapshot finalState, String rawDigest) {
    public History {
      Objects.requireNonNull(program); Objects.requireNonNull(finalState);
      if (raw.size() != 2 * program.commands().size() || calls.size() != program.commands().size()) throw new IllegalArgumentException("native history completeness");
      raw = List.copyOf(raw); calls = List.copyOf(calls);
      if (!normalize(program, raw).equals(calls) || !digest(raw).equals(rawDigest)
          || finalState.entries().stream().anyMatch(e -> !program.keys().contains(e.key()))) throw new IllegalArgumentException("native history raw/normalized binding");
    }
    public static History observed(Program program, List<Event> raw, NativeIMapObservation075.Snapshot finalState) {
      var ordered = raw.stream().sorted(Comparator.comparingLong(Event::stamp)).toList();
      return new History(program, ordered, normalize(program, ordered), finalState, digest(ordered));
    }
    public boolean hasOverlap() {
      for (var a : calls) for (var b : calls) if (a.command().worker() != b.command().worker() && a.invoked() < b.returned() && b.invoked() < a.returned()) return true;
      return false;
    }
  }
  static List<Call> normalize(Program program, List<Event> events) {
    if (events.size() != program.commands().size() * 2) throw new IllegalArgumentException("native raw event count");
    var invoked = new HashMap<Integer, Invoke>(); var returned = new HashMap<Integer, Return>(); long previous = 0;
    for (var event : events) {
      if (event.stamp() != previous + 1 || event.callId() < 0 || event.callId() >= program.commands().size()
          || event.worker() != program.commands().get(event.callId()).worker()) throw new IllegalArgumentException("native raw stamp/identity");
      previous = event.stamp();
      if (event instanceof Invoke call) { if (invoked.put(call.callId(), call) != null) throw new IllegalArgumentException("duplicate native invocation"); }
      else { var call = (Return) event; if (!invoked.containsKey(call.callId()) || returned.put(call.callId(), call) != null) throw new IllegalArgumentException("native completion without invocation"); }
    }
    var calls = new ArrayList<Call>(); long[] ends = new long[2];
    for (var command : program.commands()) {
      var start = invoked.get(command.id()); var end = returned.get(command.id());
      if (start == null || end == null || start.stamp() >= end.stamp() || start.stamp() <= ends[command.worker()]) throw new IllegalArgumentException("native worker order/interval");
      ends[command.worker()] = end.stamp();
      boolean bool = Set.of(Operation.CONTAINS_KEY, Operation.REPLACE_IF_VALUE, Operation.REMOVE_IF_VALUE).contains(command.operation());
      if (end.result().booleanValue().isPresent() != bool) throw new IllegalArgumentException("native atomic result type");
      calls.add(new Call(command, start.stamp(), end.stamp(), end.result()));
    }
    return List.copyOf(calls);
  }
  public static String digest(List<Event> events) {
    if (events.size() > 32) throw new IllegalArgumentException("native history hash bound");
    var canonical = new StringBuilder();
    for (var event : events) {
      canonical.append(event instanceof Invoke ? "i" : "r").append('|').append(event.stamp()).append('|').append(event.callId()).append('|').append(event.worker());
      if (event instanceof Return end) canonical.append('|').append(end.result().value().map(BytesValue::hex).orElse("null")).append('|').append(end.result().booleanValue().map(Object::toString).orElse("null"));
      canonical.append('\n');
    }
    try { return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(canonical.toString().getBytes(StandardCharsets.UTF_8))); }
    catch (NoSuchAlgorithmException unavailable) { throw new IllegalStateException(unavailable); }
  }
}
