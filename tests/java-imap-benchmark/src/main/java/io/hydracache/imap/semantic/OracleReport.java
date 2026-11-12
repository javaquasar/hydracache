package io.hydracache.imap.semantic;

import java.util.List;

/** Immutable semantic admission report. */
public record OracleReport(
    long seed,
    AdapterIdentity leftIdentity,
    AdapterIdentity rightIdentity,
    List<SemanticDifference> differences,
    StateSnapshot leftSnapshot,
    StateSnapshot rightSnapshot) {
  public OracleReport {
    differences = List.copyOf(differences);
  }

  public boolean equivalent() { return differences.isEmpty(); }

  public void requireEquivalent() {
    if (!equivalent()) throw new SemanticMismatchException(render());
  }

  public String render() {
    if (equivalent()) {
      return "semantic-equivalent seed=" + seed + " digest=" + leftSnapshot.digest();
    }
    StringBuilder output = new StringBuilder("semantic-red seed=").append(seed);
    for (SemanticDifference difference : differences) {
      output.append(System.lineSeparator()).append(difference.path())
          .append(": left=").append(difference.left())
          .append(" right=").append(difference.right());
    }
    return output.toString();
  }
}
