package io.hydracache.imap.semantic;

import java.util.HashSet;
import java.util.List;

/** A bounded, immutable semantic trace. */
public record ScenarioManifest(long seed, List<ScenarioStep> steps) {
  public ScenarioManifest {
    steps = List.copyOf(steps);
    var ids = new HashSet<String>();
    for (ScenarioStep step : steps) {
      if (!ids.add(step.id())) throw new IllegalArgumentException("duplicate step id: " + step.id());
    }
  }
}
