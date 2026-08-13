package io.hydracache.hazelcast;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.hydracache.client.hc2.HydraCacheClient;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.Optional;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/** Proves facade semantics through the real Java HC/2 client and production daemon. */
final class BorrowedHazelcastLiveInteropTest {
  private static final Duration REQUEST_TIMEOUT = Duration.ofSeconds(2);

  @Test
  void facadeExecutesMapsListenersCasAndLocksAgainstTheProductionDaemon(@TempDir Path temp)
      throws Exception {
    Path credentials = Files.createDirectory(temp.resolve("credentials"));
    try (FacadeDaemonProcess daemon = FacadeDaemonProcess.start(credentials)) {
      HydraCacheClient firstClient = HydraCacheClient.connect(daemon.config("facade-first"));
      HydraCacheClient secondClient = HydraCacheClient.connect(
          daemon.config("facade-second", true));
      try (HydraCacheInstance first = new HydraCacheInstance(firstClient, REQUEST_TIMEOUT, true);
           HydraCacheInstance second = new HydraCacheInstance(secondClient, REQUEST_TIMEOUT, true)) {
        HydraMap<String, String> map = first.getMap(
            "borrowed-live", HydraCodec.utf8(), HydraCodec.utf8());
        CountDownLatch eventArrived = new CountDownLatch(1);
        try (var listener = map.addEntryListener(new EntryListener<>() {
          @Override public void onEvent(EntryEvent<String, String> event) {
            if (event.key().equals("key") && event.value().equals(Optional.of("value"))) {
              eventArrived.countDown();
            }
          }
          @Override public void onGap(long watermark) {
            throw new AssertionError("unexpected live-facade listener gap at " + watermark);
          }
        })) {
          map.put("key", "value");
          assertTrue(eventArrived.await(5, TimeUnit.SECONDS));
          assertEquals("value", map.get("key").orElseThrow());
          assertTrue(map.replace("key", "value", "updated"));
          assertFalse(map.replace("key", "stale", "wrong"));
          assertTrue(map.remove("key", "updated"));

          map.put("expiring", "ttl", Duration.ofMillis(25));
          long ttlDeadline = System.nanoTime() + Duration.ofSeconds(2).toNanos();
          while (map.get("expiring").isPresent() && System.nanoTime() < ttlDeadline) {
            Thread.sleep(10);
          }
          assertTrue(map.get("expiring").isEmpty(), "production daemon must expire map TTL");
        }

        HydraFencedLock<String> firstLock = first.getFencedLock(
            "borrowed-live", "lease", HydraCodec.utf8());
        long firstFence = firstLock.tryLock(Duration.ofMillis(5)).orElseThrow();
        assertEquals(firstFence, firstLock.getFence().orElseThrow());
        firstLock.renew(Duration.ofSeconds(1));
        HydraFencedLock<String> otherOwner = second.getFencedLock(
            "borrowed-live", "lease", HydraCodec.utf8());
        assertTrue(otherOwner.tryLock(Duration.ofSeconds(1)).isEmpty(),
            "a distinct verified mTLS identity must observe lock contention");
        firstLock.unlock();
        long successorFence = otherOwner.tryLock(Duration.ofSeconds(1)).orElseThrow();
        assertTrue(successorFence > firstFence, "successor fence must advance across owners");
        otherOwner.unlock();

        HydraFencedLock<String> sessionIndependent = first.getFencedLock(
            "borrowed-live", "session-independent", HydraCodec.utf8());
        sessionIndependent.tryLock(Duration.ofSeconds(1)).orElseThrow();
        var session = firstClient.openSession(Duration.ofSeconds(1)).join();
        session.close();
        assertTrue(sessionIndependent.isLocked(),
            "facade locks are lease/fence-bound, not HC/2 session-bound");
        sessionIndependent.unlock();
      }
      String receipt = daemon.drain();
      System.out.println("HC69_DAEMON_RECEIPT\t" + receipt);
      assertTrue(receipt.contains("status=ok"));
    }
  }
}
