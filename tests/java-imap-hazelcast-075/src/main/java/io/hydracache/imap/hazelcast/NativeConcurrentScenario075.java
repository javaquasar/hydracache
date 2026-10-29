package io.hydracache.imap.hazelcast;

import io.hydracache.imap.semantic.BytesValue;
import java.nio.ByteBuffer;
import java.util.*;

/** Seeded immutable finite program for single-call native operations, two workers, eternal values. */
public final class NativeConcurrentScenario075 {
  private NativeConcurrentScenario075() {}
  public enum Operation { GET, CONTAINS_KEY, PUT, PUT_IF_ABSENT, REPLACE_IF_PRESENT,
    GET_AND_PUT, GET_AND_REMOVE, REPLACE_IF_VALUE, REMOVE_IF_VALUE }
  public record Command(int id, int worker, int position, Operation operation, BytesValue key,
      Optional<BytesValue> value, Optional<BytesValue> expected) {
    public Command {
      Objects.requireNonNull(operation); Objects.requireNonNull(key); Objects.requireNonNull(value); Objects.requireNonNull(expected);
      if (id < 0 || id >= 16 || worker < 0 || worker > 1 || position < 0 || position >= 16 || key.size() > 64
          || value.filter(v -> v.size() > 64).isPresent() || expected.filter(v -> v.size() > 64).isPresent()) throw new IllegalArgumentException("native concurrent command bound");
      boolean writesValue = Set.of(Operation.PUT, Operation.PUT_IF_ABSENT, Operation.REPLACE_IF_PRESENT, Operation.GET_AND_PUT, Operation.REPLACE_IF_VALUE).contains(operation);
      boolean compares = operation == Operation.REPLACE_IF_VALUE || operation == Operation.REMOVE_IF_VALUE;
      if (writesValue != value.isPresent() || compares != expected.isPresent()) throw new IllegalArgumentException("native concurrent command shape");
    }
  }
  public record Program(long seed, List<BytesValue> keys, List<Command> commands) {
    public Program {
      if (keys.isEmpty() || keys.size() > 4 || commands.size() < 2 || commands.size() > 16) throw new IllegalArgumentException("native concurrent program bound");
      keys = List.copyOf(keys); commands = List.copyOf(commands);
      if (new HashSet<>(keys).size() != keys.size() || keys.stream().anyMatch(k -> k.size() > 64)) throw new IllegalArgumentException("native concurrent key allowlist");
      int[] positions = new int[2];
      for (int index = 0; index < commands.size(); index++) {
        var command = commands.get(index);
        if (command.id() != index || !keys.contains(command.key()) || command.position() != positions[command.worker()]++) throw new IllegalArgumentException("native concurrent program order");
      }
      if (positions[0] == 0 || positions[1] == 0) throw new IllegalArgumentException("two nonempty workers required");
    }
    Program without(int id) {
      var result = new ArrayList<Command>(); int[] positions = new int[2];
      for (var command : commands) if (command.id() != id) result.add(new Command(result.size(), command.worker(), positions[command.worker()]++, command.operation(), command.key(), command.value(), command.expected()));
      return new Program(seed, keys, result);
    }
  }
  public static Program seeded(long seed) {
    var random = new SplittableRandom(seed); var a = BytesValue.fromHex("00ff"); var b = BytesValue.fromHex("61");
    var values = new ArrayList<BytesValue>(); for (int i = 0; i < 6; i++) values.add(BytesValue.copyOf(ByteBuffer.allocate(8).putLong(random.nextLong()).array()));
    var commands = new ArrayList<Command>();
    add(commands, 0, 0, Operation.PUT_IF_ABSENT, a, values.get(0), null);
    add(commands, 1, 0, Operation.PUT_IF_ABSENT, a, values.get(1), null);
    add(commands, 0, 1, Operation.GET, a, null, null);
    add(commands, 1, 1, Operation.CONTAINS_KEY, a, null, null);
    add(commands, 0, 2, Operation.PUT, b, values.get(2), null);
    add(commands, 1, 2, Operation.GET_AND_PUT, a, values.get(3), null);
    add(commands, 0, 3, Operation.REPLACE_IF_PRESENT, a, values.get(4), null);
    add(commands, 1, 3, Operation.REPLACE_IF_VALUE, a, values.get(5), values.get(0));
    add(commands, 0, 4, Operation.REMOVE_IF_VALUE, b, null, values.get(2));
    add(commands, 1, 4, Operation.GET_AND_REMOVE, a, null, null);
    add(commands, 0, 5, Operation.GET_AND_PUT, b, values.get(3), null);
    add(commands, 1, 5, Operation.GET, b, null, null);
    return new Program(seed, List.of(a, b), commands);
  }
  private static void add(List<Command> commands, int worker, int position, Operation operation, BytesValue key, BytesValue value, BytesValue expected) {
    commands.add(new Command(commands.size(), worker, position, operation, key, Optional.ofNullable(value), Optional.ofNullable(expected)));
  }
}
