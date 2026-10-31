package io.hydracache.imap.semantic;

import static io.hydracache.imap.semantic.ExtendedSemanticScenario.*;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.TreeMap;

/** Independent sequential domain model. It never calls a map service to compute expectations. */
public final class ExtendedSemanticOracle {
  public record Report(long seed, AdapterIdentity actual, List<SemanticDifference> differences, int checkedSteps) {
    public Report { Objects.requireNonNull(actual); differences = List.copyOf(differences); }
    public boolean equivalent() { return differences.isEmpty(); }
    public void requireEquivalent() { if (!equivalent()) throw new IllegalStateException("extended semantic mismatch seed=" + seed + ": " + differences); }
  }
  public Report compare(ExtendedSemanticScenario scenario, ExtendedMapSemanticAdapter actual) {
    Objects.requireNonNull(scenario); Objects.requireNonNull(actual);
    var differences = new ArrayList<SemanticDifference>();
    for (Op required : scenario.steps().stream().map(Step::operation).distinct().toList()) {
      if (!actual.extendedCapabilities().contains(required)) differences.add(new SemanticDifference("capability." + required, "supported", "absent"));
    }
    if (!differences.isEmpty()) return new Report(scenario.seed(), actual.identity(), differences, 0);
    var model = new Model();
    int checked = 0;
    for (Step step : scenario.steps()) {
      Observation expected = model.execute(step), observed = actual.executeExtended(step);
      if (!expected.result().equals(observed.result())) difference(differences, step, "result", expected.result(), observed.result());
      if (!expected.snapshot().equals(observed.snapshot())) difference(differences, step, "snapshot", expected.snapshot(), observed.snapshot());
      if (!expected.events().equals(observed.events())) difference(differences, step, "events", expected.events(), observed.events());
      if (!expected.tokenRelations().equals(observed.tokenRelations())) difference(differences, step, "token_relations", expected.tokenRelations(), observed.tokenRelations());
      if (!expected.owners().equals(observed.owners())) difference(differences, step, "owners", expected.owners(), observed.owners());
      if (expected.tick() != observed.tick()) difference(differences, step, "time", expected.tick(), observed.tick());
      checked++;
    }
    return new Report(scenario.seed(), actual.identity(), differences, checked);
  }
  /** Streams expectations from the independent model without retaining a second observation history. */
  public void forEachExpected(ExtendedSemanticScenario scenario, java.util.function.BiConsumer<Step, Observation> sink) {
    Objects.requireNonNull(scenario); Objects.requireNonNull(sink); var model = new Model();
    for (var step : scenario.steps()) sink.accept(step, model.execute(step));
  }
  private static void difference(List<SemanticDifference> differences, Step step, String field, Object expected, Object observed) {
    if (differences.size() < 32) differences.add(new SemanticDifference("step." + step.id() + "." + field, brief(expected), brief(observed)));
  }
  static String brief(Object value) {
    var text = new StringBuilder(256); appendBrief(text, value); return text.toString();
  }
  private static void appendBrief(StringBuilder text, Object value) {
    if (text.length() >= 256) return;
    if (value instanceof Result result) value = List.of(result.kind(), result.value(), result.ttl(), result.items(), result.statuses(), result.number());
    else if (value instanceof Item item) value = List.of(item.index(), item.key(), item.kind(), item.value(), item.ttl());
    else if (value instanceof Status status) value = List.of(status.identity().client(), status.identity().sequence(), status.kind(), status.result());
    else if (value instanceof Row row) value = List.of(row.key(), row.value(), row.ttl());
    else if (value instanceof Event event) value = List.of(event.kind(), event.key(), event.oldValue(), event.value(), event.watermark());
    else if (value instanceof Owners owners) value = List.of(owners.outcomes(), owners.pending(), owners.listeners(), owners.snapshotPayload());
    if (value instanceof Optional<?> optional) appendBrief(text, optional.orElse(null));
    else if (value instanceof List<?> list) {
      text.append('[');
      for (Object item : list) { if (text.length() >= 256) break; appendBrief(text, item); appendText(text, ","); }
      appendText(text, "]");
    } else if (value instanceof Map<?, ?> map) {
      text.append('{');
      for (var entry : map.entrySet()) { if (text.length() >= 256) break; appendBrief(text, entry.getKey()); appendText(text, "="); appendBrief(text, entry.getValue()); appendText(text, ","); }
      appendText(text, "}");
    } else if (value instanceof BytesValue bytes) {
      appendText(text, "hex:");
      for (byte item : bytes.copy()) { if (text.length() >= 256) break; appendText(text, "0123456789abcdef".substring((item & 255) >>> 4, ((item & 255) >>> 4) + 1)); appendText(text, "0123456789abcdef".substring(item & 15, (item & 15) + 1)); }
    } else appendText(text, value == null ? "absent" : value instanceof String || value instanceof Long || value instanceof Integer || value instanceof Boolean || value instanceof TtlState ? value.toString() : value.getClass().getSimpleName());
  }
  private static void appendText(StringBuilder text, String value) { text.append(value, 0, Math.min(value.length(), 256 - text.length())); }

  private record Token(long instance, long revision) {}
  private record Stored(BytesValue value, Long expiry, long revision) {}
  private record Input(BytesValue key, Optional<BytesValue> value, Optional<Token> token, TtlDirective ttl) {}
  private record Shape(Op operation, List<Input> entries) {}
  private record Retained(Shape shape, Result result) {}
  private static final class Refusal extends RuntimeException {
    Refusal(String kind) { super(kind); }
  }
  private static final class Model {
    private final TreeMap<BytesValue, Stored> data = new TreeMap<>();
    private final TreeMap<String, Optional<Token>> tokens = new TreeMap<>();
    private final Map<Request, Retained> outcomes = new HashMap<>();
    private final Map<Request, Shape> pending = new HashMap<>();
    private final Map<String, Long> retired = new HashMap<>();
    private final List<Event> events = new ArrayList<>();
    private long tick, revision, watermark, instance;
    private boolean destroyed, gap, held, heldPayload;

    Observation execute(Step step) {
      Result result;
      try { result = dispatch(step); }
      catch (Refusal failure) { result = Result.simple("error:" + failure.getMessage()); }
      var rows = data.entrySet().stream().map(entry -> new Row(entry.getKey(), entry.getValue().value(), ttl(entry.getValue()))).toList();
      var relations = new TreeMap<String, Boolean>();
      for (var left : tokens.entrySet()) for (var right : tokens.entrySet())
        relations.put(left.getKey() + "=" + right.getKey(), left.getValue().isPresent() && left.getValue().equals(right.getValue()));
      var observation = new Observation(result, rows, events, relations, new Owners(outcomes.size(), pending.size(), destroyed ? 0 : 1, heldPayload), tick);
      events.clear(); return observation;
    }
    private Result dispatch(Step step) {
      if (destroyed && step.operation() != Op.QUERY_OUTCOMES && step.operation() != Op.RESOURCES
          && step.operation() != Op.CLOSE_LISTENER_SNAPSHOT && step.operation() != Op.CANCEL_RESERVED) throw new Refusal("map_destroyed");
      return switch (step.operation()) {
      case QUERY_OUTCOMES -> {
        var statuses = step.requests().stream().map(identity -> {
          Retained completed = outcomes.get(identity);
          return new Status(identity, completed != null ? "completed" : pending.containsKey(identity) ? "pending" : "unknown", completed == null ? Optional.empty() : Optional.of(completed.result()));
        }).toList();
        yield new Result("outcomes", Optional.empty(), TtlState.absent(), List.of(), statuses, statuses.size());
      }
      case RESOURCES -> Result.simple("resources");
        case CAPTURE_VERSION -> {
          Entry entry = step.entries().get(0); String label = entry.versionRef().orElseThrow();
          Stored stored = data.get(entry.key());
          tokens.put(label, stored == null ? Optional.empty() : Optional.of(new Token(instance, stored.revision())));
          yield stored == null ? Result.simple("absent") : point("entry_view", Optional.of(stored.value()), stored);
        }
        case PUT, REPLACE_IF_VERSION, REMOVE_IF_VERSION, SET_TTL_IF_VERSION, BULK_REPLACE, BULK_REMOVE, BULK_SET_TTL, RESERVE_PUT, DISPATCH_PUT, EVICT, CLEAR, DESTROY -> mutate(step);
        case CANCEL_RESERVED -> Result.simple(pending.remove(step.requests().get(0)) == null ? "absent" : "cancelled");
        case RETIRE_OUTCOMES -> {
          Request through = step.requests().get(0);
          if (pending.keySet().stream().anyMatch(id -> id.client().equals(through.client()) && id.sequence() <= through.sequence())) throw new Refusal("pending_retirement");
          long previous = retired.getOrDefault(through.client(), 0L);
          if (through.sequence() < previous) throw new Refusal("invalid_argument");
          long before = outcomes.size(); outcomes.keySet().removeIf(id -> id.client().equals(through.client()) && id.sequence() <= through.sequence());
          retired.put(through.client(), through.sequence()); yield Result.numbered("retired", before - outcomes.size());
        }
        case LISTENER_RESYNC, HOLD_LISTENER_SNAPSHOT -> {
          if (held) throw new Refusal("active_owners");
          Limits limits = step.limits().orElseThrow();
          long bytes = data.entrySet().stream().mapToLong(entry -> 32L + entry.getKey().size() + entry.getValue().value().size()).sum();
          if (data.size() > limits.maximumItems() || bytes > limits.maximumBytes()) throw new Refusal("resource_budget_exhausted");
          var items = new ArrayList<Item>();
          data.forEach((key, stored) -> items.add(new Item(items.size(), key, "present", Optional.of(stored.value()), ttl(stored))));
          gap = false; events.clear();
          held = step.operation() == Op.HOLD_LISTENER_SNAPSHOT; heldPayload = held && !items.isEmpty();
          yield new Result("snapshot", Optional.empty(), TtlState.absent(), items, List.of(), watermark);
        }
        case CLOSE_LISTENER_SNAPSHOT -> { held = false; heldPayload = false; yield Result.simple("closed"); }
        case CHECKPOINT_RESTORE -> {
          if (held || !pending.isEmpty()) throw new Refusal("active_owners");
          instance++; gap = false; yield Result.simple("restored");
        }
        case ADVANCE -> {
          long target;
          try { target = Math.addExact(tick, step.ticks()); }
          catch (ArithmeticException overflow) { throw new Refusal("invalid_argument"); }
          tick = target;
          var expired = data.entrySet().stream().filter(entry -> entry.getValue().expiry() != null && entry.getValue().expiry() <= tick).map(Map.Entry::getKey).toList();
          for (BytesValue key : expired) { Stored old = data.remove(key); revision++; emit("expired", key, Optional.of(old.value()), Optional.empty()); }
          yield Result.simple("advanced");
        }
        case SIGNAL_GAP -> { watermark++; events.clear(); gap = true; events.add(new Event("gap", BytesValue.copyOf(new byte[0]), Optional.empty(), Optional.empty(), watermark)); yield Result.simple("gap"); }
      };
    }
    private Result mutate(Step step) {
      var keys = new HashSet<BytesValue>();
      for (Entry entry : step.entries()) {
        if (!keys.add(entry.key())) throw new Refusal("invalid_argument");
        if ((step.operation() == Op.SET_TTL_IF_VERSION || step.operation() == Op.BULK_SET_TTL) && entry.ttl().kind() == TtlDirective.Kind.PRESERVE) throw new Refusal("invalid_argument");
      }
      if (step.operation() != Op.DISPATCH_PUT) validateExpiry(step);
      Op operation = step.operation() == Op.RESERVE_PUT || step.operation() == Op.DISPATCH_PUT ? Op.PUT : step.operation();
      var inputs = step.entries().stream().map(entry -> new Input(entry.key(), entry.value(),
          entry.versionRef().map(label -> {
            Optional<Token> token = tokens.get(label);
            if (token == null || token.isEmpty()) throw new IllegalArgumentException("unknown/absent token reference");
            return token.orElseThrow();
          }), entry.ttl())).toList();
      Shape shape = new Shape(operation, inputs); Request identity = step.requests().get(0);
      if (identity.sequence() <= retired.getOrDefault(identity.client(), 0L)) throw new Refusal("retired_identity");
      Retained prior = outcomes.get(identity);
      if (prior != null) {
        if (!prior.shape().equals(shape)) throw new Refusal("mutation_identity_conflict");
        if (step.operation() == Op.DISPATCH_PUT) validateExpiry(step);
        return step.operation() == Op.RESERVE_PUT ? Result.simple("pending") : prior.result();
      }
      Shape reserved = pending.get(identity);
      if (step.operation() == Op.DISPATCH_PUT && reserved != null && tick == Long.MAX_VALUE) validateExpiry(step);
      if (reserved != null && !reserved.equals(shape)) throw new Refusal("mutation_identity_conflict");
      if (step.operation() == Op.DISPATCH_PUT && reserved == null) throw new Refusal("invalid_argument");
      if (step.operation() == Op.DISPATCH_PUT) validateExpiry(step);
      if (tick == Long.MAX_VALUE && step.operation() != Op.RESERVE_PUT) { pending.remove(identity); return Result.simple("not_applied"); }
      if (operation == Op.PUT && inputs.get(0).ttl().kind() == TtlDirective.Kind.PRESERVE && !data.containsKey(inputs.get(0).key())) throw new Refusal("invalid_argument");
      if (step.operation() == Op.RESERVE_PUT) { pending.put(identity, shape); return Result.simple("pending"); }
      Result result;
      if (operation == Op.CLEAR || operation == Op.DESTROY) {
        int count = data.size(); data.clear();
        if (operation == Op.DESTROY) { destroyed = true; outcomes.clear(); pending.clear(); retired.clear(); }
        if (count > 0 || destroyed) emit(destroyed ? "destroyed" : "cleared", BytesValue.copyOf(new byte[0]), Optional.empty(), Optional.empty());
        result = Result.numbered(destroyed ? "destroyed" : "cleared", count);
      } else if (operation == Op.BULK_REPLACE || operation == Op.BULK_REMOVE || operation == Op.BULK_SET_TTL) {
        var items = new ArrayList<Item>();
        for (Input input : inputs) {
          Stored old = data.get(input.key()); String kind = old == null ? "absent" : !matches(input, old) ? "mismatch" : operation == Op.BULK_REMOVE ? "removed" : "replaced";
          Stored current = old;
          if (kind.equals("removed")) { data.remove(input.key()); revision++; current = null; emit("removed", input.key(), Optional.of(old.value()), Optional.empty()); }
          if (kind.equals("replaced")) { current = store(input, old); emit("updated", input.key(), Optional.of(old.value()), Optional.of(current.value())); }
          items.add(new Item(items.size(), input.key(), kind, Optional.empty(), current == null ? TtlState.absent() : ttl(current)));
        }
        result = new Result("bulk", Optional.empty(), TtlState.absent(), items, List.of(), items.size());
      } else {
        Input input = inputs.get(0); Stored old = data.get(input.key());
        if (operation == Op.PUT) {
          Stored value = store(input, old); emit(old == null ? "added" : "updated", input.key(), old == null ? Optional.empty() : Optional.of(old.value()), Optional.of(value.value()));
          result = point(old == null ? "inserted" : "replaced", Optional.empty(), value);
        } else if (old == null) result = Result.simple("absent");
        else if (operation != Op.EVICT && !matches(input, old)) result = point("mismatch", Optional.empty(), old);
        else if (operation == Op.REMOVE_IF_VERSION || operation == Op.EVICT) {
          data.remove(input.key()); revision++; emit(operation == Op.EVICT ? "evicted" : "removed", input.key(), Optional.of(old.value()), Optional.empty());
          result = new Result(operation == Op.EVICT ? "evicted" : "removed", Optional.empty(), TtlState.absent(), List.of(), List.of(), 0);
        } else {
          Stored value = store(input, old); emit("updated", input.key(), Optional.of(old.value()), Optional.of(value.value()));
          result = point("replaced", operation == Op.REPLACE_IF_VERSION ? Optional.of(old.value()) : Optional.empty(), value);
        }
      }
      pending.remove(identity); if (!destroyed) outcomes.put(identity, new Retained(shape, result));
      return step.loseResponse() ? Result.simple("outcome_unknown") : result;
    }
    private boolean matches(Input input, Stored stored) { return input.token().equals(Optional.of(new Token(instance, stored.revision()))); }
    private void validateExpiry(Step step) {
      for (var entry : step.entries()) if (entry.ttl().kind() == TtlDirective.Kind.EXPIRE_AFTER) {
        try { Math.addExact(tick, entry.ttl().ticks()); }
        catch (ArithmeticException overflow) { throw new Refusal("invalid_argument"); }
      }
    }
    private Stored store(Input input, Stored old) {
      Long expiry = switch (input.ttl().kind()) { case ETERNAL -> null; case EXPIRE_AFTER -> Math.addExact(tick, input.ttl().ticks()); case PRESERVE -> old == null ? null : old.expiry(); };
      Stored value = new Stored(input.value().orElseGet(() -> old.value()), expiry, ++revision); data.put(input.key(), value); return value;
    }
    private TtlState ttl(Stored stored) { return stored.expiry() == null ? TtlState.eternal() : TtlState.expiring(stored.expiry() - tick); }
    private Result point(String kind, Optional<BytesValue> value, Stored stored) { return new Result(kind, value, ttl(stored), List.of(), List.of(), 0); }
    private void emit(String kind, BytesValue key, Optional<BytesValue> old, Optional<BytesValue> value) {
      watermark++; if (!gap) events.add(new Event(kind, key, old, value, watermark));
    }
  }
}
