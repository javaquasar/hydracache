package io.hydracache.client.hc2;

import java.net.URI;
import java.util.Objects;

/** One advertised HC/2 node endpoint. */
public record NodeEndpoint(String nodeId, long nodeEpoch, URI endpoint, String serverName) {
  public NodeEndpoint {
    nodeId = requireText(nodeId, "nodeId");
    if (nodeEpoch < 0) throw new IllegalArgumentException("nodeEpoch must be nonnegative");
    endpoint = Objects.requireNonNull(endpoint, "endpoint");
    String scheme = endpoint.getScheme();
    if (!("https".equalsIgnoreCase(scheme) || "hc2+grpc".equalsIgnoreCase(scheme))
        || endpoint.getHost() == null) {
      throw new IllegalArgumentException("endpoint must be an absolute https or hc2+grpc URI");
    }
    serverName = requireText(serverName, "serverName");
  }

  private static String requireText(String value, String name) {
    Objects.requireNonNull(value, name);
    if (value.isBlank()) throw new IllegalArgumentException(name + " must not be blank");
    return value;
  }
}
