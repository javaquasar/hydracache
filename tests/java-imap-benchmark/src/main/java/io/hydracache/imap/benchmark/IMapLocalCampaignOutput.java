package io.hydracache.imap.benchmark;

import static java.nio.file.StandardOpenOption.CREATE_NEW;
import static java.nio.file.StandardOpenOption.WRITE;

import io.hydracache.imap.benchmark.FixedRateIMapLoad.Histogram;
import io.hydracache.imap.benchmark.FixedRateIMapLoad.Phase;
import io.hydracache.imap.benchmark.FixedRateIMapLoad.Trial;
import io.hydracache.imap.benchmark.IMapCampaignPublication.PairEvidence;
import io.hydracache.imap.benchmark.IMapCampaignPublication.Registration;
import java.io.BufferedWriter;
import java.io.DataOutputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

final class IMapLocalCampaignOutput {
  private IMapLocalCampaignOutput() {}

  static List<IMapPairedAnalysis.CellReport> write(Path output, IMapLocalCampaign.Options options, Registration registration,
      List<PairEvidence> pairs) throws IOException {
    var manifest = IMapCampaignPublication.seal(registration, pairs);
    var assessment = IMapCampaignPublication.assess(registration, manifest.sha256(), manifest, pairs);
    var index = new StringBuilder("format=imap-local-campaign-v1\nscope=local-diagnostic\n")
        .append("manifest.version=").append(manifest.version()).append('\n')
        .append("manifest.bin.sha256=").append(manifest.sha256()).append('\n')
        .append("registration.bin.sha256=").append(manifest.registrationSha256()).append('\n')
        .append("pairs.expected=").append(registration.campaign().runs().size()).append('\n')
        .append("pairs.retained=").append(pairs.size()).append('\n');
    try (var histograms = Files.newBufferedWriter(output.resolve("histograms.tsv"), CREATE_NEW, WRITE)) {
      histograms.write("pair\tside\tphase\tseries\tupper_nanos\tcount\n");
      for (int indexInPlan = 0; indexInPlan < pairs.size(); indexInPlan++) {
        int number = indexInPlan + 1;
        var pair = pairs.get(indexInPlan);
        String name = "pair-" + number + ".bin";
        try (var binary = binary(output.resolve(name))) {
          IMapCampaignEvidenceDigest.writePair(binary, pair);
        }
        index.append(name).append(".sha256=").append(manifest.pairSha256().get(indexInPlan)).append('\n');
        if (pair.attempt() instanceof IMapLoadCampaign.Measured measured) {
          writeTrial(histograms, number, "left", measured.result().left());
          writeTrial(histograms, number, "right", measured.result().right());
        }
      }
    }
    try (var binary = binary(output.resolve("registration.bin"))) {
      IMapCampaignEvidenceDigest.writeRegistration(binary, registration);
    }
    try (var binary = binary(output.resolve("manifest.bin"))) {
      IMapCampaignEvidenceDigest.writeManifest(binary, manifest);
    }
    var report = new StringBuilder("Scope: local diagnostic\n").append(options).append('\n');
    for (var blocker : assessment.blockers()) {
      report.append("Publication blocked: ").append(blocker.reason()).append(' ').append(blocker.subject())
          .append(' ').append(blocker.detail()).append('\n');
    }
    var attempts = pairs.stream().map(PairEvidence::attempt).toList();
    var cells = IMapPairedAnalysis.analyze(registration.campaign(), attempts);
    for (var cell : cells) {
      report.append(cell.cell()).append(' ').append(cell.verdict()).append(' ').append(cell.reason()).append('\n');
      cell.statistics().ifPresent(statistics -> report.append(statistics).append('\n'));
      for (var resource : cell.resources()) report.append(resource).append('\n');
    }
    Files.writeString(output.resolve("report.txt"), report, CREATE_NEW, WRITE);
    Files.writeString(output.resolve("manifest.properties"), index, CREATE_NEW, WRITE);
    return cells;
  }

  private static DataOutputStream binary(Path path) throws IOException {
    return new DataOutputStream(new java.io.BufferedOutputStream(Files.newOutputStream(path, CREATE_NEW, WRITE)));
  }

  private static void writeTrial(BufferedWriter out, int pair, String side, Trial trial) throws IOException {
    writePhase(out, pair + "\t" + side + "\twarmup\t", trial.warmup());
    writePhase(out, pair + "\t" + side + "\tmeasurement\t", trial.measured());
  }

  private static void writePhase(BufferedWriter out, String prefix, Phase phase) throws IOException {
    var histograms = phase.histograms().orElseThrow();
    writeHistogram(out, prefix + "end-to-end\t", histograms.endToEnd());
    writeHistogram(out, prefix + "queue\t", histograms.queue());
    writeHistogram(out, prefix + "service\t", histograms.service());
  }

  private static void writeHistogram(BufferedWriter out, String prefix, Histogram histogram) throws IOException {
    for (var bucket : histogram.buckets()) {
      out.write(prefix + bucket.upperNanos() + "\t" + bucket.count() + "\n");
    }
  }
}
