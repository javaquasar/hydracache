package io.hydracache.consumer;

import io.hydracache.client.hc2.HydraCacheClient;
import io.hydracache.hazelcast.HydraCacheInstance;
import io.hydracache.hazelcast.HydraCodec;
import io.hydracache.hazelcast.HydraFencedLock;
import io.hydracache.hazelcast.HydraMap;
import java.time.Duration;

/** Compile-only external consumer of the preview facade surface. */
public final class FacadeConsumer {
  private FacadeConsumer() {}

  public static HydraMap<String, String> map(HydraCacheClient client) {
    return new HydraCacheInstance(client, Duration.ofSeconds(1))
        .getMap("consumer", HydraCodec.utf8(), HydraCodec.utf8());
  }

  public static HydraFencedLock<String> lock(HydraCacheClient client) {
    return new HydraCacheInstance(client, Duration.ofSeconds(1))
        .getFencedLock("consumer", "key", HydraCodec.utf8());
  }
}
