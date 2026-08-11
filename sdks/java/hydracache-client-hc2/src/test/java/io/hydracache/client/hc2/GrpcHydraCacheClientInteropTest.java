package io.hydracache.client.hc2;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.List;
import java.util.Set;
import java.util.concurrent.CompletionException;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

final class GrpcHydraCacheClientInteropTest {
  private static final RequestOptions DEFAULT = RequestOptions.withTimeout(Duration.ofSeconds(2));

  @Test
  void javaSdkExecutesTheContractAgainstASeparateRustMtlsProcess(@TempDir Path temp) throws Exception {
    Path credentials = Files.createDirectory(temp.resolve("credentials"));
    try (InteropProcess server = InteropProcess.start(credentials)) {
      CountDownLatch eventArrived = new CountDownLatch(1);
      CacheEventListener listener = new CacheEventListener() {
        @Override public void onEvent(CacheEvent event) {
          if (new String(event.key(), StandardCharsets.UTF_8).startsWith("key")) eventArrived.countDown();
        }
        @Override public void onGap(long subscriptionId, long afterWatermark) {
          throw new AssertionError("unexpected gap");
        }
      };

      HydraCacheClient client = HydraCacheClient.connect(server.config("localhost"));
      assertEquals("hc2-java-interop", client.clusterId());
      Subscription subscription = client.subscribe(bytes("key"), 0, listener).join();
      assertTrue(client.put(bytes("key-1"), bytes("value-1"), Duration.ZERO, DEFAULT).join().applied());
      assertTrue(eventArrived.await(5, TimeUnit.SECONDS));
      assertArrayEquals(bytes("value-1"), client.get(bytes("key-1"), DEFAULT).join().orElseThrow().value());
      assertTrue(client.compareAndSet(bytes("key-1"), bytes("value-1"), bytes("value-2"),
          Duration.ZERO, DEFAULT).join().applied());
      assertFalse(client.removeIfValue(bytes("key-1"), bytes("stale"), DEFAULT).join().applied());
      LockAcquireResult lock = client.tryLock(bytes("lock-1"), Duration.ofSeconds(5), DEFAULT).join();
      assertTrue(lock.acquired());
      assertEquals(lock.fence(), client.lockOwnership(bytes("lock-1"), DEFAULT).join().fence());
      assertTrue(client.renewLock(bytes("lock-1"), lock.fence(), Duration.ofSeconds(5), DEFAULT)
          .join().applied());
      assertTrue(client.unlock(bytes("lock-1"), lock.fence(), DEFAULT).join().applied());

      List<BatchItemResult> batch = client.batch(List.of(
          new BatchOperation.Get(bytes("key-1")),
          new BatchOperation.Put(bytes("other"), bytes("batch"), Duration.ZERO)), DEFAULT).join();
      assertEquals(2, batch.size());
      assertArrayEquals(bytes("value-2"), batch.get(0).value().orElseThrow().value());
      assertTrue(batch.get(1).mutation().orElseThrow().applied());

      CompletionException timeout = assertThrows(CompletionException.class,
          () -> client.get(bytes("__hc2_delay__"), RequestOptions.withTimeout(Duration.ofMillis(50))).join());
      HydraCacheException timeoutCause = assertInstanceOf(HydraCacheException.class, timeout.getCause());
      assertEquals(ErrorCode.DEADLINE_EXCEEDED, timeoutCause.code());

      FencedSession session = client.openSession(Duration.ofSeconds(10)).join();
      assertTrue(session.fence() > 0);
      awaitTopology(client);
      assertEquals("interop-node", client.topology().nodes().get(0).nodeId());

      subscription.close();
      session.close();
      assertEquals(0, client.metrics().activeSubscriptions());
      assertEquals(0, client.metrics().activeSessions());
      assertEquals(0, client.metrics().pendingInvocations());
      assertTrue(client.metrics().completed() >= 4);
      assertTrue(client.metrics().cancelled() >= 1);
      client.close();

      String receipt = server.awaitClosed();
      assertTrue(receipt.contains("active_subscriptions=0"), receipt);
      assertTrue(receipt.contains("active_sessions=0"), receipt);
      assertFalse(hasLiveNonDaemonSdkThread(), "SDK retained a non-daemon lifecycle thread");
    }
  }

  @Test
  void javaSdkExecutesAgainstTheProductionDaemonAndDrainsCleanly(@TempDir Path temp)
      throws Exception {
    Path credentials = Files.createDirectory(temp.resolve("daemon-credentials"));
    try (DaemonProcess daemon = DaemonProcess.start(credentials)) {
      CountDownLatch eventArrived = new CountDownLatch(1);
      HydraCacheClient client = HydraCacheClient.connect(daemon.config());
      assertEquals("daemon-proof", client.clusterId());
      if (client.protocolGeneration() == HydraCacheClientConfig.MINIMUM_PROTOCOL_GENERATION) {
        int preferred = Integer.parseInt(System.getenv().getOrDefault(
            "HC2_JAVA_EXPECTED_PREFERRED_GENERATION",
            Integer.toString(HydraCacheClientConfig.MINIMUM_PROTOCOL_GENERATION)));
        boolean deprecated = Boolean.parseBoolean(System.getenv().getOrDefault(
            "HC2_JAVA_EXPECTED_DEPRECATED", "false"));
        assertEquals(preferred, client.preferredProtocolGeneration());
        assertEquals(deprecated, client.negotiatedGenerationDeprecated());
      }
      Subscription subscription = client.subscribe(bytes("daemon"), 0, new CacheEventListener() {
        @Override public void onEvent(CacheEvent event) { eventArrived.countDown(); }
        @Override public void onGap(long subscriptionId, long afterWatermark) {
          throw new AssertionError("unexpected production-daemon gap");
        }
      }).join();
      assertTrue(client.put(bytes("daemon-key"), bytes("daemon-value"), Duration.ZERO, DEFAULT)
          .join().applied());
      assertTrue(eventArrived.await(5, TimeUnit.SECONDS));
      assertArrayEquals(bytes("daemon-value"),
          client.get(bytes("daemon-key"), DEFAULT).join().orElseThrow().value());
      LockAcquireResult lock = client.tryLock(
          bytes("daemon-lock"), Duration.ofSeconds(3), DEFAULT).join();
      assertTrue(lock.acquired());
      assertTrue(client.unlock(bytes("daemon-lock"), lock.fence(), DEFAULT).join().applied());
      FencedSession session = client.openSession(Duration.ofSeconds(3)).join();
      assertTrue(session.fence() > 0);
      session.close();
      subscription.close();
      client.close();
      assertTrue(daemon.drain().contains("status=ok"));
      assertFalse(hasLiveNonDaemonSdkThread(), "SDK retained a non-daemon lifecycle thread");
    }
  }

  @Test
  void javaRecoveryReplacesADeadRustProcessRepairsSubscriptionAndLosesSession(@TempDir Path temp)
      throws Exception {
    Path firstCredentials = Files.createDirectory(temp.resolve("first"));
    Path secondCredentials = Files.createDirectory(temp.resolve("second"));
    InteropProcess first = InteropProcess.start(firstCredentials);
    try (InteropProcess second = InteropProcess.start(secondCredentials)) {
      AtomicInteger gaps = new AtomicInteger();
      CountDownLatch gapArrived = new CountDownLatch(1);
      RecoveringHydraCacheClient client = RecoveringHydraCacheClient.connect(
          List.of(
              new ReconnectEndpoint("first", first.config("localhost")),
              new ReconnectEndpoint("second", second.config("localhost"))),
          new ReconnectPolicy(3, Duration.ofMillis(10), Duration.ofMillis(10), Duration.ZERO, 11),
          InvocationRetryPolicy.defaults());
      Subscription subscription = client.subscribe(bytes("repair"), 0, new CacheEventListener() {
        @Override public void onEvent(CacheEvent event) { }
        @Override public void onGap(long subscriptionId, long afterWatermark) {
          gaps.incrementAndGet();
          gapArrived.countDown();
        }
      }).join();
      FencedSession session = client.openSession(Duration.ofSeconds(3)).join();

      first.close();
      client.reconnectNow().join();
      assertEquals("second", client.currentEndpoint());
      assertEquals("hc2-java-interop", client.clusterId());
      assertTrue(gapArrived.await(5, TimeUnit.SECONDS));
      assertEquals(1, gaps.get());
      HydraCacheException lost = assertThrows(HydraCacheException.class, session::fence);
      assertEquals(ErrorCode.SESSION_LOST, lost.code());
      subscription.repair(0).join();
      assertTrue(client.put(bytes("repair-key"), bytes("repair-value"), Duration.ZERO,
          new RequestOptions(Duration.ofSeconds(2), bytes("repair-put"))).join().applied());
      subscription.close();
      client.close();

      RecoveryMetricsSnapshot metrics = client.recoveryMetrics();
      assertEquals(2, metrics.reconnectAttempts());
      assertEquals(1, metrics.reconnectSuccesses());
      assertEquals(2, metrics.subscriptionRepairs());
      assertEquals(1, metrics.sessionLosses());
      String receipt = second.awaitClosed();
      assertTrue(receipt.contains("active_subscriptions=0"), receipt);
      assertTrue(receipt.contains("active_sessions=0"), receipt);
      assertFalse(hasLiveNonDaemonSdkThread(), "SDK retained a non-daemon lifecycle thread");
    } finally {
      first.close();
    }
  }

  @Test
  void javaSdkRotatesBetweenIndependentRustMtlsProcesses(@TempDir Path temp) throws Exception {
    Path firstCredentials = Files.createDirectory(temp.resolve("first"));
    Path secondCredentials = Files.createDirectory(temp.resolve("second"));
    InteropProcess first = InteropProcess.start(firstCredentials);
    try (InteropProcess second = InteropProcess.start(secondCredentials)) {
      HydraCacheClient firstClient = HydraCacheClient.connect(first.config("localhost"));
      assertEquals("hc2-java-interop", firstClient.clusterId());
      firstClient.close();
      first.awaitClosed();

      HydraCacheClient secondClient = HydraCacheClient.connect(second.config("localhost"));
      assertEquals("hc2-java-interop", secondClient.clusterId());
      secondClient.close();
      second.awaitClosed();
    } finally {
      first.close();
    }
  }

  @Test
  void wrongServerIdentityFailsBeforeTheRustServiceAcceptsAStream(@TempDir Path temp) throws Exception {
    Path credentials = Files.createDirectory(temp.resolve("credentials"));
    try (InteropProcess server = InteropProcess.start(credentials)) {
      HydraCacheException error = assertThrows(HydraCacheException.class,
          () -> HydraCacheClient.connect(server.config("wrong.example")));
      assertEquals(ErrorCode.UNAVAILABLE, error.code());
      assertEquals(RetryAdvice.NEVER, error.retryAdvice());
    }
  }

  @ParameterizedTest
  @ValueSource(strings = {"expired-server", "not-yet-server", "wrong-server-eku",
      "expired-client", "not-yet-client", "wrong-client-eku", "untrusted-client"})
  void invalidCertificateProfilesFailBeforeApplicationDispatch(String profile, @TempDir Path temp)
      throws Exception {
    Path credentials = Files.createDirectory(temp.resolve("credentials"));
    try (InteropProcess server = InteropProcess.start(credentials, profile)) {
      HydraCacheException error = assertThrows(HydraCacheException.class,
          () -> HydraCacheClient.connect(server.config("localhost")));
      assertEquals(ErrorCode.UNAVAILABLE, error.code());
      assertEquals(RetryAdvice.NEVER, error.retryAdvice());
    }
  }

  @Test
  void publicApiDoesNotExposeGeneratedWireTypes() {
    for (Class<?> api : Set.of(HydraCacheClient.class, HydraCacheClientConfig.class,
        CacheValue.class, BatchOperation.class, Subscription.class, FencedSession.class)) {
      for (var method : api.getMethods()) {
        assertFalse(method.toGenericString().contains("internal.wire"), method.toGenericString());
      }
    }
  }

  private static void awaitTopology(HydraCacheClient client) throws InterruptedException {
    long deadline = System.nanoTime() + Duration.ofSeconds(2).toNanos();
    while (client.topology().nodes().isEmpty() && System.nanoTime() < deadline) {
      Thread.sleep(10);
    }
    assertFalse(client.topology().nodes().isEmpty(), "topology was not delivered");
  }

  private static boolean hasLiveNonDaemonSdkThread() {
    return Thread.getAllStackTraces().keySet().stream()
        .anyMatch(thread -> thread.isAlive() && !thread.isDaemon()
            && thread.getName().startsWith("hydracache-hc2-"));
  }

  private static byte[] bytes(String value) { return value.getBytes(StandardCharsets.UTF_8); }
}
