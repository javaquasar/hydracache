# HydraCache Management Center

This is the read-only 0.72 Management Center 2.0 bundle. It is intentionally separate
from the simulator `demo/` bundle: the console renders the real admin read
endpoints, while the demo remains a teaching lab.

The console expects to be served from the HydraCache admin origin at `/console/`.
It fetches typed, bounded `/management/v1` snapshots from that same origin, so no
CORS policy is needed for the normal `kubectl port-forward` operator flow. It
never scrapes arbitrary Prometheus text and never calls write endpoints.

Every API read carries the dedicated `management.read` capability. That capability cannot call
write-admin routes; write-admin identities imply read only for operational compatibility. The
management API is intended for the internal/loopback admin listener. For remote access, terminate
TLS (prefer mTLS) at a trusted reverse proxy, strip all inbound `x-hydracache-*` identity and
capability headers, and set verified replacements. Do not expose the raw admin listener publicly.

Static responses use a self-only Content Security Policy; JSON is `nosniff` and non-cacheable.
The bundle has no CDN or runtime third-party dependency and constructs diagnostic values as text,
never raw HTML. Management reads have an independent fail-fast concurrency budget of 16, returning
429 without consuming write-admin admission capacity.

Charts retain history only in the current tab. The ring is frozen at 24 series,
360 points per series, 4,320 total points, and 256 KiB encoded state. Oldest
samples are evicted first, authority-epoch changes clear incompatible history,
counter resets produce gaps, and gauges are never differentiated. Polling pauses
while the page is hidden or the browser is offline and aborts obsolete requests.

Run locally:

```powershell
cd console
npm ci
npm test
npm run supply-chain
npm audit --audit-level=high
```

The Playwright gate covers desktop, tablet/200%-zoom and narrow mobile truth parity, axe-core,
keyboard focus, forced colors, reduced motion, hostile diagnostic strings and GET-only network
behavior. `npm run supply-chain` validates exact pins, registry provenance, integrity and reviewed
licenses, then emits `target/management-center-0.72-sbom.cdx.json`.

The release gate is `cargo xtask verify`; it runs these specs when Node/npm are
available and logs an explicit skip when they are not installed.

Fidelity note: `live`, `modeled`, `partial`, `stale`, and `unavailable` are
first-class states. Missing CPU/RSS/retained-byte/TTL sources remain visibly
unavailable rather than becoming zero.
