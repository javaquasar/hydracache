package io.hydracache.client.hc2;

import java.net.URI;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.EnumSet;
import java.util.Objects;
import java.util.Set;
import java.util.concurrent.Executor;
import java.util.concurrent.ForkJoinPool;

/** Immutable fail-closed connection configuration. */
public final class HydraCacheClientConfig {
  public static final int MINIMUM_PROTOCOL_GENERATION = 5;
  public static final int CURRENT_PROTOCOL_GENERATION = 6;

  private final URI endpoint;
  private final String serverName;
  private final Path trustCertificate;
  private final Path clientCertificate;
  private final Path clientPrivateKey;
  private final String clientId;
  private final String tenant;
  private final int protocolGeneration;
  private final Duration connectTimeout;
  private final Duration defaultRequestTimeout;
  private final Set<Capability> requestedCapabilities;
  private final Executor callbackExecutor;
  private final int maxInboundMessageBytes;
  private final int maxPendingInvocations;
  private final int maxSubscriptions;
  private final int maxSessions;
  private final int maxTopologyNodes;
  private final TransportPolicy transportPolicy;

  private HydraCacheClientConfig(Builder builder) {
    endpoint = requireEndpoint(builder.endpoint);
    serverName = requireText(builder.serverName, "serverName");
    trustCertificate = requireReadable(builder.trustCertificate, "trustCertificate");
    clientCertificate = requireReadable(builder.clientCertificate, "clientCertificate");
    clientPrivateKey = requireReadable(builder.clientPrivateKey, "clientPrivateKey");
    clientId = requireBoundedText(builder.clientId, "clientId", 128);
    tenant = requireBoundedText(builder.tenant, "tenant", 128);
    protocolGeneration = requireRange(builder.protocolGeneration, "protocolGeneration",
        MINIMUM_PROTOCOL_GENERATION, CURRENT_PROTOCOL_GENERATION);
    connectTimeout = requirePositive(builder.connectTimeout, "connectTimeout");
    defaultRequestTimeout = requirePositive(builder.defaultRequestTimeout, "defaultRequestTimeout");
    requestedCapabilities = Set.copyOf(builder.requestedCapabilities);
    if (requestedCapabilities.isEmpty()) {
      throw new IllegalArgumentException("at least one capability is required");
    }
    callbackExecutor = Objects.requireNonNull(builder.callbackExecutor, "callbackExecutor");
    maxInboundMessageBytes = requireRange(builder.maxInboundMessageBytes, "maxInboundMessageBytes", 1024, 16 * 1024 * 1024);
    maxPendingInvocations = requireRange(builder.maxPendingInvocations, "maxPendingInvocations", 1, 1_000_000);
    maxSubscriptions = requireRange(builder.maxSubscriptions, "maxSubscriptions", 1, 65_536);
    maxSessions = requireRange(builder.maxSessions, "maxSessions", 1, 65_536);
    maxTopologyNodes = requireRange(builder.maxTopologyNodes, "maxTopologyNodes", 1, 4096);
    transportPolicy = Objects.requireNonNull(builder.transportPolicy, "transportPolicy");
    if (transportPolicy != TransportPolicy.GRPC_MTLS_REQUIRED) {
      throw new IllegalArgumentException("unsupported transport policy");
    }
  }

  public static Builder builder() { return new Builder(); }
  public URI endpoint() { return endpoint; }
  public String serverName() { return serverName; }
  public Path trustCertificate() { return trustCertificate; }
  public Path clientCertificate() { return clientCertificate; }
  public Path clientPrivateKey() { return clientPrivateKey; }
  public String clientId() { return clientId; }
  public String tenant() { return tenant; }
  public int protocolGeneration() { return protocolGeneration; }
  public Duration connectTimeout() { return connectTimeout; }
  public Duration defaultRequestTimeout() { return defaultRequestTimeout; }
  public Set<Capability> requestedCapabilities() { return requestedCapabilities; }
  public Executor callbackExecutor() { return callbackExecutor; }
  public int maxInboundMessageBytes() { return maxInboundMessageBytes; }
  public int maxPendingInvocations() { return maxPendingInvocations; }
  public int maxSubscriptions() { return maxSubscriptions; }
  public int maxSessions() { return maxSessions; }
  public int maxTopologyNodes() { return maxTopologyNodes; }
  public TransportPolicy transportPolicy() { return transportPolicy; }

  private static URI requireEndpoint(URI value) {
    Objects.requireNonNull(value, "endpoint");
    if (!"https".equalsIgnoreCase(value.getScheme()) || value.getHost() == null || value.getPort() <= 0
        || value.getUserInfo() != null || value.getQuery() != null || value.getFragment() != null
        || !(value.getPath() == null || value.getPath().isEmpty() || "/".equals(value.getPath()))) {
      throw new IllegalArgumentException("endpoint must be https://host:port with no credentials, query, or path");
    }
    return value;
  }

  private static String requireText(String value, String name) {
    return requireBoundedText(value, name, 253);
  }

  private static String requireBoundedText(String value, String name, int max) {
    Objects.requireNonNull(value, name);
    if (value.isBlank() || value.length() > max || value.chars().anyMatch(Character::isISOControl)) {
      throw new IllegalArgumentException(name + " must be nonblank, bounded, and contain no controls");
    }
    return value;
  }

  private static Path requireReadable(Path value, String name) {
    Objects.requireNonNull(value, name);
    Path normalized = value.toAbsolutePath().normalize();
    if (!Files.isRegularFile(normalized) || !Files.isReadable(normalized)) {
      throw new IllegalArgumentException(name + " must be a readable regular file");
    }
    return normalized;
  }

  private static Duration requirePositive(Duration value, String name) {
    Objects.requireNonNull(value, name);
    if (value.isZero() || value.isNegative() || value.compareTo(Duration.ofMinutes(5)) > 0) {
      throw new IllegalArgumentException(name + " must be positive and at most five minutes");
    }
    return value;
  }

  private static int requireRange(int value, String name, int min, int max) {
    if (value < min || value > max) {
      throw new IllegalArgumentException(name + " must be in [" + min + ", " + max + "]");
    }
    return value;
  }

  /** Builder with no insecure or plaintext fallback. */
  public static final class Builder {
    private URI endpoint;
    private String serverName;
    private Path trustCertificate;
    private Path clientCertificate;
    private Path clientPrivateKey;
    private String clientId = "hydracache-java-preview";
    private String tenant = "default";
    private int protocolGeneration = CURRENT_PROTOCOL_GENERATION;
    private Duration connectTimeout = Duration.ofSeconds(10);
    private Duration defaultRequestTimeout = Duration.ofSeconds(5);
    private Set<Capability> requestedCapabilities = EnumSet.allOf(Capability.class);
    private Executor callbackExecutor = ForkJoinPool.commonPool();
    private int maxInboundMessageBytes = 4 * 1024 * 1024;
    private int maxPendingInvocations = 4096;
    private int maxSubscriptions = 1024;
    private int maxSessions = 1024;
    private int maxTopologyNodes = 256;
    private TransportPolicy transportPolicy = TransportPolicy.GRPC_MTLS_REQUIRED;

    private Builder() {}
    public Builder endpoint(URI value) { endpoint = value; return this; }
    public Builder serverName(String value) { serverName = value; return this; }
    public Builder trustCertificate(Path value) { trustCertificate = value; return this; }
    public Builder clientCertificate(Path value) { clientCertificate = value; return this; }
    public Builder clientPrivateKey(Path value) { clientPrivateKey = value; return this; }
    public Builder clientId(String value) { clientId = value; return this; }
    public Builder tenant(String value) { tenant = value; return this; }
    public Builder protocolGeneration(int value) { protocolGeneration = value; return this; }
    public Builder connectTimeout(Duration value) { connectTimeout = value; return this; }
    public Builder defaultRequestTimeout(Duration value) { defaultRequestTimeout = value; return this; }
    public Builder requestedCapabilities(Set<Capability> value) {
      Objects.requireNonNull(value, "requestedCapabilities");
      requestedCapabilities = value.isEmpty()
          ? EnumSet.noneOf(Capability.class)
          : EnumSet.copyOf(value);
      return this;
    }
    public Builder callbackExecutor(Executor value) { callbackExecutor = value; return this; }
    public Builder maxInboundMessageBytes(int value) { maxInboundMessageBytes = value; return this; }
    public Builder maxPendingInvocations(int value) { maxPendingInvocations = value; return this; }
    public Builder maxSubscriptions(int value) { maxSubscriptions = value; return this; }
    public Builder maxSessions(int value) { maxSessions = value; return this; }
    public Builder maxTopologyNodes(int value) { maxTopologyNodes = value; return this; }
    public Builder transportPolicy(TransportPolicy value) { transportPolicy = value; return this; }
    public HydraCacheClientConfig build() { return new HydraCacheClientConfig(this); }
  }
}
