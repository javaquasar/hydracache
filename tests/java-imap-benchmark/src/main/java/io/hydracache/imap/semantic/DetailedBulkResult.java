package io.hydracache.imap.semantic;

import java.util.HashSet;
import java.util.List;

/** Request-indexed bulk result that makes partial completion explicit and bounded. */
public record DetailedBulkResult(List<ItemOutcome> items, boolean complete, ErrorClass errorClass) {
  public DetailedBulkResult {
    items = List.copyOf(items);
    if (complete && errorClass != ErrorClass.NONE) {
      throw new IllegalArgumentException("complete bulk result cannot carry an error");
    }
    if (!complete && errorClass != ErrorClass.PARTIAL) {
      throw new IllegalArgumentException("incomplete bulk result must be classified PARTIAL");
    }
  }

  public void requireCoverage(int inputCount) {
    if (inputCount <= 0) throw new IllegalArgumentException("inputCount must be positive");
    var seen = new HashSet<Integer>();
    for (ItemOutcome item : items) {
      if (item.inputIndex() >= inputCount || !seen.add(item.inputIndex())) {
        throw new SemanticMismatchException("bulk result has duplicate or out-of-range index");
      }
    }
    if (complete && seen.size() != inputCount) {
      throw new SemanticMismatchException("complete bulk result omits an input index");
    }
  }
}
