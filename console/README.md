# HydraCache Management Center

This is the read-only 0.72 Management Center 2.0 bundle. It is intentionally separate
from the simulator `demo/` bundle: the console renders the real admin read
endpoints, while the demo remains a teaching lab.

The console expects to be served from the HydraCache admin origin at `/console/`.
It fetches typed, bounded `/management/v1` snapshots from that same origin, so no
CORS policy is needed for the normal `kubectl port-forward` operator flow. It
never scrapes arbitrary Prometheus text and never calls write endpoints.

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
```

The release gate is `cargo xtask verify`; it runs these specs when Node/npm are
available and logs an explicit skip when they are not installed.

Fidelity note: `live`, `modeled`, `partial`, `stale`, and `unavailable` are
first-class states. Missing CPU/RSS/retained-byte/TTL sources remain visibly
unavailable rather than becoming zero.
