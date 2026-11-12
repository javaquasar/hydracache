package io.hydracache.imap.semantic;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.function.ToIntFunction;

/** Stable request-indexed ownership ledger for retrying only unfinished bulk items. */
public final class BulkRetryLedger {
  public record PartitionBatch(int partition, long ownerGeneration, List<Integer> inputIndices) {
    public PartitionBatch {
      if (partition < 0 || ownerGeneration <= 0) {
        throw new IllegalArgumentException(
            "partition must be non-negative and owner generation must be positive");
      }
      inputIndices = List.copyOf(inputIndices);
      if (inputIndices.isEmpty()) throw new IllegalArgumentException("batch must not be empty");
    }
  }

  private final List<BytesValue> keys;
  private final ItemOutcome[] completed;
  private final long[] attemptedGeneration;

  public BulkRetryLedger(List<BytesValue> keys) {
    this.keys = List.copyOf(keys);
    if (this.keys.isEmpty()) throw new IllegalArgumentException("bulk input must not be empty");
    if (new HashSet<>(this.keys).size() != this.keys.size()) {
      throw new IllegalArgumentException("duplicate bulk keys are ambiguous");
    }
    completed = new ItemOutcome[this.keys.size()];
    attemptedGeneration = new long[this.keys.size()];
  }

  public List<PartitionBatch> routePending(
      long ownerGeneration, ToIntFunction<BytesValue> partitioner) {
    if (ownerGeneration <= 0) throw new IllegalArgumentException("owner generation must be positive");
    Objects.requireNonNull(partitioner, "partitioner");
    Map<Integer, List<Integer>> grouped = new LinkedHashMap<>();
    for (int index = 0; index < keys.size(); index++) {
      if (completed[index] != null) continue;
      if (ownerGeneration <= attemptedGeneration[index]) {
        throw new SemanticMismatchException("bulk retry did not advance owner generation");
      }
      int partition = partitioner.applyAsInt(keys.get(index));
      if (partition < 0) throw new IllegalArgumentException("partition must be non-negative");
      attemptedGeneration[index] = ownerGeneration;
      grouped.computeIfAbsent(partition, ignored -> new ArrayList<>()).add(index);
    }
    return grouped.entrySet().stream()
        .sorted(Map.Entry.comparingByKey())
        .map(entry -> new PartitionBatch(entry.getKey(), ownerGeneration, entry.getValue()))
        .toList();
  }

  public void complete(int inputIndex, long ownerGeneration, ItemOutcome outcome) {
    if (inputIndex < 0 || inputIndex >= keys.size()) {
      throw new IllegalArgumentException("input index out of range");
    }
    Objects.requireNonNull(outcome, "outcome");
    if (completed[inputIndex] != null) {
      throw new SemanticMismatchException("bulk input completed more than once");
    }
    if (outcome.inputIndex() != inputIndex || !outcome.key().equals(keys.get(inputIndex))) {
      throw new SemanticMismatchException("bulk completion does not match request position");
    }
    if (attemptedGeneration[inputIndex] == 0 || ownerGeneration != attemptedGeneration[inputIndex]) {
      throw new SemanticMismatchException("stale or unattempted bulk completion");
    }
    completed[inputIndex] = outcome;
  }

  public DetailedBulkResult result() {
    List<ItemOutcome> items = java.util.Arrays.stream(completed)
        .filter(Objects::nonNull)
        .sorted(Comparator.comparingInt(ItemOutcome::inputIndex))
        .toList();
    boolean complete = items.size() == keys.size();
    return new DetailedBulkResult(
        items, complete, complete ? ErrorClass.NONE : ErrorClass.PARTIAL);
  }
}
