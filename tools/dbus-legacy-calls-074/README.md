# Whole legacy D-Bus helper compatibility

This private test-only projection includes all eight selected legacy helper bodies
without edits: start, inspect, optional inspect, stop/wait, reset/wait, loaded inspect,
property conversion and namespace validation. The whole original source is pinned.
The four original Connection::system acquisition sites resolve to a private fixture
factory; each successful acquisition receives a fresh in-process p2p connection.
Calls and lazy property-cache discovery are checked against explicit peer scripts.
No original-owner, live manager-channel or kernel-context checks are stubbed.
Those facilities are absent from this legacy helper gate and remain unqualified.

All 67 earlier identities are retained beside eleven new functions. They cover
property shapes/order, exact start/stop/reset modes, bad reply, error, disconnect,
withheld acknowledgement without retry, namespace/policy refusal before connection,
fresh inspection acquisition, lazy property reads, failed-unit reset preconditions,
retirement observation and the unchanged original one-second supported stop timeout.
This is behavior compatibility, not a production execution or timing claim.
The fake peer is upstream authenticated_socket Channel, which bypasses OS auth.
No real bus, systemd service, clock, host or product workload is addressed.

The fixed prepare/session commands are `python -B tools/dbus-legacy-calls-074/run.py
prepare` and `python -B tools/dbus-legacy-calls-074/run.py session-io-safe`. Outputs
are exclusive; preserve first refusals and never delete receipts to retry.
The independently pinned existing CI owner retains offline/locked builds, complete
actual catalogs, disjoint GNU 1.94/1.88 targets, original 1200/1100 absolute origins,
4 GiB floors and five-second capacity/retirement. No package/780, workspace/4800,
original live deadline, root dependency or frozen qualification identity changes.
