# HydraCache Management Center

This is the read-only 0.57 Management Center bundle. It is intentionally separate
from the simulator `demo/` bundle: the console renders the real admin read
endpoints, while the demo remains a teaching lab.

The console expects to be served from the HydraCache admin origin at `/console/`.
It fetches `/cluster/overview` and `/metrics` from that same origin, so no CORS
policy is needed for the normal `kubectl port-forward` operator flow. It never
calls the authz-gated write endpoints.

Run locally:

```powershell
cd console
npm ci
npm test
```

Fidelity note: `source:"modeled"` is a first-class state, not an error. The UI
must keep that badge visible and must not show modeled leader data as live.
