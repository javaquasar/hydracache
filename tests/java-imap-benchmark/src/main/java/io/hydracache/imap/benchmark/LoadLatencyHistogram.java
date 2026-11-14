package io.hydracache.imap.benchmark;

import java.util.ArrayList;

/** Fixed memory histogram. Buckets report upper bounds with at least seven significant bits. */
final class LoadLatencyHistogram {
  private static final int SUB_BUCKETS = 128;
  private final int[] counts = new int[57 * SUB_BUCKETS];
  private int samples;

  void record(long nanos) {
    int shift = Math.max(0, 63 - Long.numberOfLeadingZeros(nanos) - 7);
    counts[shift * SUB_BUCKETS + (int) (nanos >>> shift)]++;
    samples++;
  }

  FixedRateIMapLoad.Histogram snapshot() {
    var buckets = new ArrayList<FixedRateIMapLoad.Bucket>();
    for (int index = 0; index < counts.length; index++) {
      if (counts[index] > 0) {
        int shift = Math.max(0, index / SUB_BUCKETS - 1);
        long upper = ((long) (index - shift * SUB_BUCKETS) << shift) | ((1L << shift) - 1);
        buckets.add(new FixedRateIMapLoad.Bucket(upper, counts[index]));
      }
    }
    return new FixedRateIMapLoad.Histogram(samples, buckets);
  }
}
