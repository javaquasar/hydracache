package io.hydracache.imap.semantic;

import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.Objects;

/** Runs one frozen trace against two fresh adapters and fails before any timed phase on mismatch. */
public final class SemanticOracle {
  public OracleReport compare(
      ScenarioManifest scenario, MapSemanticAdapter left, MapSemanticAdapter right) {
    Objects.requireNonNull(scenario, "scenario");
    Objects.requireNonNull(left, "left");
    Objects.requireNonNull(right, "right");
    var differences = new ArrayList<SemanticDifference>();

    var required = new LinkedHashSet<Operation>();
    for (ScenarioStep step : scenario.steps()) required.add(step.operation());
    for (Operation operation : required) {
      boolean leftSupports = left.capabilities().contains(operation);
      boolean rightSupports = right.capabilities().contains(operation);
      if (!leftSupports || !rightSupports) {
        differences.add(new SemanticDifference("capability." + operation.name().toLowerCase(),
            Boolean.toString(leftSupports), Boolean.toString(rightSupports)));
      }
    }

    if (differences.isEmpty()) {
      for (ScenarioStep step : scenario.steps()) {
        OperationOutcome leftOutcome = left.execute(step);
        OperationOutcome rightOutcome = right.execute(step);
        if (!leftOutcome.equals(rightOutcome)) {
          differences.add(new SemanticDifference(
              "step." + step.id() + ".outcome", leftOutcome.toString(), rightOutcome.toString()));
        }
        var leftEvents = left.drainEvents();
        var rightEvents = right.drainEvents();
        if (!leftEvents.equals(rightEvents)) {
          differences.add(new SemanticDifference(
              "step." + step.id() + ".events", leftEvents.toString(), rightEvents.toString()));
        }
      }
    }

    StateSnapshot leftSnapshot = left.snapshot();
    StateSnapshot rightSnapshot = right.snapshot();
    if (!leftSnapshot.equals(rightSnapshot)) {
      differences.add(new SemanticDifference(
          "final.snapshot", leftSnapshot.toString(), rightSnapshot.toString()));
    }
    return new OracleReport(scenario.seed(), left.identity(), right.identity(), differences,
        leftSnapshot, rightSnapshot);
  }
}
