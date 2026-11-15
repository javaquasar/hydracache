package io.hydracache.imap.benchmark;

import io.hydracache.imap.benchmark.IMapPairedAnalysis.CellReport;
import io.hydracache.imap.benchmark.IMapPairedAnalysis.Verdict;
import io.hydracache.imap.semantic.AdapterIdentity;
import io.hydracache.imap.semantic.MapEvent;
import io.hydracache.imap.semantic.MapSemanticAdapter;
import io.hydracache.imap.semantic.Operation;
import io.hydracache.imap.semantic.OperationOutcome;
import io.hydracache.imap.semantic.ScenarioStep;
import io.hydracache.imap.semantic.StateSnapshot;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.Arrays;
import java.util.List;
import java.util.Set;
import java.util.concurrent.CancellationException;
import java.util.function.BooleanSupplier;

/** Repeated A/A, a preregistered saturation ladder and a bounded slowdown control. */
final class IMapLocalDiagnostics {
  private IMapLocalDiagnostics() {}

  record Controls(List<Integer> rates, int delayMillis) {
    Controls {
      rates = List.copyOf(rates);
      if (rates.size() < 2 || rates.size() > 8) throw new IllegalArgumentException("use two to eight ladder rates");
      int previous = 0;
      for (int rate : rates) {
        if (rate <= previous) throw new IllegalArgumentException("ladder rates must be positive and strictly increasing");
        previous = rate;
      }
      if (delayMillis < 1 || delayMillis > 250) throw new IllegalArgumentException("delay must be 1 to 250 ms");
    }

    static Controls parse(String rates, String delayMillis) {
      return new Controls(Arrays.stream(rates.split(",", -1)).map(Integer::parseInt).toList(),
          Integer.parseInt(delayMillis));
    }
  }

  static void run(Path output, IMapLocalCampaign.Options options, Controls controls, IMapLocalCapture capture,
      IMapResourceMonitor.Targets targets, BooleanSupplier cancelled) throws IOException {
    var ladder = IMapLocalCampaign.plan(options, controls.rates(), capture);
    var fault = IMapLocalCampaign.plan(options, List.of(options.settings().requestsPerSecond()), capture);
    Files.createDirectory(output);
    var repeated = IMapLocalCampaign.execute(output.resolve("aa"), options, ladder, capture, targets,
        cancelled, IMapLocalCampaign::reference);
    var slowed = IMapLocalCampaign.execute(output.resolve("slowdown"), options, fault, capture, targets,
        cancelled, () -> new SlowAdapter(IMapLocalCampaign.reference(), controls.delayMillis()));

    var report = new StringBuilder("Scope: local diagnostic\n")
        .append("A/A ladder ").append(IMapLoadControls.saturationFromReports(ladder, repeated)).append('\n');
    for (var cell : repeated) {
      report.append("A/A ").append(cell.cell()).append(' ').append(repeatability(cell.verdict())).append('\n');
      tail(report, cell);
    }
    var faultCell = slowed.get(0);
    report.append("Slowdown ").append(controls.delayMillis()).append(" ms ")
        .append(faultCell.cell()).append(' ').append(faultCell.verdict()).append('\n')
        .append("Slowdown limits ").append(IMapLoadControls.saturationFromReports(fault, slowed)).append('\n');
    tail(report, faultCell);
    Files.writeString(output.resolve("controls.txt"), report, StandardOpenOption.CREATE_NEW, StandardOpenOption.WRITE);
  }

  static Verdict repeatability(Verdict observed) {
    return observed == Verdict.EQUIVALENT ? Verdict.EQUIVALENT : Verdict.INCONCLUSIVE;
  }

  private static void tail(StringBuilder report, CellReport cell) {
    cell.statistics().ifPresent(statistics -> report.append("p99 ").append(statistics.p99())
        .append("\np999 ").append(statistics.p999()).append('\n'));
  }

  record SlowAdapter(MapSemanticAdapter delegate, int delayMillis) implements MapSemanticAdapter {
    @Override public AdapterIdentity identity() { return delegate.identity(); }
    @Override public Set<Operation> capabilities() { return delegate.capabilities(); }
    @Override public List<MapEvent> drainEvents() { return delegate.drainEvents(); }
    @Override public StateSnapshot snapshot() { return delegate.snapshot(); }
    @Override public void close() { delegate.close(); }

    @Override public OperationOutcome execute(ScenarioStep step) {
      try {
        Thread.sleep(delayMillis);
      } catch (InterruptedException interrupted) {
        Thread.currentThread().interrupt();
        throw new CancellationException("local slowdown interrupted");
      }
      return delegate.execute(step);
    }
  }
}
