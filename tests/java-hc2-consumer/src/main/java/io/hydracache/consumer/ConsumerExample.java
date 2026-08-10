package io.hydracache.consumer;

import io.hydracache.client.hc2.HydraCacheClient;
import io.hydracache.client.hc2.HydraCacheClientConfig;
import java.net.URI;
import java.nio.file.Path;

/** Compilable installed-artifact example; credentials are supplied by the operator. */
public final class ConsumerExample {
  private ConsumerExample() {}

  public static HydraCacheClient connect(
      String endpoint, String serverName, Path ca, Path clientCertificate, Path clientKey) {
    HydraCacheClientConfig config = HydraCacheClientConfig.builder()
        .endpoint(URI.create(endpoint))
        .serverName(serverName)
        .trustCertificate(ca)
        .clientCertificate(clientCertificate)
        .clientPrivateKey(clientKey)
        .build();
    return HydraCacheClient.connect(config);
  }
}
