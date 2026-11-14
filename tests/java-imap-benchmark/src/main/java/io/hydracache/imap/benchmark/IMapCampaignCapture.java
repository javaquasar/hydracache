package io.hydracache.imap.benchmark;

import io.hydracache.imap.benchmark.IMapCampaignPublication.Calibration;
import io.hydracache.imap.benchmark.IMapCampaignPublication.CalibrationSample;
import io.hydracache.imap.benchmark.IMapCampaignPublication.Capture;
import io.hydracache.imap.benchmark.IMapCampaignPublication.Hosts;
import io.hydracache.imap.benchmark.IMapCampaignPublication.TrialCapture;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Side;
import io.hydracache.imap.semantic.MapSemanticAdapter;
import java.util.EnumMap;
import java.util.Optional;
import java.util.concurrent.TimeoutException;
import java.util.function.LongSupplier;

/** Captures evidence at the actual measurement boundary on the coordinator clock. */
public final class IMapCampaignCapture {
  private IMapCampaignCapture() {}

  public interface Source {
    IMapLoadCampaign.Bindings observedBindings();
    Hosts hosts();
    String startupSha256(Side side, MapSemanticAdapter adapter);
    Optional<Calibration> calibrate(Side side);
  }

  static final class Collector {
    private final Source source;
    private final LongSupplier clock;
    private final EnumMap<Side, TrialCapture> trials = new EnumMap<>(Side.class);

    Collector(Source source, LongSupplier clock) {
      this.source = java.util.Objects.requireNonNull(source);
      this.clock = clock;
    }

    <T> T measure(Side side, MapSemanticAdapter adapter, IMapResourceMonitor.Action<T> measurement)
        throws InterruptedException, TimeoutException {
      String startup = source.startupSha256(side, adapter);
      var before = sample(side);
      long start = clock.getAsLong();
      T result = measurement.run();
      long end = clock.getAsLong();
      trials.put(side, new TrialCapture(startup, start, end, before, sample(side)));
      return result;
    }

    private Optional<CalibrationSample> sample(Side side) {
      return source.calibrate(side).map(value -> new CalibrationSample(clock.getAsLong(), value));
    }

    Capture complete() {
      return new Capture(source.observedBindings(), source.hosts(), trials.get(Side.LEFT), trials.get(Side.RIGHT));
    }
  }
}
