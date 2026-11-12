package io.hydracache.imap.semantic;

import java.io.IOException;
import java.io.InputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Locale;

/** Separate-process entry point for the provisional v2 corpus and independent model. */
public final class ExtendedSemanticProcessRunner {
  private ExtendedSemanticProcessRunner() {}
  public static void main(String[] args) throws IOException { run(args, System.in, System.out); }
  static void run(String[] args, InputStream input, PrintStream output) throws IOException {
    if (args.length == 3 && args[0].equals("--v2-generate")) {
      byte[] bytes = ExtendedSemanticCorpus.write(ExtendedSemanticCorpus.generate(Long.parseLong(args[1]), Integer.parseInt(args[2]))).getBytes(StandardCharsets.UTF_8);
      output.write(bytes, 0, bytes.length);
      if (output.checkError()) throw new IllegalStateException("corpus output failed");
      return;
    }
    boolean expected = args.length == 3 && args[2].equals("--v2-expected");
    boolean faulty = args.length == 4 && args[2].equals("--v2-fault");
    if (args.length < 2 || !args[0].equals("--v2-script") || (args.length != 2 && !expected && !faulty)) throw new IllegalArgumentException("v2 script/generate arguments");
    var fault = UnifiedIMapExtendedSemanticAdapter.Fault.NONE;
    if (faulty) {
      fault = UnifiedIMapExtendedSemanticAdapter.Fault.valueOf(args[3].toUpperCase(Locale.ROOT));
      if (!fault.name().toLowerCase(Locale.ROOT).equals(args[3])) throw new IllegalArgumentException("noncanonical v2 fault");
    }
    String corpus;
    if (args[1].equals("-")) corpus = read(input);
    else try (var file = Files.newInputStream(Path.of(args[1]))) { corpus = read(file); }
    var scenario = ExtendedSemanticCorpus.parse(corpus);
    var frames = new Frames(output, expected);
    if (expected) new ExtendedSemanticOracle().forEachExpected(scenario, frames::observation);
    else try (var owner = UnifiedIMapExtendedSemanticAdapter.fresh("v2-process", fault)) {
      for (var step : scenario.steps()) frames.observation(step, owner.executeExtended(step));
    }
  }
  private static String read(InputStream input) throws IOException {
    byte[] bytes = input.readNBytes(ExtendedSemanticCorpus.MAX_BYTES + 1);
    if (bytes.length > ExtendedSemanticCorpus.MAX_BYTES) throw new IllegalArgumentException("script byte bound");
    for (byte value : bytes) if (value < 0) throw new IllegalArgumentException("script must be ASCII");
    return new String(bytes, StandardCharsets.US_ASCII);
  }
  static final class Frames {
    private final PrintStream output; private long bytes;
    Frames(PrintStream output, boolean expected) { this.output = output; line(ExtendedSemanticObservations.FORMAT + "\t" + (expected ? "expected" : "actual")); }
    void observation(ExtendedSemanticScenario.Step step, ExtendedSemanticScenario.Observation observation) {
      line(ExtendedSemanticCorpus.textHex(step.id()) + "\t" + ExtendedSemanticObservations.json(observation));
    }
    void line(String line) {
      byte[] encoded = line.getBytes(StandardCharsets.UTF_8);
      int size = encoded.length + 1;
      if (size > ExtendedSemanticObservations.MAX_LINE_BYTES + 194 || size > ExtendedSemanticObservations.MAX_OUTPUT_BYTES - bytes) throw new IllegalArgumentException("observation output bound");
      bytes += size; output.write(encoded, 0, encoded.length); output.write('\n');
      if (output.checkError()) throw new IllegalStateException("observation output failed");
    }
  }
}
