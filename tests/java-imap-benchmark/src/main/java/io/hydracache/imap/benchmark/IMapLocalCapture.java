package io.hydracache.imap.benchmark;

import static io.hydracache.imap.benchmark.IMapCampaignEvidenceDigest.code;
import static io.hydracache.imap.benchmark.IMapCampaignEvidenceDigest.file;
import static io.hydracache.imap.benchmark.IMapCampaignEvidenceDigest.text;

import io.hydracache.imap.benchmark.IMapCampaignPublication.Calibration;
import io.hydracache.imap.benchmark.IMapCampaignPublication.Hosts;
import io.hydracache.imap.benchmark.IMapLoadCampaign.Bindings;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Side;
import io.hydracache.imap.semantic.MapSemanticAdapter;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetAddress;
import java.nio.file.Path;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Optional;
import java.util.UUID;

/** Observations of the shared process and host used by the local campaign. */
final class IMapLocalCapture implements IMapCampaignCapture.Source {
  record Artifacts(Path left, Path right, Path harness, Path java, Path contract) {}

  @FunctionalInterface
  interface CalibrationProbe {
    Calibration sample() throws IOException;
  }

  private final Artifacts artifacts;
  private final String configuration;
  private final Hosts hosts;
  private final CalibrationProbe calibration;
  private final IdentityHashMap<MapSemanticAdapter, String> starts = new IdentityHashMap<>();

  IMapLocalCapture(Artifacts artifacts, String configuration, String host, CalibrationProbe calibration) {
    this.artifacts = artifacts;
    this.configuration = configuration;
    String fingerprint = text("imap-local-host-v1", host);
    this.hosts = new Hosts(List.of(fingerprint), fingerprint, false);
    this.calibration = calibration;
  }

  MapSemanticAdapter started(MapSemanticAdapter adapter) {
    starts.computeIfAbsent(adapter, instance -> text("imap-local-start-v1", UUID.randomUUID().toString()));
    return adapter;
  }

  @Override public String startupSha256(Side side, MapSemanticAdapter adapter) {
    return java.util.Objects.requireNonNull(starts.get(adapter), "unobserved adapter creation");
  }

  @Override public Hosts hosts() { return hosts; }

  String contractSha256() throws IOException { return file(artifacts.contract()); }

  @Override public Bindings observedBindings() {
    try {
      return new Bindings(code(artifacts.left()), code(artifacts.right()), file(artifacts.java()),
          code(artifacts.harness()), text("imap-local-config-v1", contractSha256() + "\n" + configuration),
          hosts.generator());
    } catch (IOException failure) {
      throw new UncheckedIOException("cannot fingerprint IMap campaign inputs", failure);
    }
  }

  @Override public Optional<Calibration> calibrate(Side side) {
    try {
      return Optional.of(calibration.sample());
    } catch (IOException unavailable) {
      return Optional.empty();
    }
  }

  static Calibration loopback() throws IOException {
    var address = InetAddress.getLoopbackAddress();
    try (var receiver = new DatagramSocket(0, address); var sender = new DatagramSocket(0, address)) {
      receiver.connect(sender.getLocalSocketAddress());
      receiver.setSoTimeout(50);
      byte[] outgoing = new byte[512];
      var packet = new DatagramPacket(outgoing, outgoing.length, receiver.getLocalSocketAddress());
      var reply = new DatagramPacket(new byte[outgoing.length], outgoing.length);
      int packets = 16;
      long start = System.nanoTime();
      for (int index = 0; index < packets; index++) {
        sender.send(packet);
        receiver.receive(reply);
      }
      long elapsed = Math.max(1, System.nanoTime() - start);
      return new Calibration(Math.max(1, packets * outgoing.length * 8_000_000_000L / elapsed),
          Math.max(1, packets * 1_000_000_000L / elapsed));
    }
  }
}
