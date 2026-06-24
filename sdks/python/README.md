# HydraCache Python Client

This package is the non-JVM SDK selected for HydraCache 0.49 W3. It is tied to
client protocol version 1 and reads the same language-agnostic conformance
manifest as the Rust SDK.

The first release keeps the Python code small: protocol constants, stable error
retryability, B1 near-cache repair behavior, and a conformance runner entry point.
The nightly Docker tier runs the runner against a live HydraCache grid before the
SDK is claimed for external consumption.
