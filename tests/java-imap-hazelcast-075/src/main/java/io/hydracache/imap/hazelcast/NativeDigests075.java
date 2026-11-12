package io.hydracache.imap.hazelcast;

import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;

/** One explicit failure boundary for the native fixture's required JDK digest. */
final class NativeDigests075 {
  private NativeDigests075() {}
  @FunctionalInterface interface Factory { MessageDigest create() throws NoSuchAlgorithmException; }
  static MessageDigest sha256() { return sha256(()->MessageDigest.getInstance("SHA-256")); }
  static MessageDigest sha256(Factory factory) {
    try { return factory.create(); }
    catch (NoSuchAlgorithmException unavailable) { throw new IllegalStateException(unavailable); }
  }
}
