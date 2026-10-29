package io.hydracache.imap.hazelcast;

import com.hazelcast.client.HazelcastClient;
import com.hazelcast.client.config.ClientConfig;
import com.hazelcast.config.Config;
import com.hazelcast.config.InMemoryFormat;
import com.hazelcast.config.MapConfig;
import com.hazelcast.core.Hazelcast;
import com.hazelcast.core.HazelcastInstance;
import java.io.IOException;
import java.net.ServerSocket;
import java.util.UUID;

/** Owned single-member diagnostic fixture. Discovery, backups and TLS are explicitly disabled. */
public final class HazelcastLoopback075 implements AutoCloseable {
  private final HazelcastInstance member, client;
  private boolean closed;
  public HazelcastLoopback075() throws IOException {
    this(null);
  }
  HazelcastLoopback075(String clientClusterOverride) throws IOException {
    HazelcastArtifactPin.verifyLoadedArtifact();
    int port; try (var socket = new ServerSocket(0)) { port = socket.getLocalPort(); }
    var config = new Config().setClusterName("imap075-"+UUID.randomUUID());
    config.getNetworkConfig().setPort(port).setPortAutoIncrement(false);
    config.getNetworkConfig().getInterfaces().setEnabled(true).addInterface("127.0.0.1");
    config.getNetworkConfig().getJoin().getMulticastConfig().setEnabled(false);
    config.getNetworkConfig().getJoin().getAutoDetectionConfig().setEnabled(false);
    config.setProperty("hazelcast.phone.home.enabled","false").setProperty("hazelcast.logging.type","none")
        .setProperty("hazelcast.socket.bind.any","false")
        .setProperty("hazelcast.partition.count","17").setProperty("hazelcast.operation.thread.count","2")
        .setProperty("hazelcast.operation.generic.thread.count","2").setProperty("hazelcast.event.thread.count","1")
        .setProperty("hazelcast.io.input.thread.count","1").setProperty("hazelcast.io.output.thread.count","1");
    config.addMapConfig(new MapConfig("imap075-*").setBackupCount(0).setAsyncBackupCount(0).setInMemoryFormat(InMemoryFormat.BINARY));
    member = Hazelcast.newHazelcastInstance(config);
    try {
      var cc = new ClientConfig().setClusterName(clientClusterOverride==null?config.getClusterName():clientClusterOverride);
      cc.getNetworkConfig().addAddress("127.0.0.1:"+port);
      cc.setProperty("hazelcast.logging.type","none");
      cc.getConnectionStrategyConfig().getConnectionRetryConfig().setClusterConnectTimeoutMillis(3000);
      client = HazelcastClient.newHazelcastClient(cc);
    } catch (RuntimeException failure) { member.shutdown(); throw failure; }
  }
  public HazelcastInstance client() { return client; }
  public HazelcastInstance member() { return member; }
  @Override public void close() { if (!closed) { try { client.shutdown(); } finally { member.shutdown(); closed=true; } } }
}
